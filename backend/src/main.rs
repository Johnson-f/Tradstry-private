mod middleware;
mod models;
mod routes;
mod service;
mod turso;
mod websocket;

use actix_cors::Cors;
use actix_web::{
    App, HttpMessage, HttpServer, Result as ActixResult,
    dev::ServiceRequest,
    middleware::Logger,
    web::{self, Data, Json},
};
use actix_web_httpauth::{
    extractors::{
        AuthenticationError,
        bearer::{BearerAuth, Config},
    },
    middleware::HttpAuthentication,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::service::brokerage::sync::start_brokerage_sync_scheduler;
use crate::service::market_engine::MarketStreamService;
use crate::service::notebook_engine::collaboration::RoomManager;
use crate::service::notifications::{
    earnings_calendar::start_earnings_calendar_scheduler,
    system::start_discipline_notification_scheduler,
};
use routes::{
    configure_ai_analysis_routes, configure_ai_chat_routes, configure_ai_reports_routes,
    configure_alert_routes, configure_analytics_routes, configure_brokerage_routes,
    configure_calendar_routes, configure_collaboration_routes, configure_notebook_ai_routes,
    configure_notebook_folders_routes, configure_notebook_images_routes,
    configure_notebook_notes_routes, configure_notebook_tags_routes,
    configure_options_routes, configure_playbook_routes, configure_stocks_routes,
    configure_trade_notes_routes, configure_trade_tags_routes, configure_user_routes,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use turso::{AppState, SupabaseClaims, get_supabase_user_id, validate_supabase_jwt_token};
use websocket::{ConnectionManager, collab_ws_handler, ws_handler};

#[derive(Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    message: Option<String>,
}

impl<T> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
        }
    }

    #[allow(dead_code)]
    fn error(message: &str) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            message: Some(message.to_string()),
        }
    }
}

#[derive(Serialize)]
struct HealthCheck {
    status: String,
    database: String,
    timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct UserQuery {
    limit: Option<i32>,
    offset: Option<i32>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logging with module-level filter to quiet noisy upstream logs
    let mut log_builder =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));
    log_builder.filter_module(
        "finance_query_core::client::yahoo_client",
        log::LevelFilter::Warn,
    );
    log_builder.init();

    // Load environment variables
    dotenvy::dotenv().ok();

    // Initialize application state
    let app_state = AppState::new()
        .await
        .expect("Failed to initialize app state");
    let app_state_arc = Arc::new(app_state.clone());
    let app_data = Data::new(app_state);

    // Brokerage sync scheduler (SnapTrade) - always enabled
    let interval_secs = std::env::var("BROKERAGE_SYNC_INTERVAL_SECS")
        .unwrap_or_else(|_| "900".to_string())
        .parse::<u64>()
        .unwrap_or(900);

    let max_concurrency = std::env::var("BROKERAGE_SYNC_MAX_CONCURRENCY")
        .unwrap_or_else(|_| "4".to_string())
        .parse::<usize>()
        .unwrap_or(4);

    start_brokerage_sync_scheduler(Arc::clone(&app_state_arc), interval_secs, max_concurrency);

    // System discipline notifications (registry-wide; ET 9:30–17:00 every 2h)
    start_discipline_notification_scheduler(Arc::clone(&app_state_arc));
    start_earnings_calendar_scheduler(Arc::clone(&app_state_arc));

    // Start background schema sync for all user databases (non-blocking)
    // Only enabled in production via ENABLE_STARTUP_SCHEMA_SYNC=true
    if std::env::var("ENABLE_STARTUP_SCHEMA_SYNC")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(false)
    {
        log::info!("Background schema sync enabled - starting sync for all users");
        turso::sync::sync_all_user_schemas_background(Arc::clone(&app_data.turso_client));
    } else {
        log::info!(
            "Background schema sync disabled (set ENABLE_STARTUP_SCHEMA_SYNC=true to enable)"
        );
    }

    // Initialize WebSocket connection manager
    let ws_manager = Arc::new(Mutex::new(ConnectionManager::new()));
    let ws_manager_data = Data::new(Arc::clone(&ws_manager));

    // Initialize Collaboration service for real-time note editing
    let collab_service = Arc::new(service::notebook_engine::CollaborationService::new());
    let collab_service_data = Data::new(Arc::clone(&collab_service));

    // Initialize Y-CRDT RoomManager for real-time collaborative editing
    let room_manager = Arc::new(Mutex::new(RoomManager::with_defaults()));
    let room_manager_data = Data::new(Arc::clone(&room_manager));
    
    // Start periodic persistence for collaboration rooms
    {
        let manager = room_manager.lock().await;
        manager.start_periodic_persistence(Arc::clone(&app_state_arc));
    }

    // Initialize Market Stream Service for real-time quotes, indices, and movers
    let poll_interval = std::env::var("MARKET_POLL_INTERVAL")
        .unwrap_or_else(|_| "5".to_string())
        .parse::<u64>()
        .unwrap_or(5);

    let rule_resync_interval = std::env::var("ALERT_RULE_RESYNC_INTERVAL_SECS")
        .unwrap_or_else(|_| "60".to_string())
        .parse::<u64>()
        .unwrap_or(60);

    let market_stream = Arc::new(MarketStreamService::new(
        Arc::clone(&ws_manager),
        poll_interval,
        Arc::clone(&app_state_arc),
        rule_resync_interval,
    ));

    // Start the market stream service background task
    let stream_for_start = Arc::clone(&market_stream);
    tokio::spawn(async move {
        if let Err(e) = stream_for_start.start().await {
            log::error!("Failed to start market stream service: {}", e);
        }
    });

    let market_stream_data = Data::new(market_stream);

    // Get port from environment or default
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "9000".to_string())
        .parse::<u16>()
        .expect("Invalid PORT value");

    log::info!("Server starting on http://127.0.0.1:{}", port);
    log::info!("Registering routes...");

    // Start HTTP server
    let _ws_manager_clone = Arc::clone(&ws_manager);

    HttpServer::new(move || {
        log::info!("Creating new App instance");

        // CORS configuration - supports multiple origins for separated deployment
        let allowed_origins = std::env::var("ALLOWED_ORIGINS")
            .unwrap_or_else(|_| "https://your-domain.com,http://localhost:3000".to_string());

        // Parse multiple origins from environment variable
        let origins: Vec<&str> = allowed_origins.split(',').map(|s| s.trim()).collect();

        let cors = if std::env::var("RUST_ENV").unwrap_or_default() == "production" {
            // Production: strict CORS with multiple allowed origins
            let mut cors = Cors::default();

            // Add each allowed origin
            for origin in &origins {
                cors = cors.allowed_origin(origin);
            }

            cors.allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS", "PATCH"])
                .allowed_headers(vec![
                    actix_web::http::header::AUTHORIZATION,
                    actix_web::http::header::CONTENT_TYPE,
                    actix_web::http::header::HeaderName::from_static("x-requested-with"),
                ])
                .supports_credentials()
                .max_age(3600)
        } else {
            // Development: allow localhost and production domains
            Cors::default()
                .allowed_origin("http://localhost:3000")
                .allowed_origin("https://your-domain.com")
                .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS", "PATCH"])
                .allowed_headers(vec![
                    actix_web::http::header::AUTHORIZATION,
                    actix_web::http::header::CONTENT_TYPE,
                    actix_web::http::header::HeaderName::from_static("x-requested-with"),
                ])
                .supports_credentials()
                .max_age(3600)
        };

        App::new()
            .app_data(ws_manager_data.clone())
            .app_data(app_data.clone())
            .app_data(market_stream_data.clone())
            .app_data(room_manager_data.clone())
            // CRITICAL: Add TursoClient as separate app_data for user routes
            .app_data(Data::new(app_data.as_ref().turso_client.clone()))
            // CRITICAL: Add SupabaseConfig as separate app_data for user routes
            .app_data(Data::new(app_data.as_ref().config.supabase.clone()))
            // CRITICAL: Add CacheService as separate app_data for user routes
            .app_data(Data::new(app_data.as_ref().cache_service.clone()))
            // CRITICAL: Add AIReportsService as separate app_data for AI reports routes
            .app_data(Data::new(app_data.as_ref().ai_reports_service.clone()))
            // CRITICAL: Add TradeAnalysisService as separate app_data for AI analysis routes
            .app_data(Data::new(app_data.as_ref().analysis_service.clone()))
            // VectorizationService removed - no longer using Upstash
            // CRITICAL: Add TradeVectorService as separate app_data for vectorizing trade mistakes and notes
            .app_data(Data::new(app_data.as_ref().trade_vector_service.clone()))
            // CRITICAL: Add TradeVectorization as separate app_data for structured trade vectorization
            .app_data(Data::new(app_data.as_ref().trade_vectorization.clone()))
            // CRITICAL: Add PlaybookVectorization as separate app_data for playbook routes
            .app_data(Data::new(app_data.as_ref().playbook_vector_service.clone()))
            // CRITICAL: Add NotebookVectorization as separate app_data for notebook routes
            .app_data(Data::new(app_data.as_ref().notebook_vector_service.clone()))
            // CRITICAL: Add TradeNotesService as separate app_data for trade notes routes
            .app_data(Data::new(app_data.as_ref().trade_notes_service.clone()))
            // CRITICAL: Add ModelSelector as separate app_data for notebook AI routes
            .app_data(Data::new(app_data.as_ref().model_selector.clone()))
            // CRITICAL: Add CollaborationService as separate app_data for collaboration routes
            .app_data(collab_service_data.clone())
            .wrap(cors)
            .wrap(Logger::default())
            .wrap(actix_web::middleware::from_fn(rate_limit_middleware))
            // Register user routes FIRST with explicit logging
            .configure(|cfg| {
                log::info!("Configuring user routes");
                configure_user_routes(cfg);
            })
            // Register options routes (rate limiting handled in middleware)
            .configure(|cfg| {
                log::info!("Configuring options routes");
                configure_options_routes(cfg);
            })
            // Register stocks routes (rate limiting handled in middleware)
            .configure(|cfg| {
                log::info!("Configuring stocks routes");
                configure_stocks_routes(cfg);
            })
            // Register trade notes routes (rate limiting handled in middleware)
            .configure(|cfg| {
                log::info!("Configuring trade notes routes");
                configure_trade_notes_routes(cfg);
            })
            // Register trade tags routes (rate limiting handled in middleware)
            .configure(|cfg| {
                log::info!("Configuring trade tags routes");
                configure_trade_tags_routes(cfg);
            })
            // Register playbook routes (rate limiting handled in middleware)
            .configure(|cfg| {
                log::info!("Configuring playbook routes");
                configure_playbook_routes(cfg);
            })
            
            // Register notebook routes
            .configure(|cfg| {
                log::info!("Configuring notebook notes routes");
                configure_notebook_notes_routes(cfg);
            })
            .configure(|cfg| {
                log::info!("Configuring notebook folders routes");
                configure_notebook_folders_routes(cfg);
            })
            .configure(|cfg| {
                log::info!("Configuring notebook tags routes");
                configure_notebook_tags_routes(cfg);
            })
            .configure(|cfg| {
                log::info!("Configuring notebook images routes");
                configure_notebook_images_routes(cfg);
            })
            .configure(|cfg| {
                log::info!("Configuring notebook AI routes");
                configure_notebook_ai_routes(cfg);
            })
            .configure(|cfg| {
                log::info!("Configuring calendar routes");
                configure_calendar_routes(cfg);
            })
            .configure(|cfg| {
                log::info!("Configuring collaboration routes");
                configure_collaboration_routes(cfg);
            })

            // Register AI Analysis routes
            .configure(|cfg| {
                log::info!("Configuring AI analysis routes");
                configure_ai_analysis_routes(cfg);
            })
            // Register AI Chat routes
            .configure(|cfg| {
                log::info!("Configuring AI chat routes");
                configure_ai_chat_routes(cfg);
            })
            // Register AI Reports routes
            .configure(|cfg| {
                log::info!("Configuring AI reports routes");
                configure_ai_reports_routes(cfg);
            })
            // Register analytics routes
            .configure(|cfg| {
                log::info!("Configuring analytics routes");
                configure_analytics_routes(cfg);
            })
            // Register WebSocket routes
            .configure(|cfg| {
                log::info!("Configuring WebSocket routes");
                cfg.route("/api/ws", web::get().to(ws_handler));
                cfg.route("/ws/collab/{note_id}", web::get().to(collab_ws_handler));
            })
            // Register brokerage routes
            .configure(|cfg| {
                log::info!("Configuring brokerage routes");
                configure_brokerage_routes(cfg);
            })
            .configure(configure_public_routes)
            .configure(configure_auth_routes)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}

// Public routes configuration
fn configure_public_routes(cfg: &mut web::ServiceConfig) {
    log::info!("Configuring public routes");
    cfg.route("/", web::get().to(root_handler))
        .route("/health", web::get().to(health_check))
        .route(
            "/webhooks/supabase",
            web::post().to(supabase_webhook_handler),
        )
        .route("/profile", web::get().to(get_profile));

    // Market Data public routes
    crate::routes::market::configure_market_routes(cfg);
}

use middleware::rate_limit::rate_limit_middleware;

// Protected routes configuration
fn configure_auth_routes(cfg: &mut web::ServiceConfig) {
    log::info!("Configuring auth routes");
    cfg.service(
        web::scope("")
            .wrap(HttpAuthentication::bearer(jwt_validator))
            .wrap(actix_web::middleware::from_fn(rate_limit_middleware))
            .route("/me", web::get().to(get_current_user))
            .route("/my-data", web::get().to(get_user_data))
            // Push notification routes
            .configure(crate::routes::configure_push_routes)
            // Alert routes (rules + subscriptions)
            .configure(configure_alert_routes),
    );
}

// JWT validation middleware - Updated for Supabase Auth
pub async fn jwt_validator(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, (actix_web::Error, ServiceRequest)> {
    let config = req
        .app_data::<Config>()
        .cloned()
        .unwrap_or_else(Default::default);

    let app_state = req
        .app_data::<Data<AppState>>()
        .expect("AppState not found");

    // Try Supabase JWT validation first (no caching)
    match validate_supabase_jwt_token(credentials.token(), &app_state.config.supabase).await {
        Ok(claims) => {
            // Store Supabase claims in request extensions
            req.extensions_mut().insert(claims);
            Ok(req)
        }
        Err(_) => {
            let error = AuthenticationError::from(config).into();
            Err((error, req))
        }
    }
}

// Route handlers

async fn root_handler() -> ActixResult<Json<ApiResponse<HashMap<String, String>>>> {
    let mut data = HashMap::new();
    data.insert(
        "message".to_string(),
        "Tradistry API - Turso & Supabase Auth".to_string(),
    );
    data.insert("version".to_string(), "1.1.0".to_string());
    data.insert("auth_provider".to_string(), "supabase".to_string());
    Ok(Json(ApiResponse::success(data)))
}

async fn health_check(app_state: Data<AppState>) -> ActixResult<Json<ApiResponse<HealthCheck>>> {
    match app_state.health_check().await {
        Ok(_) => {
            let health = HealthCheck {
                status: "healthy".to_string(),
                database: "connected".to_string(),
                timestamp: chrono::Utc::now(),
            };
            Ok(Json(ApiResponse::success(health)))
        }
        Err(_) => {
            let health = HealthCheck {
                status: "unhealthy".to_string(),
                database: "disconnected".to_string(),
                timestamp: chrono::Utc::now(),
            };
            Ok(Json(ApiResponse::success(health)))
        }
    }
}

async fn get_profile(
    req: actix_web::HttpRequest,
) -> ActixResult<Json<ApiResponse<serde_json::Value>>> {
    // Try Supabase claims first
    if let Some(claims) = req.extensions().get::<SupabaseClaims>() {
        let profile = serde_json::json!({
            "user_id": claims.sub,
            "email": claims.email,
            "authenticated": true,
            "auth_provider": "supabase"
        });
        return Ok(Json(ApiResponse::success(profile)));
    }

    // No authentication
    let profile = serde_json::json!({
        "authenticated": false
    });
    Ok(Json(ApiResponse::success(profile)))
}

async fn get_current_user(
    app_state: Data<AppState>,
    req: actix_web::HttpRequest,
) -> ActixResult<Json<ApiResponse<serde_json::Value>>> {
    let (user_id, _email, auth_provider) = {
        let extensions = req.extensions();
        if let Some(supabase_claims) = extensions.get::<SupabaseClaims>() {
            // Handle Supabase claims
            let user_id = get_supabase_user_id(supabase_claims);
            (user_id, supabase_claims.email.clone(), "supabase")
        } else {
            return Err(actix_web::error::ErrorUnauthorized(
                "No authentication claims found",
            ));
        }
    };

    // Get user database entry from registry
    match app_state.turso_client.get_user_database(&user_id).await {
        Ok(Some(user_db)) => {
            let user_info = serde_json::json!({
                "user_id": user_db.user_id,
                "email": user_db.email,
                "database_name": user_db.db_name,
                "created_at": user_db.created_at,
                "updated_at": user_db.updated_at,
                "auth_provider": auth_provider
            });
            Ok(Json(ApiResponse::success(user_info)))
        }
        Ok(None) => Err(actix_web::error::ErrorNotFound("User not found")),
        Err(_) => Err(actix_web::error::ErrorInternalServerError("Database error")),
    }
}

async fn get_user_data(
    app_state: Data<AppState>,
    req: actix_web::HttpRequest,
) -> ActixResult<Json<ApiResponse<serde_json::Value>>> {
    let user_id = {
        let extensions = req.extensions();
        if let Some(supabase_claims) = extensions.get::<SupabaseClaims>() {
            // Handle Supabase claims
            get_supabase_user_id(supabase_claims)
        } else {
            return Err(actix_web::error::ErrorUnauthorized(
                "No authentication claims found",
            ));
        }
    };

    // Get user's database connection
    match app_state.get_user_db_connection(&user_id).await {
        Ok(Some(conn)) => {
            // Query user's personal data from their database
            // This is a placeholder - you'll implement actual queries based on your schema
            let mut rows = conn
                .prepare("SELECT COUNT(*) as record_count FROM user_info")
                .await
                .map_err(|_| actix_web::error::ErrorInternalServerError("Database query failed"))?
                .query(libsql::params![])
                .await
                .map_err(|_| actix_web::error::ErrorInternalServerError("Database query failed"))?;

            let count =
                if let Some(row) = rows.next().await.map_err(|_| {
                    actix_web::error::ErrorInternalServerError("Database query failed")
                })? {
                    row.get::<i64>(0).unwrap_or(0)
                } else {
                    0
                };

            let data = serde_json::json!({
                "user_id": user_id,
                "record_count": count,
                "message": "User data from personal database"
            });

            Ok(Json(ApiResponse::success(data)))
        }
        Ok(None) => {
            let error_data = serde_json::json!({
                "error": "User database not found",
                "user_id": user_id
            });
            Ok(Json(ApiResponse::success(error_data)))
        }
        Err(_) => Err(actix_web::error::ErrorInternalServerError(
            "Database connection error",
        )),
    }
}

/// Wrapper handler for Supabase webhooks
async fn supabase_webhook_handler(
    app_state: Data<AppState>,
    _req: actix_web::HttpRequest,
    body: web::Bytes,
) -> ActixResult<Json<ApiResponse<serde_json::Value>>> {
    // Parse the webhook payload
    let payload: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|_| actix_web::error::ErrorBadRequest("Invalid JSON payload"))?;

    log::info!("Received Supabase webhook: {:?}", payload);

    // Handle different Supabase Auth events
    if let Some(event_type) = payload.get("type").and_then(|t| t.as_str()) {
        match event_type {
            "user.created" => handle_user_created(&app_state, &payload).await,
            "user.updated" => handle_user_updated(&app_state, &payload).await,
            "user.deleted" => handle_user_deleted(&app_state, &payload).await,
            _ => {
                log::warn!("Unhandled Supabase webhook event: {}", event_type);
                Ok(Json(ApiResponse::success(serde_json::json!({
                    "message": "Event received but not handled",
                    "event_type": event_type
                }))))
            }
        }
    } else {
        Err(actix_web::error::ErrorBadRequest("Missing event type"))
    }
}

/// Handle user creation from Supabase webhook
async fn handle_user_created(
    app_state: &Data<AppState>,
    payload: &serde_json::Value,
) -> ActixResult<Json<ApiResponse<serde_json::Value>>> {
    if let Some(user_data) = payload.get("record") {
        if let Some(user_id) = user_data.get("id").and_then(|id| id.as_str()) {
            let email = user_data
                .get("email")
                .and_then(|e| e.as_str())
                .unwrap_or("unknown@example.com");

            // Create user database
            match app_state
                .turso_client
                .create_user_database(user_id, email)
                .await
            {
                Ok(_) => {
                    log::info!("Created database for Supabase user: {}", user_id);
                    Ok(Json(ApiResponse::success(serde_json::json!({
                        "message": "User database created successfully",
                        "user_id": user_id
                    }))))
                }
                Err(e) => {
                    log::error!("Failed to create database for user {}: {}", user_id, e);
                    Err(actix_web::error::ErrorInternalServerError(
                        "Database creation failed",
                    ))
                }
            }
        } else {
            Err(actix_web::error::ErrorBadRequest(
                "Missing user ID in payload",
            ))
        }
    } else {
        Err(actix_web::error::ErrorBadRequest(
            "Missing user record in payload",
        ))
    }
}

/// Handle user update from Supabase webhook
async fn handle_user_updated(
    _app_state: &Data<AppState>,
    payload: &serde_json::Value,
) -> ActixResult<Json<ApiResponse<serde_json::Value>>> {
    if let Some(user_data) = payload.get("record") {
        if let Some(user_id) = user_data.get("id").and_then(|id| id.as_str()) {
            log::info!("User updated: {}", user_id);
            Ok(Json(ApiResponse::success(serde_json::json!({
                "message": "User update acknowledged",
                "user_id": user_id
            }))))
        } else {
            Err(actix_web::error::ErrorBadRequest(
                "Missing user ID in payload",
            ))
        }
    } else {
        Err(actix_web::error::ErrorBadRequest(
            "Missing user record in payload",
        ))
    }
}

/// Handle user deletion from Supabase webhook
async fn handle_user_deleted(
    _app_state: &Data<AppState>,
    payload: &serde_json::Value,
) -> ActixResult<Json<ApiResponse<serde_json::Value>>> {
    if let Some(user_data) = payload.get("old_record") {
        if let Some(user_id) = user_data.get("id").and_then(|id| id.as_str()) {
            // Optionally delete user database or mark as inactive
            log::info!("User deleted: {}", user_id);
            Ok(Json(ApiResponse::success(serde_json::json!({
                "message": "User deletion acknowledged",
                "user_id": user_id
            }))))
        } else {
            Err(actix_web::error::ErrorBadRequest(
                "Missing user ID in payload",
            ))
        }
    } else {
        Err(actix_web::error::ErrorBadRequest(
            "Missing user record in payload",
        ))
    }
}
