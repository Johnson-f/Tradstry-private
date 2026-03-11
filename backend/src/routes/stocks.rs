use crate::models::stock::stocks::{
    CreateStockRequest, Stock, StockQuery, TimeRange, UpdateStockRequest,
};
use crate::service::ai_service::TradeVectorService;
use crate::service::caching::CacheService;
use crate::turso::auth::{AuthError, validate_supabase_jwt_token};
use crate::turso::config::{SupabaseClaims, SupabaseConfig};
use crate::turso::{AppState, client::TursoClient};
use crate::websocket::{ConnectionManager, broadcast_stock_update};
use actix_web::{HttpRequest, HttpResponse, Result, web};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Response wrapper for API responses
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
}

impl<T> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
        }
    }

    fn error(message: &str) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            message: Some(message.to_string()),
        }
    }
}

/// Analytics response structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StocksAnalytics {
    pub total_pnl: String,
    pub profit_factor: String,
    pub win_rate: String,
    pub loss_rate: String,
    pub avg_gain: String,
    pub avg_loss: String,
    pub biggest_winner: String,
    pub biggest_loser: String,
    pub avg_hold_time_winners: String,
    pub avg_hold_time_losers: String,
    pub risk_reward_ratio: String,
    pub trade_expectancy: String,
    pub avg_position_size: String,
    pub net_pnl: String,
}

/// Parse JWT claims without full validation (for middleware)
fn parse_jwt_claims(token: &str) -> Result<SupabaseClaims, AuthError> {
    use base64::{Engine as _, engine::general_purpose};

    info!("Parsing JWT token, length: {}", token.len());

    let parts: Vec<&str> = token.split('.').collect();
    info!("JWT parts count: {}", parts.len());

    if parts.len() != 3 {
        error!("Invalid JWT format: expected 3 parts, got {}", parts.len());
        return Err(AuthError::InvalidToken);
    }

    let payload_b64 = parts[1];
    info!("Payload base64 length: {}", payload_b64.len());

    let payload_bytes = general_purpose::URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|e| {
            error!("Base64 decode error: {}", e);
            AuthError::InvalidToken
        })?;

    info!("Decoded payload bytes length: {}", payload_bytes.len());
    let payload_str = String::from_utf8_lossy(&payload_bytes);
    info!("Payload JSON: {}", payload_str);

    let claims: SupabaseClaims = serde_json::from_slice(&payload_bytes).map_err(|e| {
        error!("JSON parsing error: {}", e);
        AuthError::InvalidToken
    })?;

    info!("Successfully parsed claims for user: {}", claims.sub);
    Ok(claims)
}

/// Extract JWT token from request headers
fn extract_token_from_request(req: &HttpRequest) -> Option<String> {
    let auth_header = req.headers().get("authorization");
    info!("Authorization header present: {}", auth_header.is_some());

    if let Some(header_value) = auth_header {
        let header_str = header_value.to_str().ok()?;
        info!("Authorization header value: '{}'", header_str);

        if let Some(token) = header_str.strip_prefix("Bearer ") {
            info!("Token extracted, length: {}", token.len());
            info!("Token first 20 chars: {}", &token[..token.len().min(20)]);
            Some(token.to_string())
        } else {
            error!("Authorization header doesn't start with 'Bearer '");
            None
        }
    } else {
        error!("No authorization header found");
        None
    }
}

/// Extract and validate auth from request
async fn get_authenticated_user(
    req: &HttpRequest,
    supabase_config: &SupabaseConfig,
) -> Result<SupabaseClaims, actix_web::Error> {
    let token = extract_token_from_request(req)
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("Missing authorization token"))?;

    // Parse claims first (quick check)
    let claims = parse_jwt_claims(&token)
        .map_err(|_| actix_web::error::ErrorUnauthorized("Invalid token format"))?;

    // Validate with Supabase
    validate_supabase_jwt_token(&token, supabase_config)
        .await
        .map_err(|e| {
            error!("JWT validation failed: {}", e);
            actix_web::error::ErrorUnauthorized("Invalid or expired authentication token")
        })?;

    Ok(claims)
}

/// Get user's database connection with authentication
async fn get_user_db_connection(
    req: &HttpRequest,
    turso_client: &Arc<TursoClient>,
    supabase_config: &SupabaseConfig,
) -> Result<libsql::Connection, actix_web::Error> {
    let claims = get_authenticated_user(req, supabase_config).await?;

    match turso_client.get_user_database_connection(&claims.sub).await {
        Ok(Some(conn)) => Ok(conn),
        Ok(None) => Err(actix_web::error::ErrorNotFound("User database not found")),
        Err(e) => {
            error!("Error getting user database connection: {}", e);
            Err(actix_web::error::ErrorInternalServerError(
                "Database access error",
            ))
        }
    }
}

// CRUD Route Handlers

// Create a new stock trade with cache invalidation - DEPRECATED
/*
pub async fn create_stock(
    req: HttpRequest,
    payload: web::Json<CreateStockRequest>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
    cache_service: web::Data<Arc<CacheService>>,
    vectorization_service: web::Data<Arc<VectorizationService>>,
    ws_manager: web::Data<Arc<Mutex<ConnectionManager>>>,
) -> Result<HttpResponse> {
    info!("Creating new stock trade");

    let conn = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let user_id = get_authenticated_user(&req, &supabase_config).await?.sub;

    match Stock::create(&conn, payload.into_inner()).await {
        Ok(stock) => {
            info!("Successfully created stock with ID: {}", stock.id);

            // Invalidate cache after successful creation
            let cache_service_clone = cache_service.get_ref().clone();
            let user_id_clone = user_id.clone();

            tokio::spawn(async move {
                match cache_service_clone.invalidate_table_cache(&user_id_clone, "stocks").await {
                    Ok(count) => info!("Invalidated {} stock cache keys for user: {}", count, user_id_clone),
                    Err(e) => error!("Failed to invalidate stock cache for user {}: {}", user_id_clone, e),
                }

                // Also invalidate analytics cache
                match cache_service_clone.invalidate_user_analytics(&user_id_clone).await {
                    Ok(count) => info!("Invalidated {} analytics cache keys for user: {}", count, user_id_clone),
                    Err(e) => error!("Failed to invalidate analytics cache for user {}: {}", user_id_clone, e),
                }
            });

            // Broadcast real-time create
            let ws_manager_clone = ws_manager.clone();
            let user_id_ws = user_id.clone();
            let stock_ws = stock.clone();
            tokio::spawn(async move {
                broadcast_stock_update(ws_manager_clone, &user_id_ws, "created", &stock_ws).await;
            });

            // Vectorization removed - trades are vectorized via TradeVectorService for mistakes/notes only

            Ok(HttpResponse::Created().json(ApiResponse::success(stock)))
        }
        Err(e) => {
            error!("Failed to create stock: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                ApiResponse::<()>::error("Failed to create stock trade")
            ))
        }
    }
}
*/

pub async fn create_stock(
    req: HttpRequest,
    body: web::Bytes, // Changed from web::Json to get raw bytes
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    cache_service: web::Data<Arc<CacheService>>,
    trade_vectorization: web::Data<Arc<crate::service::ai_service::TradeVectorization>>,
    ws_manager: web::Data<Arc<Mutex<ConnectionManager>>>,
) -> Result<HttpResponse> {
    // Log raw request body
    info!("📦 Raw request body: {}", String::from_utf8_lossy(&body));

    // Try to deserialize manually and log any errors
    let payload: CreateStockRequest = match serde_json::from_slice(&body) {
        Ok(p) => {
            info!("✅ Successfully deserialized payload: {:?}", p);
            p
        }
        Err(e) => {
            error!("❌ Deserialization error: {}", e);
            error!("❌ Error at line: {}, column: {}", e.line(), e.column());
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "error": format!("Invalid request format: {}", e),
                "line": e.line(),
                "column": e.column()
            })));
        }
    };

    info!("Creating new stock trade");

    let user_id = get_authenticated_user(&req, &supabase_config).await?.sub;
    let conn = get_user_db_connection(&req, &app_state.turso_client, &supabase_config).await?;

    // Check storage quota before creating
    app_state
        .storage_quota_service
        .check_storage_quota(&user_id, &conn)
        .await
        .map_err(|e| {
            error!("Storage quota check failed for user {}: {}", user_id, e);
            e
        })?;

    match Stock::create(&conn, payload).await {
        Ok(stock) => {
            info!("Successfully created stock with ID: {}", stock.id);

            // Invalidate cache after successful creation
            let cache_service_clone = cache_service.get_ref().clone();
            let user_id_clone = user_id.clone();

            tokio::spawn(async move {
                match cache_service_clone
                    .invalidate_table_cache(&user_id_clone, "stocks")
                    .await
                {
                    Ok(count) => info!(
                        "Invalidated {} stock cache keys for user: {}",
                        count, user_id_clone
                    ),
                    Err(e) => error!(
                        "Failed to invalidate stock cache for user {}: {}",
                        user_id_clone, e
                    ),
                }

                // Also invalidate analytics cache
                match cache_service_clone
                    .invalidate_user_analytics(&user_id_clone)
                    .await
                {
                    Ok(count) => info!(
                        "Invalidated {} analytics cache keys for user: {}",
                        count, user_id_clone
                    ),
                    Err(e) => error!(
                        "Failed to invalidate analytics cache for user {}: {}",
                        user_id_clone, e
                    ),
                }
            });

            // Broadcast real-time create
            let ws_manager_clone = ws_manager.clone();
            let user_id_ws = user_id.clone();
            let stock_ws = stock.clone();
            tokio::spawn(async move {
                broadcast_stock_update(ws_manager_clone, &user_id_ws, "created", &stock_ws).await;
            });

            if let Some(ref mistakes) = stock.mistakes && !mistakes.trim().is_empty() {
                let trade_vectorization_clone = trade_vectorization.get_ref().clone();
                let stock_id = stock.id;
                let user_id_vec = user_id.clone();
                let conn_clone = conn.clone();

                tokio::spawn(async move {
                    if let Err(e) = trade_vectorization_clone
                        .vectorize_trade(&user_id_vec, stock_id, "stock", &conn_clone)
                        .await
                    {
                        error!("Failed to vectorize trade for stock {}: {}", stock_id, e);
                    } else {
                        info!("Successfully vectorized trade for stock {}", stock_id);
                    }
                });
            }

            Ok(HttpResponse::Created().json(ApiResponse::success(stock)))
        }
        Err(e) => {
            error!("Failed to create stock: {}", e);
            Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to create stock trade")))
        }
    }
}

/// Get stock by ID with caching
pub async fn get_stock_by_id(
    req: HttpRequest,
    stock_id: web::Path<i64>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
    cache_service: web::Data<Arc<CacheService>>,
) -> Result<HttpResponse> {
    let id = stock_id.into_inner();
    info!("Fetching stock with ID: {}", id);

    let conn = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let user_id = get_authenticated_user(&req, &supabase_config).await?.sub;

    // Generate cache key for individual stock
    let cache_key = format!("db:{}:stocks:item:{}", user_id, id);

    match cache_service
        .get_or_fetch(&cache_key, 1800, || async {
            info!("Cache miss for stock ID: {}, fetching from database", id);
            Stock::find_by_id(&conn, id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))
        })
        .await
    {
        Ok(Some(stock)) => {
            info!("Found stock with ID: {} (cached)", id);
            Ok(HttpResponse::Ok().json(ApiResponse::success(stock)))
        }
        Ok(None) => {
            info!("Stock with ID {} not found", id);
            Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error("Stock not found")))
        }
        Err(e) => {
            error!("Failed to fetch stock {}: {}", id, e);
            Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to fetch stock")))
        }
    }
}

/// Get all stocks with optional filtering and caching
pub async fn get_all_stocks(
    req: HttpRequest,
    query: web::Query<StockQuery>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
    cache_service: web::Data<Arc<CacheService>>,
) -> Result<HttpResponse> {
    info!(
        "get_all_stocks: Received request with query params: {:?}",
        query
    );

    // Log: start getting DB connection
    info!("get_all_stocks: Attempting to get user DB connection");
    let conn = match get_user_db_connection(&req, &turso_client, &supabase_config).await {
        Ok(conn) => {
            info!("get_all_stocks: Acquired user DB connection successfully.");
            conn
        }
        Err(e) => {
            error!("get_all_stocks: Failed to get DB connection: {}", e);
            return Err(e);
        }
    };

    // Log: try get user id
    info!("get_all_stocks: Attempting to authenticate user.");
    let user_id = match get_authenticated_user(&req, &supabase_config).await {
        Ok(user) => {
            info!("get_all_stocks: Authenticated user with id: {}", user.sub);
            user.sub
        }
        Err(e) => {
            error!("get_all_stocks: User authentication failed: {}", e);
            return Err(e);
        }
    };

    let stock_query = query.into_inner();
    info!("get_all_stocks: Stock query to be used: {:?}", stock_query);

    // Check if we need simplified response for open trades
    if stock_query.open_only == Some(true) {
        info!("get_all_stocks: Fetching open trades with simplified response");
        // Generate cache key based on query parameters
        let query_hash = format!("{:?}", stock_query);
        let cache_key = format!("db:{}:stocks:open:summary:{}", user_id, query_hash);
        info!("get_all_stocks: Using cache key: {}", cache_key);

        match cache_service.get_or_fetch(&cache_key, 1800, || async {
            info!(
                "get_all_stocks: Cache miss for open stocks summary; fetching from DB for user {} with query {:?}",
                user_id, stock_query
            );
            match Stock::find_all_open_summary(&conn, stock_query).await {
                Ok(open_trades) => {
                    info!(
                        "get_all_stocks: Successfully fetched {} open stocks from DB",
                        open_trades.len()
                    );
                    Ok(open_trades)
                },
                Err(e) => {
                    error!("get_all_stocks: DB error when fetching open stocks: {}", e);
                    Err(anyhow::anyhow!("{}", e))
                }
            }
        }).await {
            Ok(open_trades) => {
                info!("get_all_stocks: Returning {} open stocks to client (may be cached)", open_trades.len());
                Ok(HttpResponse::Ok().json(ApiResponse::success(open_trades)))
            }
            Err(e) => {
                error!("get_all_stocks: Failed to fetch open stocks: {}", e);
                Ok(HttpResponse::InternalServerError().json(
                    ApiResponse::<()>::error("Failed to fetch open stocks")
                ))
            }
        }
    } else {
        // Generate cache key based on query parameters
        let query_hash = format!("{:?}", stock_query);
        let cache_key = format!("db:{}:stocks:list:{}", user_id, query_hash);
        info!("get_all_stocks: Using cache key: {}", cache_key);

        // Try using cache
        info!("get_all_stocks: Checking cache for stocks list");
        match cache_service.get_or_fetch(&cache_key, 1800, || async {
            info!(
                "get_all_stocks: Cache miss for stocks list; fetching from DB for user {} with query {:?}",
                user_id, stock_query
            );
            match Stock::find_all(&conn, stock_query).await {
                Ok(stocks) => {
                    info!(
                        "get_all_stocks: Successfully fetched {} stocks from DB",
                        stocks.len()
                    );
                    Ok(stocks)
                },
                Err(e) => {
                    error!("get_all_stocks: DB error when fetching stocks: {}", e);
                    Err(anyhow::anyhow!("{}", e))
                }
            }
        }).await {
            Ok(stocks) => {
                info!("get_all_stocks: Returning {} stocks to client (may be cached)", stocks.len());
                Ok(HttpResponse::Ok().json(ApiResponse::success(stocks)))
            }
            Err(e) => {
                error!("get_all_stocks: Failed to fetch stocks: {}", e);
                Ok(HttpResponse::InternalServerError().json(
                    ApiResponse::<()>::error("Failed to fetch stocks")
                ))
            }
        }
    }
}

/// Update a stock trade with cache invalidation
#[allow(clippy::too_many_arguments)]
pub async fn update_stock(
    req: HttpRequest,
    stock_id: web::Path<i64>,
    body: web::Bytes, // Changed from web::Json to get raw bytes for logging
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
    cache_service: web::Data<Arc<CacheService>>,
    trade_vector_service: web::Data<Arc<TradeVectorService>>,
    ws_manager: web::Data<Arc<Mutex<ConnectionManager>>>,
) -> Result<HttpResponse> {
    let id = stock_id.into_inner();
    info!("🔄 [UPDATE_STOCK] Starting update for stock ID: {}", id);

    // Log raw request body
    let body_str = String::from_utf8_lossy(&body);
    info!("📦 [UPDATE_STOCK] Raw request body: {}", body_str);

    // Try to deserialize manually and log any errors
    let payload: UpdateStockRequest = match serde_json::from_slice(&body) {
        Ok(p) => {
            info!(
                "✅ [UPDATE_STOCK] Successfully deserialized payload: {:?}",
                p
            );
            p
        }
        Err(e) => {
            error!("❌ [UPDATE_STOCK] Deserialization error: {}", e);
            error!(
                "❌ [UPDATE_STOCK] Error at line: {}, column: {}",
                e.line(),
                e.column()
            );
            error!("❌ [UPDATE_STOCK] Error path: {}", e);
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "error": format!("Invalid request format: {}", e),
                "line": e.line(),
                "column": e.column(),
                "path": e.to_string()
            })));
        }
    };

    info!("🔐 [UPDATE_STOCK] Getting database connection for user");
    let conn = match get_user_db_connection(&req, &turso_client, &supabase_config).await {
        Ok(c) => {
            info!("✅ [UPDATE_STOCK] Successfully obtained database connection");
            c
        }
        Err(e) => {
            error!("❌ [UPDATE_STOCK] Failed to get database connection: {}", e);
            return Err(e);
        }
    };

    info!("👤 [UPDATE_STOCK] Getting authenticated user");
    let user_id = match get_authenticated_user(&req, &supabase_config).await {
        Ok(claims) => {
            info!("✅ [UPDATE_STOCK] Authenticated user: {}", claims.sub);
            claims.sub
        }
        Err(e) => {
            error!("❌ [UPDATE_STOCK] Authentication failed: {}", e);
            return Err(e);
        }
    };

    info!(
        "💾 [UPDATE_STOCK] Calling Stock::update with payload: {:?}",
        payload
    );
    match Stock::update(&conn, id, payload).await {
        Ok(Some(stock)) => {
            info!(
                "✅ [UPDATE_STOCK] Successfully updated stock with ID: {}",
                id
            );
            info!("✅ [UPDATE_STOCK] Updated stock data: {:?}", stock);

            // Invalidate cache after successful update
            let cache_service_clone = cache_service.get_ref().clone();
            let user_id_clone = user_id.clone();

            tokio::spawn(async move {
                match cache_service_clone
                    .invalidate_table_cache(&user_id_clone, "stocks")
                    .await
                {
                    Ok(count) => info!(
                        "Invalidated {} stock cache keys for user: {}",
                        count, user_id_clone
                    ),
                    Err(e) => error!(
                        "Failed to invalidate stock cache for user {}: {}",
                        user_id_clone, e
                    ),
                }

                // Also invalidate analytics cache
                match cache_service_clone
                    .invalidate_user_analytics(&user_id_clone)
                    .await
                {
                    Ok(count) => info!(
                        "Invalidated {} analytics cache keys for user: {}",
                        count, user_id_clone
                    ),
                    Err(e) => error!(
                        "Failed to invalidate analytics cache for user {}: {}",
                        user_id_clone, e
                    ),
                }
            });

            // Broadcast real-time update (non-blocking, errors are logged but don't affect response)
            let ws_manager_clone = ws_manager.get_ref().clone();
            let user_id_ws = user_id.clone();
            let stock_ws = stock.clone();
            tokio::spawn(async move {
                // Use broadcast_to_user directly since we don't have Data wrapper in spawned task
                use crate::websocket::{EventType, WsMessage};
                let envelope = WsMessage::new(
                    EventType::StockUpdated,
                    serde_json::to_value(&stock_ws).unwrap_or(serde_json::Value::Null),
                );
                let manager = ws_manager_clone.lock().await;
                manager.broadcast_to_user(&user_id_ws, envelope);
            });

            // Re-vectorize trade mistakes and notes
            let trade_vector_service_clone = trade_vector_service.get_ref().clone();
            let stock_id = stock.id;
            let user_id_vec = user_id.clone();
            let conn_clone = conn.clone();

            tokio::spawn(async move {
                if let Err(e) = trade_vector_service_clone
                    .vectorize_trade_mistakes_and_notes(
                        &user_id_vec,
                        stock_id,
                        "stock",
                        &conn_clone,
                    )
                    .await
                {
                    error!(
                        "Failed to re-vectorize trade mistakes and notes for stock {}: {}",
                        stock_id, e
                    );
                } else {
                    info!(
                        "Successfully re-vectorized mistakes and notes for stock {}",
                        stock_id
                    );
                }
            });

            Ok(HttpResponse::Ok().json(ApiResponse::success(stock)))
        }
        Ok(None) => {
            warn!(
                "⚠️ [UPDATE_STOCK] Stock with ID {} not found for update",
                id
            );
            Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error("Stock not found")))
        }
        Err(e) => {
            error!("❌ [UPDATE_STOCK] Failed to update stock {}: {}", id, e);
            error!("❌ [UPDATE_STOCK] Error details: {:?}", e);
            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(&format!(
                    "Failed to update stock: {}",
                    e
                ))),
            )
        }
    }
}

/// Delete a stock trade with cache invalidation
pub async fn delete_stock(
    req: HttpRequest,
    stock_id: web::Path<i64>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
    cache_service: web::Data<Arc<CacheService>>,
    trade_vector_service: web::Data<Arc<TradeVectorService>>,
    ws_manager: web::Data<Arc<Mutex<ConnectionManager>>>,
) -> Result<HttpResponse> {
    let id = stock_id.into_inner();
    info!("Deleting stock with ID: {}", id);

    let conn = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let user_id = get_authenticated_user(&req, &supabase_config).await?.sub;

    match Stock::delete(&conn, id).await {
        Ok(true) => {
            info!("Successfully deleted stock with ID: {}", id);

            // Invalidate cache after successful deletion
            let cache_service_clone = cache_service.get_ref().clone();
            let user_id_clone = user_id.clone();

            tokio::spawn(async move {
                match cache_service_clone
                    .invalidate_table_cache(&user_id_clone, "stocks")
                    .await
                {
                    Ok(count) => info!(
                        "Invalidated {} stock cache keys for user: {}",
                        count, user_id_clone
                    ),
                    Err(e) => error!(
                        "Failed to invalidate stock cache for user {}: {}",
                        user_id_clone, e
                    ),
                }

                // Also invalidate analytics cache
                match cache_service_clone
                    .invalidate_user_analytics(&user_id_clone)
                    .await
                {
                    Ok(count) => info!(
                        "Invalidated {} analytics cache keys for user: {}",
                        count, user_id_clone
                    ),
                    Err(e) => error!(
                        "Failed to invalidate analytics cache for user {}: {}",
                        user_id_clone, e
                    ),
                }
            });

            // Delete trade vector
            let trade_vector_service_clone = trade_vector_service.get_ref().clone();
            let stock_id = id;
            let user_id_vec = user_id.clone();

            tokio::spawn(async move {
                if let Err(e) = trade_vector_service_clone
                    .delete_trade_vector(&user_id_vec, stock_id)
                    .await
                {
                    error!(
                        "Failed to delete trade vector for stock {}: {}",
                        stock_id, e
                    );
                } else {
                    info!("Successfully deleted trade vector for stock {}", stock_id);
                }
            });

            // Broadcast deletion event
            let ws_manager_clone = ws_manager.clone();
            let user_id_ws = user_id.clone();
            tokio::spawn(async move {
                broadcast_stock_update(
                    ws_manager_clone,
                    &user_id_ws,
                    "deleted",
                    serde_json::json!({"id": id}),
                )
                .await;
            });

            Ok(
                HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
                    "deleted": true,
                    "id": id
                }))),
            )
        }
        Ok(false) => {
            info!("Stock with ID {} not found for deletion", id);
            Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error("Stock not found")))
        }
        Err(e) => {
            error!("Failed to delete stock {}: {}", id, e);
            Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to delete stock")))
        }
    }
}

/// Get total count of stocks for pagination with caching
pub async fn get_stocks_count(
    req: HttpRequest,
    query: web::Query<StockQuery>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
    cache_service: web::Data<Arc<CacheService>>,
) -> Result<HttpResponse> {
    info!("Getting stocks count");

    let conn = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let user_id = get_authenticated_user(&req, &supabase_config).await?.sub;
    let stock_query = query.into_inner();

    // Generate cache key for count
    let query_hash = format!("{:?}", stock_query);
    let cache_key = format!("db:{}:stocks:count:{}", user_id, query_hash);

    match cache_service
        .get_or_fetch(&cache_key, 1800, || async {
            info!("Cache miss for stocks count, fetching from database");
            Stock::count(&conn, &stock_query)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))
        })
        .await
    {
        Ok(count) => {
            info!("Total stocks count: {} (cached)", count);
            Ok(
                HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
                    "count": count
                }))),
            )
        }
        Err(e) => {
            error!("Failed to get stocks count: {}", e);
            Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to get stocks count")))
        }
    }
}

// Analytics Route Handlers

/// Get comprehensive stocks analytics with caching
pub async fn get_stocks_analytics(
    req: HttpRequest,
    query: web::Query<TimeRangeQuery>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
    cache_service: web::Data<Arc<CacheService>>,
) -> Result<HttpResponse> {
    info!("Generating stocks analytics");
    // Attempt to get user DB connection; if not found, return empty analytics instead of 404
    let conn = match get_user_db_connection(&req, &turso_client, &supabase_config).await {
        Ok(c) => c,
        Err(e) => {
            if e.as_response_error().status_code() == actix_web::http::StatusCode::NOT_FOUND {
                let time_range = query.time_range.clone().unwrap_or(TimeRange::AllTime);
                let user_id = match get_authenticated_user(&req, &supabase_config).await {
                    Ok(claims) => claims.sub,
                    Err(_) => "unknown".to_string(),
                };
                let cache_key = format!("analytics:db:{}:stocks:{:?}", user_id, time_range);
                // Return zeroed analytics (and do not cache)
                let analytics = StocksAnalytics {
                    total_pnl: "0".to_string(),
                    profit_factor: "0".to_string(),
                    win_rate: "0".to_string(),
                    loss_rate: "0".to_string(),
                    avg_gain: "0".to_string(),
                    avg_loss: "0".to_string(),
                    biggest_winner: "0".to_string(),
                    biggest_loser: "0".to_string(),
                    avg_hold_time_winners: "0".to_string(),
                    avg_hold_time_losers: "0".to_string(),
                    risk_reward_ratio: "0".to_string(),
                    trade_expectancy: "0".to_string(),
                    avg_position_size: "0".to_string(),
                    net_pnl: "0".to_string(),
                };
                info!(
                    "User DB not found; returning default stocks analytics for cache key {}",
                    cache_key
                );
                return Ok(HttpResponse::Ok().json(ApiResponse::success(analytics)));
            }
            return Err(e);
        }
    };
    let time_range = query.time_range.clone().unwrap_or(TimeRange::AllTime);
    let user_id = get_authenticated_user(&req, &supabase_config).await?.sub;

    // Generate cache key for this analytics request
    let cache_key = format!("analytics:db:{}:stocks:{:?}", user_id, time_range);

    // Try to get from cache first
    match cache_service
        .get_or_fetch(&cache_key, 900, || async {
            info!("Cache miss for stocks analytics, calculating from database");

            // Collect all analytics in parallel for better performance
            let total_pnl = Stock::calculate_total_pnl(&conn)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            let profit_factor = Stock::calculate_profit_factor(&conn, time_range.clone())
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            let win_rate = Stock::calculate_win_rate(&conn, time_range.clone())
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            let loss_rate = Stock::calculate_loss_rate(&conn, time_range.clone())
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            let avg_gain = Stock::calculate_avg_gain(&conn, time_range.clone())
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            let avg_loss = Stock::calculate_avg_loss(&conn, time_range.clone())
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            let biggest_winner = Stock::calculate_biggest_winner(&conn, time_range.clone())
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            let biggest_loser = Stock::calculate_biggest_loser(&conn, time_range.clone())
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            let avg_hold_time_winners =
                Stock::calculate_avg_hold_time_winners(&conn, time_range.clone())
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
            let avg_hold_time_losers =
                Stock::calculate_avg_hold_time_losers(&conn, time_range.clone())
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
            let risk_reward_ratio = Stock::calculate_risk_reward_ratio(&conn, time_range.clone())
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            let trade_expectancy = Stock::calculate_trade_expectancy(&conn, time_range.clone())
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            let avg_position_size = Stock::calculate_avg_position_size(&conn, time_range.clone())
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            let net_pnl = Stock::calculate_net_pnl(&conn, time_range)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            Ok(StocksAnalytics {
                total_pnl: total_pnl.to_string(),
                profit_factor: profit_factor.to_string(),
                win_rate: win_rate.to_string(),
                loss_rate: loss_rate.to_string(),
                avg_gain: avg_gain.to_string(),
                avg_loss: avg_loss.to_string(),
                biggest_winner: biggest_winner.to_string(),
                biggest_loser: biggest_loser.to_string(),
                avg_hold_time_winners: avg_hold_time_winners.to_string(),
                avg_hold_time_losers: avg_hold_time_losers.to_string(),
                risk_reward_ratio: risk_reward_ratio.to_string(),
                trade_expectancy: trade_expectancy.to_string(),
                avg_position_size: avg_position_size.to_string(),
                net_pnl: net_pnl.to_string(),
            })
        })
        .await
    {
        Ok(analytics) => {
            info!("Generated comprehensive analytics (cached)");
            Ok(HttpResponse::Ok().json(ApiResponse::success(analytics)))
        }
        Err(e) => {
            error!("Failed to generate analytics: {}", e);
            Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to generate analytics")))
        }
    }
}

/// Get total P&L with caching
pub async fn get_total_pnl(
    req: HttpRequest,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
    cache_service: web::Data<Arc<CacheService>>,
) -> Result<HttpResponse> {
    info!("Calculating total P&L");

    let conn = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let user_id = get_authenticated_user(&req, &supabase_config).await?.sub;

    // Generate cache key for total PnL
    let cache_key = format!("analytics:db:{}:stocks:total_pnl", user_id);

    match cache_service
        .get_or_fetch(&cache_key, 1800, || async {
            info!("Cache miss for total P&L, calculating from database");
            Stock::calculate_total_pnl(&conn)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))
        })
        .await
    {
        Ok(pnl) => {
            info!("Total P&L: {} (cached)", pnl);
            Ok(
                HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
                    "total_pnl": pnl.to_string()
                }))),
            )
        }
        Err(e) => {
            error!("Failed to calculate total P&L: {}", e);
            Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to calculate total P&L")))
        }
    }
}

/// Get profit factor with caching
pub async fn get_profit_factor(
    req: HttpRequest,
    query: web::Query<TimeRangeQuery>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
    cache_service: web::Data<Arc<CacheService>>,
) -> Result<HttpResponse> {
    info!("Calculating profit factor");

    let conn = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let time_range = query.time_range.clone().unwrap_or(TimeRange::AllTime);
    let user_id = get_authenticated_user(&req, &supabase_config).await?.sub;

    // Generate cache key for profit factor
    let cache_key = format!(
        "analytics:db:{}:stocks:profit_factor:{:?}",
        user_id, time_range
    );

    match cache_service
        .get_or_fetch(&cache_key, 900, || async {
            info!("Cache miss for profit factor, calculating from database");
            Stock::calculate_profit_factor(&conn, time_range)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))
        })
        .await
    {
        Ok(factor) => {
            info!("Profit factor: {} (cached)", factor);
            Ok(
                HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
                    "profit_factor": factor.to_string()
                }))),
            )
        }
        Err(e) => {
            error!("Failed to calculate profit factor: {}", e);
            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to calculate profit factor",
                )),
            )
        }
    }
}

/// Get win rate
pub async fn get_win_rate(
    req: HttpRequest,
    query: web::Query<TimeRangeQuery>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> Result<HttpResponse> {
    info!("Calculating win rate");

    let conn = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let time_range = query.time_range.clone().unwrap_or(TimeRange::AllTime);

    match Stock::calculate_win_rate(&conn, time_range).await {
        Ok(rate) => {
            info!("Win rate: {}%", rate);
            Ok(
                HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
                    "win_rate": rate.to_string()
                }))),
            )
        }
        Err(e) => {
            error!("Failed to calculate win rate: {}", e);
            Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to calculate win rate")))
        }
    }
}

/// Get loss rate
pub async fn get_loss_rate(
    req: HttpRequest,
    query: web::Query<TimeRangeQuery>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> Result<HttpResponse> {
    info!("Calculating loss rate");

    let conn = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let time_range = query.time_range.clone().unwrap_or(TimeRange::AllTime);

    match Stock::calculate_loss_rate(&conn, time_range).await {
        Ok(rate) => {
            info!("Loss rate: {}%", rate);
            Ok(
                HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
                    "loss_rate": rate.to_string()
                }))),
            )
        }
        Err(e) => {
            error!("Failed to calculate loss rate: {}", e);
            Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to calculate loss rate")))
        }
    }
}

/// Get average gain
pub async fn get_avg_gain(
    req: HttpRequest,
    query: web::Query<TimeRangeQuery>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> Result<HttpResponse> {
    info!("Calculating average gain");

    let conn = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let time_range = query.time_range.clone().unwrap_or(TimeRange::AllTime);

    match Stock::calculate_avg_gain(&conn, time_range).await {
        Ok(gain) => {
            info!("Average gain: {}", gain);
            Ok(
                HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
                    "avg_gain": gain.to_string()
                }))),
            )
        }
        Err(e) => {
            error!("Failed to calculate average gain: {}", e);
            Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to calculate average gain")))
        }
    }
}

/// Get average loss
pub async fn get_avg_loss(
    req: HttpRequest,
    query: web::Query<TimeRangeQuery>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> Result<HttpResponse> {
    info!("Calculating average loss");

    let conn = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let time_range = query.time_range.clone().unwrap_or(TimeRange::AllTime);

    match Stock::calculate_avg_loss(&conn, time_range).await {
        Ok(loss) => {
            info!("Average loss: {}", loss);
            Ok(
                HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
                    "avg_loss": loss.to_string()
                }))),
            )
        }
        Err(e) => {
            error!("Failed to calculate average loss: {}", e);
            Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to calculate average loss")))
        }
    }
}

/// Get biggest winner
pub async fn get_biggest_winner(
    req: HttpRequest,
    query: web::Query<TimeRangeQuery>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> Result<HttpResponse> {
    info!("Calculating biggest winner");

    let conn = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let time_range = query.time_range.clone().unwrap_or(TimeRange::AllTime);

    match Stock::calculate_biggest_winner(&conn, time_range).await {
        Ok(winner) => {
            info!("Biggest winner: {}", winner);
            Ok(
                HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
                    "biggest_winner": winner.to_string()
                }))),
            )
        }
        Err(e) => {
            error!("Failed to calculate biggest winner: {}", e);
            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to calculate biggest winner",
                )),
            )
        }
    }
}

/// Get biggest loser
pub async fn get_biggest_loser(
    req: HttpRequest,
    query: web::Query<TimeRangeQuery>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> Result<HttpResponse> {
    info!("Calculating biggest loser");

    let conn = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let time_range = query.time_range.clone().unwrap_or(TimeRange::AllTime);

    match Stock::calculate_biggest_loser(&conn, time_range).await {
        Ok(loser) => {
            info!("Biggest loser: {}", loser);
            Ok(
                HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
                    "biggest_loser": loser.to_string()
                }))),
            )
        }
        Err(e) => {
            error!("Failed to calculate biggest loser: {}", e);
            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to calculate biggest loser",
                )),
            )
        }
    }
}

/// Get average hold time for winners
pub async fn get_avg_hold_time_winners(
    req: HttpRequest,
    query: web::Query<TimeRangeQuery>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> Result<HttpResponse> {
    info!("Calculating average hold time for winners");

    let conn = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let time_range = query.time_range.clone().unwrap_or(TimeRange::AllTime);

    match Stock::calculate_avg_hold_time_winners(&conn, time_range).await {
        Ok(hold_time) => {
            info!("Average hold time for winners: {}", hold_time);
            Ok(
                HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
                    "avg_hold_time_winners": hold_time.to_string()
                }))),
            )
        }
        Err(e) => {
            error!("Failed to calculate average hold time for winners: {}", e);
            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to calculate average hold time for winners",
                )),
            )
        }
    }
}

/// Get average hold time for losers
pub async fn get_avg_hold_time_losers(
    req: HttpRequest,
    query: web::Query<TimeRangeQuery>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> Result<HttpResponse> {
    info!("Calculating average hold time for losers");

    let conn = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let time_range = query.time_range.clone().unwrap_or(TimeRange::AllTime);

    match Stock::calculate_avg_hold_time_losers(&conn, time_range).await {
        Ok(hold_time) => {
            info!("Average hold time for losers: {}", hold_time);
            Ok(
                HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
                    "avg_hold_time_losers": hold_time.to_string()
                }))),
            )
        }
        Err(e) => {
            error!("Failed to calculate average hold time for losers: {}", e);
            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to calculate average hold time for losers",
                )),
            )
        }
    }
}

/// Get risk reward ratio
pub async fn get_risk_reward_ratio(
    req: HttpRequest,
    query: web::Query<TimeRangeQuery>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> Result<HttpResponse> {
    info!("Calculating risk reward ratio");

    let conn = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let time_range = query.time_range.clone().unwrap_or(TimeRange::AllTime);

    match Stock::calculate_risk_reward_ratio(&conn, time_range).await {
        Ok(ratio) => {
            info!("Risk reward ratio: {}", ratio);
            Ok(
                HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
                    "risk_reward_ratio": ratio.to_string()
                }))),
            )
        }
        Err(e) => {
            error!("Failed to calculate risk reward ratio: {}", e);
            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to calculate risk reward ratio",
                )),
            )
        }
    }
}

/// Get trade expectancy
pub async fn get_trade_expectancy(
    req: HttpRequest,
    query: web::Query<TimeRangeQuery>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> Result<HttpResponse> {
    info!("Calculating trade expectancy");

    let conn = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let time_range = query.time_range.clone().unwrap_or(TimeRange::AllTime);

    match Stock::calculate_trade_expectancy(&conn, time_range).await {
        Ok(expectancy) => {
            info!("Trade expectancy: {}", expectancy);
            Ok(
                HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
                    "trade_expectancy": expectancy.to_string()
                }))),
            )
        }
        Err(e) => {
            error!("Failed to calculate trade expectancy: {}", e);
            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to calculate trade expectancy",
                )),
            )
        }
    }
}

/// Get average position size
pub async fn get_avg_position_size(
    req: HttpRequest,
    query: web::Query<TimeRangeQuery>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> Result<HttpResponse> {
    info!("Calculating average position size");

    let conn = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let time_range = query.time_range.clone().unwrap_or(TimeRange::AllTime);

    match Stock::calculate_avg_position_size(&conn, time_range).await {
        Ok(size) => {
            info!("Average position size: {}", size);
            Ok(
                HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
                    "avg_position_size": size.to_string()
                }))),
            )
        }
        Err(e) => {
            error!("Failed to calculate average position size: {}", e);
            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to calculate average position size",
                )),
            )
        }
    }
}

/// Get net P&L
pub async fn get_net_pnl(
    req: HttpRequest,
    query: web::Query<TimeRangeQuery>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> Result<HttpResponse> {
    info!("Calculating net P&L");

    let conn = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let time_range = query.time_range.clone().unwrap_or(TimeRange::AllTime);

    match Stock::calculate_net_pnl(&conn, time_range).await {
        Ok(pnl) => {
            info!("Net P&L: {}", pnl);
            Ok(
                HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
                    "net_pnl": pnl.to_string()
                }))),
            )
        }
        Err(e) => {
            error!("Failed to calculate net P&L: {}", e);
            Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to calculate net P&L")))
        }
    }
}

/// Query parameter for time range
#[derive(Debug, Deserialize)]
pub struct TimeRangeQuery {
    pub time_range: Option<TimeRange>,
}

/// Test endpoint to verify stocks routes are working
async fn test_endpoint() -> Result<HttpResponse> {
    info!("Stocks test endpoint hit!");
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Stocks routes are working!",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": "1.0.0"
    })))
}

/// Configure stocks routes
pub fn configure_stocks_routes(cfg: &mut web::ServiceConfig) {
    info!("Setting up /api/stocks routes");
    cfg.service(
        web::scope("/api/stocks")
            // Test route
            .route("/test", web::get().to(test_endpoint))
            // CRUD operations
            .route("", web::post().to(create_stock)) // POST /api/stocks
            .route("", web::get().to(get_all_stocks)) // GET /api/stocks?filters
            .route("/count", web::get().to(get_stocks_count)) // GET /api/stocks/count
            .route("/{id}", web::get().to(get_stock_by_id)) // GET /api/stocks/{id}
            .route("/{id}", web::put().to(update_stock)) // PUT /api/stocks/{id}
            .route("/{id}", web::delete().to(delete_stock)) // DELETE /api/stocks/{id}
            // Analytics endpoints
            .route("/analytics", web::get().to(get_stocks_analytics)) // GET /api/stocks/analytics?time_range=
            .route("/analytics/pnl", web::get().to(get_total_pnl)) // GET /api/stocks/analytics/pnl
            .route("/analytics/profit-factor", web::get().to(get_profit_factor)) // GET /api/stocks/analytics/profit-factor?time_range=
            .route("/analytics/win-rate", web::get().to(get_win_rate)) // GET /api/stocks/analytics/win-rate?time_range=
            .route("/analytics/loss-rate", web::get().to(get_loss_rate)) // GET /api/stocks/analytics/loss-rate?time_range=
            .route("/analytics/avg-gain", web::get().to(get_avg_gain)) // GET /api/stocks/analytics/avg-gain?time_range=
            .route("/analytics/avg-loss", web::get().to(get_avg_loss)) // GET /api/stocks/analytics/avg-loss?time_range=
            .route(
                "/analytics/biggest-winner",
                web::get().to(get_biggest_winner),
            ) // GET /api/stocks/analytics/biggest-winner?time_range=
            .route("/analytics/biggest-loser", web::get().to(get_biggest_loser)) // GET /api/stocks/analytics/biggest-loser?time_range=
            .route(
                "/analytics/avg-hold-time-winners",
                web::get().to(get_avg_hold_time_winners),
            ) // GET /api/stocks/analytics/avg-hold-time-winners?time_range=
            .route(
                "/analytics/avg-hold-time-losers",
                web::get().to(get_avg_hold_time_losers),
            ) // GET /api/stocks/analytics/avg-hold-time-losers?time_range=
            .route(
                "/analytics/risk-reward-ratio",
                web::get().to(get_risk_reward_ratio),
            ) // GET /api/stocks/analytics/risk-reward-ratio?time_range=
            .route(
                "/analytics/trade-expectancy",
                web::get().to(get_trade_expectancy),
            ) // GET /api/stocks/analytics/trade-expectancy?time_range=
            .route(
                "/analytics/avg-position-size",
                web::get().to(get_avg_position_size),
            ) // GET /api/stocks/analytics/avg-position-size?time_range=
            .route("/analytics/net-pnl", web::get().to(get_net_pnl)), // GET /api/stocks/analytics/net-pnl?time_range=
    );
}
