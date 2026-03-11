use actix_web::{HttpRequest, HttpResponse, Result as ActixResult, web};
use chrono::Utc;
use libsql::Connection;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::models::playbook::{
    CreatePlaybookRequest, CreateRuleRequest, Playbook, PlaybookQuery, PlaybookRule,
    TagTradeRequest, TradeType, UpdatePlaybookRequest, UpdateRuleRequest,
};
use crate::models::stock::stocks::TimeRange;
use crate::service::ai_service::PlaybookVectorization;
use crate::service::analytics_engine::playbook_analytics::calculate_playbook_analytics;
use crate::service::caching::CacheService;
use crate::turso::auth::AuthError;
use crate::turso::config::{SupabaseClaims, SupabaseConfig};
use crate::turso::{AppState, client::TursoClient};
use crate::websocket::{ConnectionManager, broadcast_playbook_update};
use actix_web::web::Data;
use std::sync::Arc as StdArc;
use tokio::sync::Mutex;

/// Response wrapper for playbook operations
#[derive(Debug, Serialize)]
pub struct PlaybookResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<Playbook>,
}

/// Response wrapper for playbook list operations
#[derive(Debug, Serialize)]
pub struct PlaybookListResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<Vec<Playbook>>,
    pub total: Option<i64>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// Query parameters for playbook endpoints
#[derive(Debug, Deserialize)]
pub struct PlaybookQueryParams {
    pub name: Option<String>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// Response for trade tagging operations
#[derive(Debug, Serialize)]
pub struct TagTradeResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// Parse JWT claims without full validation (for middleware)
fn parse_jwt_claims(token: &str) -> Result<SupabaseClaims, AuthError> {
    use base64::Engine;

    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(AuthError::InvalidToken);
    }

    let payload = parts[1];
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| AuthError::InvalidToken)?;

    let claims: SupabaseClaims =
        serde_json::from_slice(&decoded).map_err(|_| AuthError::InvalidToken)?;

    Ok(claims)
}

/// Extract JWT token from Authorization header
fn extract_token_from_request(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("Authorization")
        .and_then(|header| header.to_str().ok())
        .and_then(|auth_header| {
            auth_header
                .strip_prefix("Bearer ")
                .map(|token| token.to_string())
        })
}

/// Get authenticated user from request
async fn get_authenticated_user(
    req: &HttpRequest,
    _supabase_config: &SupabaseConfig,
) -> Result<SupabaseClaims, actix_web::Error> {
    let token = extract_token_from_request(req).ok_or_else(|| {
        actix_web::error::ErrorUnauthorized("Missing or invalid authorization header")
    })?;

    let claims = parse_jwt_claims(&token)
        .map_err(|_| actix_web::error::ErrorUnauthorized("Invalid token"))?;

    Ok(claims)
}

/// Get user database connection
async fn get_user_database_connection(
    user_id: &str,
    turso_client: &Arc<TursoClient>,
) -> Result<Connection, actix_web::Error> {
    turso_client
        .get_user_database_connection(user_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Database error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))
}

/// Helper function to vectorize a playbook asynchronously
/// Fetches playbook, rules, and analytics, then vectorizes
async fn vectorize_playbook_async(
    user_id: String,
    playbook_id: String,
    turso_client: Arc<TursoClient>,
    playbook_vector_service: Arc<PlaybookVectorization>,
) {
    if let Ok(Some(conn)) = turso_client.get_user_database_connection(&user_id).await {
        // Fetch playbook
        let playbook = match Playbook::find_by_id(&conn, &playbook_id).await {
            Ok(Some(pb)) => pb,
            Ok(None) => {
                log::warn!("Playbook {} not found for vectorization", playbook_id);
                return;
            }
            Err(e) => {
                log::error!(
                    "Failed to fetch playbook {} for vectorization: {}",
                    playbook_id,
                    e
                );
                return;
            }
        };

        // Fetch rules
        let rules = match PlaybookRule::find_by_playbook_id(&conn, &playbook_id).await {
            Ok(r) => r,
            Err(e) => {
                log::warn!("Failed to fetch rules for playbook {}: {}", playbook_id, e);
                vec![]
            }
        };

        // Fetch analytics (use default time range - all time)
        let time_range = TimeRange::AllTime;
        let analytics = match calculate_playbook_analytics(&conn, &playbook_id, &time_range).await {
            Ok(a) => Some(a),
            Err(e) => {
                log::debug!(
                    "No analytics available for playbook {} (new playbook?): {}",
                    playbook_id,
                    e
                );
                None
            }
        };

        // Vectorize
        if let Err(e) = playbook_vector_service
            .vectorize_playbook(&user_id, &playbook, &rules, analytics.as_ref())
            .await
        {
            log::error!("Failed to vectorize playbook {}: {}", playbook_id, e);
        } else {
            log::info!("Successfully vectorized playbook {}", playbook_id);
        }
    } else {
        log::error!(
            "Failed to get database connection for user {} to vectorize playbook {}",
            user_id,
            playbook_id
        );
    }
}

/// Create a new playbook setup
/// Create a playbook with cache invalidation
pub async fn create_playbook(
    req: HttpRequest,
    payload: web::Json<CreatePlaybookRequest>,
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    cache_service: web::Data<Arc<CacheService>>,
    ws_manager: Data<StdArc<Mutex<ConnectionManager>>>,
    playbook_vector_service: web::Data<Arc<PlaybookVectorization>>,
) -> ActixResult<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = &claims.sub;

    let conn = get_user_database_connection(user_id, &app_state.turso_client).await?;

    // Check storage quota before creating
    app_state
        .storage_quota_service
        .check_storage_quota(user_id, &conn)
        .await
        .map_err(|e| {
            error!("Storage quota check failed for user {}: {}", user_id, e);
            e
        })?;

    match Playbook::create(&conn, payload.into_inner()).await {
        Ok(playbook) => {
            // Invalidate cache after successful creation
            let cache_service_clone = cache_service.get_ref().clone();
            let user_id_clone = user_id.clone();

            tokio::spawn(async move {
                match cache_service_clone
                    .invalidate_table_cache(&user_id_clone, "playbook")
                    .await
                {
                    Ok(count) => info!(
                        "Invalidated {} playbook cache keys for user: {}",
                        count, user_id_clone
                    ),
                    Err(e) => error!(
                        "Failed to invalidate playbook cache for user {}: {}",
                        user_id_clone, e
                    ),
                }
            });

            // Broadcast create
            let ws_manager_clone = ws_manager.clone();
            let user_id_ws = user_id.clone();
            let playbook_ws = playbook.clone();
            tokio::spawn(async move {
                broadcast_playbook_update(ws_manager_clone, &user_id_ws, "created", &playbook_ws)
                    .await;
            });

            // Vectorize playbook asynchronously
            let user_id_vec = user_id.clone();
            let playbook_id_vec = playbook.id.clone();
            let turso_client_vec = Arc::clone(&app_state.turso_client);
            let playbook_vector_service_vec = Arc::clone(playbook_vector_service.get_ref());
            tokio::spawn(async move {
                vectorize_playbook_async(
                    user_id_vec,
                    playbook_id_vec,
                    turso_client_vec,
                    playbook_vector_service_vec,
                )
                .await;
            });

            Ok(HttpResponse::Created().json(PlaybookResponse {
                success: true,
                message: "Playbook created successfully".to_string(),
                data: Some(playbook),
            }))
        }
        Err(e) => {
            log::error!("Failed to create playbook: {}", e);
            Ok(HttpResponse::InternalServerError().json(PlaybookResponse {
                success: false,
                message: "Failed to create playbook".to_string(),
                data: None,
            }))
        }
    }
}

/// Get a playbook by ID
pub async fn get_playbook(
    req: HttpRequest,
    playbook_id: web::Path<String>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> ActixResult<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = &claims.sub;

    let conn = get_user_database_connection(user_id, &turso_client).await?;

    match Playbook::find_by_id(&conn, &playbook_id).await {
        Ok(Some(playbook)) => {
            if let Ok(json) = serde_json::to_string_pretty(&playbook) {
                info!("Playbook fetched. Sample payload:\n{}", json);
            }
            Ok(HttpResponse::Ok().json(PlaybookResponse {
                success: true,
                message: "Playbook retrieved successfully".to_string(),
                data: Some(playbook),
            }))
        }
        Ok(None) => Ok(HttpResponse::NotFound().json(PlaybookResponse {
            success: false,
            message: "Playbook not found".to_string(),
            data: None,
        })),
        Err(e) => {
            log::error!("Failed to get playbook: {}", e);
            Ok(HttpResponse::InternalServerError().json(PlaybookResponse {
                success: false,
                message: "Failed to retrieve playbook".to_string(),
                data: None,
            }))
        }
    }
}

/// Get all playbooks with optional filtering
/// Get playbooks with caching
pub async fn get_playbooks(
    req: HttpRequest,
    query: web::Query<PlaybookQueryParams>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
    cache_service: web::Data<Arc<CacheService>>,
) -> ActixResult<HttpResponse> {
    info!("🔵 Starting get_playbooks request");

    let claims = match get_authenticated_user(&req, &supabase_config).await {
        Ok(claims) => {
            info!("✓ User authenticated: {}", claims.sub);
            claims
        }
        Err(e) => {
            error!("❌ Authentication failed: {:?}", e);
            return Err(e);
        }
    };
    let user_id = &claims.sub;

    info!("🔵 Getting database connection for user: {}", user_id);
    let conn = match get_user_database_connection(user_id, &turso_client).await {
        Ok(conn) => {
            info!("✓ Database connection established");
            conn
        }
        Err(e) => {
            error!("❌ Failed to get database connection: {:?}", e);
            return Err(e);
        }
    };

    let playbook_query = PlaybookQuery {
        name: query.name.clone(),
        search: query.search.clone(),
        limit: query.limit.or(query.page_size),
        offset: query.offset.or_else(|| {
            query
                .page
                .and_then(|page| query.page_size.map(|page_size| (page - 1) * page_size))
        }),
    };
    info!("🔵 Query parameters: {:?}", playbook_query);

    // Generate cache key based on query parameters
    let query_hash = format!("{:?}", playbook_query);
    let cache_key = format!("db:{}:playbook:list:{}", user_id, query_hash);
    info!("🔵 Cache key: {}", cache_key);

    // Try to get from cache first (1 hour TTL as per plan)
    info!("🔵 Attempting to fetch from cache or database...");
    match cache_service
        .get_or_fetch(&cache_key, 3600, || async {
            info!("⚠️ Cache miss for playbooks list, fetching from database");

            info!("🔵 Calling Playbook::find_all...");
            let playbooks_result = Playbook::find_all(&conn, playbook_query.clone()).await;

            match &playbooks_result {
                Ok(playbooks) => info!(
                    "✓ Playbook::find_all returned {} playbooks",
                    playbooks.len()
                ),
                Err(e) => error!("❌ Playbook::find_all failed: {}", e),
            }

            info!("🔵 Calling Playbook::total_count...");
            let total_result = Playbook::total_count(&conn).await;

            match &total_result {
                Ok(total) => info!("✓ Playbook::total_count returned: {}", total),
                Err(e) => error!("❌ Playbook::total_count failed: {}", e),
            }

            match (playbooks_result, total_result) {
                (Ok(playbooks), Ok(total)) => {
                    info!("✓ Both database queries succeeded");
                    Ok((playbooks, total))
                }
                (Err(e), _) => {
                    error!("❌ Playbook::find_all error: {}", e);
                    Err(anyhow::anyhow!("{}", e))
                }
                (_, Err(e)) => {
                    error!("❌ Playbook::total_count error: {}", e);
                    Err(anyhow::anyhow!("{}", e))
                }
            }
        })
        .await
    {
        Ok((playbooks, total)) => {
            info!(
                "✅ Successfully retrieved {} playbooks (total: {})",
                playbooks.len(),
                total
            );

            if let Some(first) = playbooks.first() {
                info!("🔵 Serializing first playbook for logging...");
                if let Ok(json) = serde_json::to_string_pretty(first) {
                    info!("Playbooks list sample (first item):\n{}", json);
                } else {
                    error!("❌ Failed to serialize first playbook");
                }
            } else {
                debug!("Playbooks list is empty");
            }

            info!("🔵 Creating response JSON...");
            let response = PlaybookListResponse {
                success: true,
                message: "Playbooks retrieved successfully".to_string(),
                data: Some(playbooks),
                total: Some(total),
                page: query.page,
                page_size: query.page_size,
            };

            info!("🔵 Serializing response...");
            match serde_json::to_string(&response) {
                Ok(json_str) => {
                    info!(
                        "✓ Response serialized successfully, length: {} bytes",
                        json_str.len()
                    );
                    info!(
                        "Response preview (first 500 chars): {}",
                        if json_str.len() > 500 {
                            &json_str[..500]
                        } else {
                            &json_str
                        }
                    );
                }
                Err(e) => {
                    error!("❌ CRITICAL: Failed to serialize response: {}", e);
                }
            }

            info!("✅ Returning HTTP 200 response");
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            error!("❌ FINAL ERROR: Failed to get playbooks: {}", e);
            error!("Error details: {:?}", e);
            Ok(
                HttpResponse::InternalServerError().json(PlaybookListResponse {
                    success: false,
                    message: "Failed to retrieve playbooks".to_string(),
                    data: None,
                    total: None,
                    page: None,
                    page_size: None,
                }),
            )
        }
    }
}

/// Update a playbook setup
pub async fn update_playbook(
    req: HttpRequest,
    playbook_id: web::Path<String>,
    payload: web::Json<UpdatePlaybookRequest>,
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    ws_manager: web::Data<StdArc<Mutex<ConnectionManager>>>,
    playbook_vector_service: web::Data<Arc<PlaybookVectorization>>,
) -> ActixResult<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = &claims.sub;

    let conn = get_user_database_connection(user_id, &app_state.turso_client).await?;

    match Playbook::update(&conn, &playbook_id, payload.into_inner()).await {
        Ok(Some(playbook)) => {
            // Broadcast update via WebSocket
            let ws_manager_clone = ws_manager.clone();
            let user_id_ws = user_id.clone();
            let playbook_ws = playbook.clone();
            tokio::spawn(async move {
                broadcast_playbook_update(ws_manager_clone, &user_id_ws, "updated", &playbook_ws)
                    .await;
            });

            // Vectorize playbook asynchronously
            let user_id_vec = user_id.clone();
            let playbook_id_vec = playbook_id.clone();
            let turso_client_vec = Arc::clone(&app_state.turso_client);
            let playbook_vector_service_vec = Arc::clone(playbook_vector_service.get_ref());
            tokio::spawn(async move {
                vectorize_playbook_async(
                    user_id_vec,
                    playbook_id_vec,
                    turso_client_vec,
                    playbook_vector_service_vec,
                )
                .await;
            });

            Ok(HttpResponse::Ok().json(PlaybookResponse {
                success: true,
                message: "Playbook updated successfully".to_string(),
                data: Some(playbook),
            }))
        }
        Ok(None) => Ok(HttpResponse::NotFound().json(PlaybookResponse {
            success: false,
            message: "Playbook not found".to_string(),
            data: None,
        })),
        Err(e) => {
            log::error!("Failed to update playbook: {}", e);
            Ok(HttpResponse::InternalServerError().json(PlaybookResponse {
                success: false,
                message: "Failed to update playbook".to_string(),
                data: None,
            }))
        }
    }
}

/// Delete a playbook setup
pub async fn delete_playbook(
    req: HttpRequest,
    playbook_id: web::Path<String>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
    ws_manager: web::Data<StdArc<Mutex<ConnectionManager>>>,
) -> ActixResult<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = &claims.sub;

    let conn = get_user_database_connection(user_id, &turso_client).await?;

    match Playbook::delete(&conn, &playbook_id).await {
        Ok(true) => {
            // Broadcast delete
            let ws_manager_clone = ws_manager.clone();
            let user_id_ws = user_id.clone();
            let id_ws = playbook_id.clone();
            tokio::spawn(async move {
                broadcast_playbook_update(
                    ws_manager_clone,
                    &user_id_ws,
                    "deleted",
                    serde_json::json!({"id": id_ws}),
                )
                .await;
            });
            Ok(HttpResponse::Ok().json(PlaybookResponse {
                success: true,
                message: "Playbook deleted successfully".to_string(),
                data: None,
            }))
        }
        Ok(false) => Ok(HttpResponse::NotFound().json(PlaybookResponse {
            success: false,
            message: "Playbook not found".to_string(),
            data: None,
        })),
        Err(e) => {
            log::error!("Failed to delete playbook: {}", e);
            Ok(HttpResponse::InternalServerError().json(PlaybookResponse {
                success: false,
                message: "Failed to delete playbook".to_string(),
                data: None,
            }))
        }
    }
}

/// Tag a trade with a playbook setup
pub async fn tag_trade(
    req: HttpRequest,
    payload: web::Json<TagTradeRequest>,
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    playbook_vector_service: web::Data<Arc<PlaybookVectorization>>,
) -> ActixResult<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = &claims.sub;

    let conn = get_user_database_connection(user_id, &app_state.turso_client).await?;
    let request = payload.into_inner();
    let setup_id = request.setup_id.clone();

    let result = match request.trade_type {
        TradeType::Stock => Playbook::tag_stock_trade(&conn, request.trade_id, &request.setup_id)
            .await
            .map(|association| serde_json::to_value(association).unwrap_or_default()),
        TradeType::Option => Playbook::tag_option_trade(&conn, request.trade_id, &request.setup_id)
            .await
            .map(|association| serde_json::to_value(association).unwrap_or_default()),
    };

    match result {
        Ok(association) => {
            // Vectorize playbook asynchronously (analytics will be recalculated)
            let user_id_vec = user_id.clone();
            let setup_id_vec = setup_id.clone();
            let turso_client_vec = Arc::clone(&app_state.turso_client);
            let playbook_vector_service_vec = Arc::clone(playbook_vector_service.get_ref());
            tokio::spawn(async move {
                vectorize_playbook_async(
                    user_id_vec,
                    setup_id_vec,
                    turso_client_vec,
                    playbook_vector_service_vec,
                )
                .await;
            });

            Ok(HttpResponse::Created().json(TagTradeResponse {
                success: true,
                message: "Trade tagged successfully".to_string(),
                data: Some(association),
            }))
        }
        Err(e) => {
            log::error!("Failed to tag trade: {}", e);
            Ok(HttpResponse::InternalServerError().json(TagTradeResponse {
                success: false,
                message: "Failed to tag trade".to_string(),
                data: None,
            }))
        }
    }
}

/// Remove a playbook tag from a trade
pub async fn untag_trade(
    req: HttpRequest,
    payload: web::Json<TagTradeRequest>,
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    playbook_vector_service: web::Data<Arc<PlaybookVectorization>>,
) -> ActixResult<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = &claims.sub;

    let conn = get_user_database_connection(user_id, &app_state.turso_client).await?;
    let request = payload.into_inner();
    let setup_id = request.setup_id.clone();

    let result = match request.trade_type {
        TradeType::Stock => {
            Playbook::untag_stock_trade(&conn, request.trade_id, &request.setup_id).await
        }
        TradeType::Option => {
            Playbook::untag_option_trade(&conn, request.trade_id, &request.setup_id).await
        }
    };

    match result {
        Ok(true) => {
            // Vectorize playbook asynchronously (analytics will be recalculated)
            let user_id_vec = user_id.clone();
            let setup_id_vec = setup_id.clone();
            let turso_client_vec = Arc::clone(&app_state.turso_client);
            let playbook_vector_service_vec = Arc::clone(playbook_vector_service.get_ref());
            tokio::spawn(async move {
                vectorize_playbook_async(
                    user_id_vec,
                    setup_id_vec,
                    turso_client_vec,
                    playbook_vector_service_vec,
                )
                .await;
            });

            Ok(HttpResponse::Ok().json(TagTradeResponse {
                success: true,
                message: "Trade untagged successfully".to_string(),
                data: None,
            }))
        }
        Ok(false) => Ok(HttpResponse::NotFound().json(TagTradeResponse {
            success: false,
            message: "Trade tag not found".to_string(),
            data: None,
        })),
        Err(e) => {
            log::error!("Failed to untag trade: {}", e);
            Ok(HttpResponse::InternalServerError().json(TagTradeResponse {
                success: false,
                message: "Failed to untag trade".to_string(),
                data: None,
            }))
        }
    }
}

/// Get playbook setups for a specific trade
pub async fn get_trade_playbooks(
    req: HttpRequest,
    trade_id: web::Path<i64>,
    query: web::Query<serde_json::Map<String, serde_json::Value>>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> ActixResult<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = &claims.sub;

    let conn = get_user_database_connection(user_id, &turso_client).await?;

    // Get trade type from query parameters
    let trade_type = query
        .get("trade_type")
        .and_then(|v| v.as_str())
        .unwrap_or("stock");

    let result = match trade_type {
        "option" => Playbook::get_option_trade_playbooks(&conn, *trade_id).await,
        _ => Playbook::get_stock_trade_playbooks(&conn, *trade_id).await,
    };

    match result {
        Ok(playbooks) => {
            info!(
                "Retrieved {} playbooks for trade {}",
                playbooks.len(),
                *trade_id
            );
            if let Some(first) = playbooks.first() {
                if let Ok(json) = serde_json::to_string_pretty(first) {
                    info!("Trade playbooks sample (first item):\n{}", json);
                }
            } else {
                debug!("No playbooks associated with this trade");
            }
            Ok(HttpResponse::Ok().json(PlaybookListResponse {
                success: true,
                message: "Trade playbooks retrieved successfully".to_string(),
                data: Some(playbooks),
                total: None,
                page: None,
                page_size: None,
            }))
        }
        Err(e) => {
            log::error!("Failed to get trade playbooks: {}", e);
            Ok(
                HttpResponse::InternalServerError().json(PlaybookListResponse {
                    success: false,
                    message: "Failed to retrieve trade playbooks".to_string(),
                    data: None,
                    total: None,
                    page: None,
                    page_size: None,
                }),
            )
        }
    }
}

/// Get all trades tagged with a specific playbook setup
pub async fn get_playbook_trades(
    req: HttpRequest,
    setup_id: web::Path<String>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> ActixResult<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = &claims.sub;

    let conn = get_user_database_connection(user_id, &turso_client).await?;

    match Playbook::get_playbook_trades(&conn, &setup_id).await {
        Ok((stock_trades, option_trades)) => {
            let data = serde_json::json!({
                "stock_trades": stock_trades,
                "option_trades": option_trades
            });
            if let Ok(json) = serde_json::to_string_pretty(&data) {
                info!("Playbook trades payload sample:\n{}", json);
            }

            Ok(HttpResponse::Ok().json(TagTradeResponse {
                success: true,
                message: "Playbook trades retrieved successfully".to_string(),
                data: Some(data),
            }))
        }
        Err(e) => {
            log::error!("Failed to get playbook trades: {}", e);
            Ok(HttpResponse::InternalServerError().json(TagTradeResponse {
                success: false,
                message: "Failed to retrieve playbook trades".to_string(),
                data: None,
            }))
        }
    }
}

/// Get playbooks count
pub async fn get_playbooks_count(
    req: HttpRequest,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> ActixResult<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = &claims.sub;

    let conn = get_user_database_connection(user_id, &turso_client).await?;

    match Playbook::total_count(&conn).await {
        Ok(count) => {
            info!("Playbooks count: {}", count);
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "message": "Playbooks count retrieved successfully",
                "data": {
                    "count": count
                }
            })))
        }
        Err(e) => {
            log::error!("Failed to get playbooks count: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "message": "Failed to retrieve playbooks count",
                "data": null
            })))
        }
    }
}

/// Test endpoint to verify playbook routes are working
async fn test_playbook_endpoint() -> ActixResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Playbook routes are working",
        "data": {
            "timestamp": Utc::now().to_rfc3339()
        }
    })))
}

/// Configure playbook routes
pub fn configure_playbook_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/playbooks")
            // Existing playbook CRUD
            .route("", web::post().to(create_playbook))
            .route("", web::get().to(get_playbooks))
            .route("/count", web::get().to(get_playbooks_count))
            .route("/test", web::get().to(test_playbook_endpoint))
            // Analytics (place static route BEFORE dynamic `/{id}` to avoid shadowing)
            .route("/analytics", web::get().to(get_all_playbooks_analytics))
            .route("/{id}/analytics", web::get().to(get_playbook_analytics))
            // Dynamic ID routes
            .route("/{id}", web::get().to(get_playbook))
            .route("/{id}", web::put().to(update_playbook))
            .route("/{id}", web::delete().to(delete_playbook))
            .route("/tag", web::post().to(tag_trade))
            .route("/untag", web::delete().to(untag_trade))
            .route("/trades/{trade_id}", web::get().to(get_trade_playbooks))
            .route("/{setup_id}/trades", web::get().to(get_playbook_trades))
            // Rules management
            .route("/{id}/rules", web::post().to(create_playbook_rule))
            .route("/{id}/rules", web::get().to(get_playbook_rules))
            .route("/{id}/rules/{rule_id}", web::put().to(update_playbook_rule))
            .route(
                "/{id}/rules/{rule_id}",
                web::delete().to(delete_playbook_rule),
            )
            // Missed trades
            .route("/{id}/missed-trades", web::post().to(create_missed_trade))
            .route("/{id}/missed-trades", web::get().to(get_missed_trades))
            .route(
                "/{id}/missed-trades/{missed_id}",
                web::delete().to(delete_missed_trade),
            ),
    );
}

// New route handlers
async fn create_playbook_rule(
    req: HttpRequest,
    path: web::Path<String>,
    payload: web::Json<CreateRuleRequest>,
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    playbook_vector_service: web::Data<Arc<PlaybookVectorization>>,
) -> ActixResult<HttpResponse> {
    let playbook_id = path.into_inner();
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = &claims.sub;

    let conn = get_user_database_connection(user_id, &app_state.turso_client).await?;

    match PlaybookRule::create(&conn, &playbook_id, payload.into_inner()).await {
        Ok(rule) => {
            // Vectorize playbook asynchronously
            let user_id_vec = user_id.clone();
            let playbook_id_vec = playbook_id.clone();
            let turso_client_vec = Arc::clone(&app_state.turso_client);
            let playbook_vector_service_vec = Arc::clone(playbook_vector_service.get_ref());
            tokio::spawn(async move {
                vectorize_playbook_async(
                    user_id_vec,
                    playbook_id_vec,
                    turso_client_vec,
                    playbook_vector_service_vec,
                )
                .await;
            });

            Ok(HttpResponse::Created().json(serde_json::json!({
                "success": true,
                "message": "Rule created successfully",
                "data": rule
            })))
        }
        Err(e) => {
            error!("Failed to create rule: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "message": format!("Failed to create rule: {}", e),
                "data": null
            })))
        }
    }
}

async fn get_playbook_rules(
    req: HttpRequest,
    path: web::Path<String>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> ActixResult<HttpResponse> {
    let playbook_id = path.into_inner();
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = &claims.sub;

    let conn = get_user_database_connection(user_id, &turso_client).await?;

    match PlaybookRule::find_by_playbook_id(&conn, &playbook_id).await {
        Ok(rules) => {
            info!(
                "Retrieved {} rules for playbook {}",
                rules.len(),
                playbook_id
            );
            if let Some(first) = rules.first() {
                if let Ok(json) = serde_json::to_string_pretty(first) {
                    info!("Rules list sample (first item):\n{}", json);
                }
            } else {
                debug!("No rules found for playbook");
            }
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "message": "Rules retrieved successfully",
                "data": rules
            })))
        }
        Err(e) => {
            error!("Failed to get rules: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "message": format!("Failed to retrieve rules: {}", e),
                "data": null
            })))
        }
    }
}

async fn update_playbook_rule(
    req: HttpRequest,
    paths: web::Path<(String, String)>,
    payload: web::Json<UpdateRuleRequest>,
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    playbook_vector_service: web::Data<Arc<PlaybookVectorization>>,
) -> ActixResult<HttpResponse> {
    let (playbook_id, rule_id) = paths.into_inner();
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = &claims.sub;

    let conn = get_user_database_connection(user_id, &app_state.turso_client).await?;

    match PlaybookRule::update(&conn, &rule_id, payload.into_inner()).await {
        Ok(rule) => {
            // Vectorize playbook asynchronously
            let user_id_vec = user_id.clone();
            let playbook_id_vec = playbook_id.clone();
            let turso_client_vec = Arc::clone(&app_state.turso_client);
            let playbook_vector_service_vec = Arc::clone(playbook_vector_service.get_ref());
            tokio::spawn(async move {
                vectorize_playbook_async(
                    user_id_vec,
                    playbook_id_vec,
                    turso_client_vec,
                    playbook_vector_service_vec,
                )
                .await;
            });

            Ok(HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "message": "Rule updated successfully",
                "data": rule
            })))
        }
        Err(e) => {
            error!("Failed to update rule: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "message": format!("Failed to update rule: {}", e),
                "data": null
            })))
        }
    }
}

async fn delete_playbook_rule(
    req: HttpRequest,
    paths: web::Path<(String, String)>,
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    playbook_vector_service: web::Data<Arc<PlaybookVectorization>>,
) -> ActixResult<HttpResponse> {
    let (playbook_id, rule_id) = paths.into_inner();
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = &claims.sub;

    let conn = get_user_database_connection(user_id, &app_state.turso_client).await?;

    match PlaybookRule::delete(&conn, &rule_id).await {
        Ok(_) => {
            // Vectorize playbook asynchronously
            let user_id_vec = user_id.clone();
            let playbook_id_vec = playbook_id.clone();
            let turso_client_vec = Arc::clone(&app_state.turso_client);
            let playbook_vector_service_vec = Arc::clone(playbook_vector_service.get_ref());
            tokio::spawn(async move {
                vectorize_playbook_async(
                    user_id_vec,
                    playbook_id_vec,
                    turso_client_vec,
                    playbook_vector_service_vec,
                )
                .await;
            });

            Ok(HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "message": "Rule deleted successfully"
            })))
        }
        Err(e) => {
            error!("Failed to delete rule: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "message": format!("Failed to delete rule: {}", e),
                "data": null
            })))
        }
    }
}

async fn create_missed_trade() -> ActixResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Not implemented yet"
    })))
}

async fn get_missed_trades() -> ActixResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Not implemented yet"
    })))
}

async fn delete_missed_trade() -> ActixResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Not implemented yet"
    })))
}

/// Get analytics for a specific playbook
async fn get_playbook_analytics(
    req: HttpRequest,
    path: web::Path<(String,)>,
    web::Query(params): web::Query<std::collections::HashMap<String, String>>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> ActixResult<HttpResponse> {
    let playbook_id = &path.0;
    info!("Getting analytics for playbook: {}", playbook_id);

    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = &claims.sub;

    let conn = get_user_database_connection(user_id, &turso_client).await?;

    // Parse time range from query params (default to all time)
    let time_range = params
        .get("timeRange")
        .and_then(|s| serde_json::from_str::<TimeRange>(s).ok())
        .unwrap_or(TimeRange::AllTime);

    match calculate_playbook_analytics(&conn, playbook_id, &time_range).await {
        Ok(analytics) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Playbook analytics retrieved successfully",
            "data": analytics
        }))),
        Err(e) => {
            error!("Failed to calculate playbook analytics: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "message": format!("Failed to retrieve playbook analytics: {}", e),
                "data": null
            })))
        }
    }
}

/// Get analytics for all playbooks
async fn get_all_playbooks_analytics(
    req: HttpRequest,
    web::Query(params): web::Query<std::collections::HashMap<String, String>>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> ActixResult<HttpResponse> {
    info!("Getting analytics for all playbooks");

    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = &claims.sub;

    let conn = get_user_database_connection(user_id, &turso_client).await?;

    // Parse time range from query params (default to all time)
    let time_range = params
        .get("timeRange")
        .and_then(|s| serde_json::from_str::<TimeRange>(s).ok())
        .unwrap_or(TimeRange::AllTime);

    // Get all playbooks for this user
    match Playbook::find_all(
        &conn,
        PlaybookQuery {
            name: None,
            search: None,
            limit: None,
            offset: None,
        },
    )
    .await
    {
        Ok(playbooks) => {
            let mut all_analytics = Vec::new();

            for playbook in playbooks {
                match calculate_playbook_analytics(&conn, &playbook.id, &time_range).await {
                    Ok(analytics) => all_analytics.push(analytics),
                    Err(e) => {
                        error!(
                            "Failed to calculate analytics for playbook {}: {}",
                            playbook.id, e
                        );
                    }
                }
            }

            Ok(HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "message": "All playbooks analytics retrieved successfully",
                "data": all_analytics,
                "total": all_analytics.len()
            })))
        }
        Err(e) => {
            error!("Failed to get playbooks: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "message": format!("Failed to retrieve playbooks: {}", e),
                "data": null
            })))
        }
    }
}
