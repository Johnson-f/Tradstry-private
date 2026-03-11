use crate::models::options::{CreateOptionRequest, OptionQuery, OptionTrade, UpdateOptionRequest};
use crate::models::stock::stocks::TimeRange;
use crate::service::ai_service::TradeVectorService;
use crate::service::caching::CacheService;
use crate::turso::auth::{AuthError, validate_supabase_jwt_token};
use crate::turso::config::{SupabaseClaims, SupabaseConfig};
use crate::turso::{AppState, client::TursoClient};
use crate::websocket::{ConnectionManager, broadcast_option_update};
use actix_web::{HttpRequest, HttpResponse, Result, web};
use log::{error, info};
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
#[derive(Debug, Serialize)]
pub struct OptionsAnalytics {
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

    let conn = turso_client
        .get_user_database_connection(&claims.sub)
        .await
        .map_err(|e| {
            error!("Failed to connect to user database: {}", e);
            actix_web::error::ErrorInternalServerError("Database connection failed")
        })?
        .ok_or_else(|| {
            error!("No database found for user: {}", claims.sub);
            actix_web::error::ErrorNotFound("User database not found")
        })?;

    Ok(conn)
}

// CRUD Route Handlers

/// Create a new option trade with cache invalidation
pub async fn create_option(
    req: HttpRequest,
    body: web::Bytes,
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    cache_service: web::Data<Arc<CacheService>>,
    trade_vectorization: web::Data<Arc<crate::service::ai_service::TradeVectorization>>,
    ws_manager: web::Data<Arc<Mutex<ConnectionManager>>>,
) -> Result<HttpResponse> {
    // Log raw request body
    info!("📦 Raw request body: {}", String::from_utf8_lossy(&body));

    // Try to deserialize manually and log any errors
    let payload: CreateOptionRequest = match serde_json::from_slice(&body) {
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

    info!("Creating new option trade");

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

    match OptionTrade::create(&conn, payload).await {
        Ok(option) => {
            info!("Successfully created option with ID: {}", option.id);

            // Invalidate cache after successful creation
            let cache_service_clone = cache_service.get_ref().clone();
            let user_id_clone = user_id.clone();

            tokio::spawn(async move {
                match cache_service_clone
                    .invalidate_table_cache(&user_id_clone, "options")
                    .await
                {
                    Ok(count) => info!(
                        "Invalidated {} option cache keys for user: {}",
                        count, user_id_clone
                    ),
                    Err(e) => error!(
                        "Failed to invalidate option cache for user {}: {}",
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
            let option_ws = option.clone();
            tokio::spawn(async move {
                broadcast_option_update(ws_manager_clone, &user_id_ws, "created", &option_ws).await;
            });

            // Vectorize trade if it has mistakes
            if let Some(ref mistakes) = option.mistakes && !mistakes.trim().is_empty() {
                    let trade_vectorization_clone = trade_vectorization.get_ref().clone();
                    let option_id = option.id;
                    let user_id_vec = user_id.clone();
                    let conn_clone = conn.clone();

                    tokio::spawn(async move {
                        if let Err(e) = trade_vectorization_clone
                            .vectorize_trade(&user_id_vec, option_id, "option", &conn_clone)
                            .await
                        {
                            error!("Failed to vectorize trade for option {}: {}", option_id, e);
                        } else {
                            info!("Successfully vectorized trade for option {}", option_id);
                        }
                    });
            }

            Ok(HttpResponse::Created().json(ApiResponse::success(option)))
        }
        Err(e) => {
            error!("Failed to create option: {}", e);
            Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to create option trade")))
        }
    }
}

/// Get option by ID
pub async fn get_option_by_id(
    req: HttpRequest,
    option_id: web::Path<i64>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> Result<HttpResponse> {
    let id = option_id.into_inner();
    info!("Fetching option with ID: {}", id);

    let conn = get_user_db_connection(&req, &turso_client, &supabase_config).await?;

    match OptionTrade::find_by_id(&conn, id).await {
        Ok(Some(option)) => {
            info!("Found option with ID: {}", id);
            Ok(HttpResponse::Ok().json(ApiResponse::success(option)))
        }
        Ok(None) => {
            info!("Option with ID {} not found", id);
            Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error("Option not found")))
        }
        Err(e) => {
            error!("Failed to fetch option {}: {}", id, e);
            Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to fetch option")))
        }
    }
}

/// Get all options with optional filtering and caching
pub async fn get_all_options(
    req: HttpRequest,
    query: web::Query<OptionQuery>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
    cache_service: web::Data<Arc<CacheService>>,
) -> Result<HttpResponse> {
    info!("Fetching options with query: {:?}", query);

    let conn = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let user_id = get_authenticated_user(&req, &supabase_config).await?.sub;
    let option_query = query.into_inner();

    // Check if we need simplified response for open trades
    if option_query.open_only == Some(true) {
        info!("get_all_options: Fetching open trades with simplified response");
        // Generate cache key based on query parameters
        let query_hash = format!("{:?}", option_query);
        let cache_key = format!("db:{}:options:open:summary:{}", user_id, query_hash);

        match cache_service
            .get_or_fetch(&cache_key, 1800, || async {
                info!("Cache miss for open options summary, fetching from database");
                OptionTrade::find_all_open_summary(&conn, option_query)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))
            })
            .await
        {
            Ok(open_trades) => {
                info!("Found {} open options (cached)", open_trades.len());
                Ok(HttpResponse::Ok().json(ApiResponse::success(open_trades)))
            }
            Err(e) => {
                error!("Failed to fetch open options: {}", e);
                Ok(HttpResponse::InternalServerError()
                    .json(ApiResponse::<()>::error("Failed to fetch open options")))
            }
        }
    } else {
        // Generate cache key based on query parameters
        let query_hash = format!("{:?}", option_query);
        let cache_key = format!("db:{}:options:list:{}", user_id, query_hash);

        match cache_service
            .get_or_fetch(&cache_key, 1800, || async {
                info!("Cache miss for options list, fetching from database");
                OptionTrade::find_all(&conn, option_query)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))
            })
            .await
        {
            Ok(options) => {
                info!("Found {} options (cached)", options.len());
                Ok(HttpResponse::Ok().json(ApiResponse::success(options)))
            }
            Err(e) => {
                error!("Failed to fetch options: {}", e);
                Ok(HttpResponse::InternalServerError()
                    .json(ApiResponse::<()>::error("Failed to fetch options")))
            }
        }
    }
}

/// Update an option trade
pub async fn update_option(
    req: HttpRequest,
    option_id: web::Path<i64>,
    payload: web::Json<UpdateOptionRequest>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
    trade_vector_service: web::Data<Arc<TradeVectorService>>,
    ws_manager: web::Data<Arc<Mutex<ConnectionManager>>>,
) -> Result<HttpResponse> {
    let id = option_id.into_inner();
    info!("Updating option with ID: {}", id);

    let conn = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let user_id = get_authenticated_user(&req, &supabase_config).await?.sub;

    match OptionTrade::update(&conn, id, payload.into_inner()).await {
        Ok(Some(option)) => {
            info!("Successfully updated option with ID: {}", id);
            // Broadcast real-time update
            let ws_manager_clone = ws_manager.clone();
            let user_id_ws = user_id.clone();
            let option_ws = option.clone();
            tokio::spawn(async move {
                broadcast_option_update(ws_manager_clone, &user_id_ws, "updated", &option_ws).await;
            });

            // Re-vectorize trade mistakes and notes
            let trade_vector_service_clone = trade_vector_service.get_ref().clone();
            let option_id = option.id;
            let user_id_vec = user_id.clone();
            let conn_clone = conn.clone();

            tokio::spawn(async move {
                if let Err(e) = trade_vector_service_clone
                    .vectorize_trade_mistakes_and_notes(
                        &user_id_vec,
                        option_id,
                        "option",
                        &conn_clone,
                    )
                    .await
                {
                    error!(
                        "Failed to re-vectorize trade mistakes and notes for option {}: {}",
                        option_id, e
                    );
                } else {
                    info!(
                        "Successfully re-vectorized mistakes and notes for option {}",
                        option_id
                    );
                }
            });

            Ok(HttpResponse::Ok().json(ApiResponse::success(option)))
        }
        Ok(None) => {
            info!("Option with ID {} not found for update", id);
            Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error("Option not found")))
        }
        Err(e) => {
            error!("Failed to update option {}: {}", id, e);
            Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to update option")))
        }
    }
}

/// Delete an option trade
pub async fn delete_option(
    req: HttpRequest,
    option_id: web::Path<i64>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
    trade_vector_service: web::Data<Arc<TradeVectorService>>,
    ws_manager: web::Data<Arc<Mutex<ConnectionManager>>>,
) -> Result<HttpResponse> {
    let id = option_id.into_inner();
    info!("Deleting option with ID: {}", id);

    let conn = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let user_id = get_authenticated_user(&req, &supabase_config).await?.sub;

    match OptionTrade::delete(&conn, id).await {
        Ok(true) => {
            info!("Successfully deleted option with ID: {}", id);

            // Delete trade vector
            let trade_vector_service_clone = trade_vector_service.get_ref().clone();
            let option_id = id;
            let user_id_vec = user_id.clone();

            tokio::spawn(async move {
                if let Err(e) = trade_vector_service_clone
                    .delete_trade_vector(&user_id_vec, option_id)
                    .await
                {
                    error!(
                        "Failed to delete trade vector for option {}: {}",
                        option_id, e
                    );
                } else {
                    info!("Successfully deleted trade vector for option {}", option_id);
                }
            });

            // Broadcast deletion
            let ws_manager_clone = ws_manager.clone();
            let user_id_ws = user_id.clone();
            tokio::spawn(async move {
                broadcast_option_update(
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
            info!("Option with ID {} not found for deletion", id);
            Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error("Option not found")))
        }
        Err(e) => {
            error!("Failed to delete option {}: {}", id, e);
            Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to delete option")))
        }
    }
}

/// Get total count of options for pagination
pub async fn get_options_count(
    req: HttpRequest,
    query: web::Query<OptionQuery>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> Result<HttpResponse> {
    info!("Getting options count");

    let conn = get_user_db_connection(&req, &turso_client, &supabase_config).await?;

    match OptionTrade::count(&conn, &query.into_inner()).await {
        Ok(count) => {
            info!("Total options count: {}", count);
            Ok(
                HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
                    "count": count
                }))),
            )
        }
        Err(e) => {
            error!("Failed to get options count: {}", e);
            Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to get options count")))
        }
    }
}

// Analytics Route Handlers

/// Get comprehensive options analytics
pub async fn get_options_analytics(
    req: HttpRequest,
    query: web::Query<TimeRangeQuery>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> Result<HttpResponse> {
    info!("Generating options analytics");
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
                let _cache_key = format!("analytics:db:{}:options:{:?}", user_id, time_range);
                // Return zeroed analytics (and do not cache)
                let analytics = OptionsAnalytics {
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
                info!("User DB not found; returning default options analytics");
                return Ok(HttpResponse::Ok().json(ApiResponse::success(analytics)));
            }
            return Err(e);
        }
    };
    let time_range = query.time_range.clone().unwrap_or(TimeRange::AllTime);

    // Collect all analytics in parallel for better performance
    let total_pnl = OptionTrade::calculate_total_pnl(&conn)
        .await
        .unwrap_or_default();
    let profit_factor = OptionTrade::calculate_profit_factor(&conn, time_range.clone())
        .await
        .unwrap_or_default();
    let win_rate = OptionTrade::calculate_win_rate(&conn, time_range.clone())
        .await
        .unwrap_or_default();
    let loss_rate = OptionTrade::calculate_loss_rate(&conn, time_range.clone())
        .await
        .unwrap_or_default();
    let avg_gain = OptionTrade::calculate_avg_gain(&conn, time_range.clone())
        .await
        .unwrap_or_default();
    let avg_loss = OptionTrade::calculate_avg_loss(&conn, time_range.clone())
        .await
        .unwrap_or_default();
    let biggest_winner = OptionTrade::calculate_biggest_winner(&conn, time_range.clone())
        .await
        .unwrap_or_default();
    let biggest_loser = OptionTrade::calculate_biggest_loser(&conn, time_range.clone())
        .await
        .unwrap_or_default();
    let avg_hold_time_winners =
        OptionTrade::calculate_avg_hold_time_winners(&conn, time_range.clone())
            .await
            .unwrap_or_default();
    let avg_hold_time_losers =
        OptionTrade::calculate_avg_hold_time_losers(&conn, time_range.clone())
            .await
            .unwrap_or_default();
    let risk_reward_ratio = OptionTrade::calculate_risk_reward_ratio(&conn, time_range.clone())
        .await
        .unwrap_or_default();
    let trade_expectancy = OptionTrade::calculate_trade_expectancy(&conn, time_range.clone())
        .await
        .unwrap_or_default();
    let avg_position_size = OptionTrade::calculate_avg_position_size(&conn, time_range.clone())
        .await
        .unwrap_or_default();
    let net_pnl = OptionTrade::calculate_net_pnl(&conn, time_range)
        .await
        .unwrap_or_default();

    let analytics = OptionsAnalytics {
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
    };

    info!("Generated comprehensive analytics");
    Ok(HttpResponse::Ok().json(ApiResponse::success(analytics)))
}

/// Get total P&L
pub async fn get_total_pnl(
    req: HttpRequest,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> Result<HttpResponse> {
    info!("Calculating total P&L");

    let conn = get_user_db_connection(&req, &turso_client, &supabase_config).await?;

    match OptionTrade::calculate_total_pnl(&conn).await {
        Ok(pnl) => {
            info!("Total P&L: {}", pnl);
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

/// Get profit factor
pub async fn get_profit_factor(
    req: HttpRequest,
    query: web::Query<TimeRangeQuery>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> Result<HttpResponse> {
    info!("Calculating profit factor");

    let conn = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let time_range = query.time_range.clone().unwrap_or(TimeRange::AllTime);

    match OptionTrade::calculate_profit_factor(&conn, time_range).await {
        Ok(factor) => {
            info!("Profit factor: {}", factor);
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

    match OptionTrade::calculate_win_rate(&conn, time_range).await {
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

    let pool = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let time_range = query.time_range.clone().unwrap_or(TimeRange::AllTime);

    match OptionTrade::calculate_loss_rate(&pool, time_range).await {
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

    let pool = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let time_range = query.time_range.clone().unwrap_or(TimeRange::AllTime);

    match OptionTrade::calculate_avg_gain(&pool, time_range).await {
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

    let pool = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let time_range = query.time_range.clone().unwrap_or(TimeRange::AllTime);

    match OptionTrade::calculate_avg_loss(&pool, time_range).await {
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

    let pool = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let time_range = query.time_range.clone().unwrap_or(TimeRange::AllTime);

    match OptionTrade::calculate_biggest_winner(&pool, time_range).await {
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

    let pool = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let time_range = query.time_range.clone().unwrap_or(TimeRange::AllTime);

    match OptionTrade::calculate_biggest_loser(&pool, time_range).await {
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

    let pool = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let time_range = query.time_range.clone().unwrap_or(TimeRange::AllTime);

    match OptionTrade::calculate_avg_hold_time_winners(&pool, time_range).await {
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

    let pool = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let time_range = query.time_range.clone().unwrap_or(TimeRange::AllTime);

    match OptionTrade::calculate_avg_hold_time_losers(&pool, time_range).await {
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

    let pool = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let time_range = query.time_range.clone().unwrap_or(TimeRange::AllTime);

    match OptionTrade::calculate_risk_reward_ratio(&pool, time_range).await {
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

    let pool = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let time_range = query.time_range.clone().unwrap_or(TimeRange::AllTime);

    match OptionTrade::calculate_trade_expectancy(&pool, time_range).await {
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

    let pool = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let time_range = query.time_range.clone().unwrap_or(TimeRange::AllTime);

    match OptionTrade::calculate_avg_position_size(&pool, time_range).await {
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

    let pool = get_user_db_connection(&req, &turso_client, &supabase_config).await?;
    let time_range = query.time_range.clone().unwrap_or(TimeRange::AllTime);

    match OptionTrade::calculate_net_pnl(&pool, time_range).await {
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

/// Test endpoint to verify options routes are working
async fn test_endpoint() -> Result<HttpResponse> {
    info!("Options test endpoint hit!");
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Options routes are working!",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": "1.0.0"
    })))
}

/// Configure options routes
pub fn configure_options_routes(cfg: &mut web::ServiceConfig) {
    info!("Setting up /api/options routes");
    cfg.service(
        web::scope("/api/options")
            // Test route
            .route("/test", web::get().to(test_endpoint))
            // CRUD operations
            .route("", web::post().to(create_option)) // POST /api/options
            .route("", web::get().to(get_all_options)) // GET /api/options?filters
            .route("/count", web::get().to(get_options_count)) // GET /api/options/count
            .route("/{id}", web::get().to(get_option_by_id)) // GET /api/options/{id}
            .route("/{id}", web::put().to(update_option)) // PUT /api/options/{id}
            .route("/{id}", web::delete().to(delete_option)) // DELETE /api/options/{id}
            // Analytics endpoints
            .route("/analytics", web::get().to(get_options_analytics)) // GET /api/options/analytics?time_range=
            .route("/analytics/pnl", web::get().to(get_total_pnl)) // GET /api/options/analytics/pnl
            .route("/analytics/profit-factor", web::get().to(get_profit_factor)) // GET /api/options/analytics/profit-factor?time_range=
            .route("/analytics/win-rate", web::get().to(get_win_rate)) // GET /api/options/analytics/win-rate?time_range=
            .route("/analytics/loss-rate", web::get().to(get_loss_rate)) // GET /api/options/analytics/loss-rate?time_range=
            .route("/analytics/avg-gain", web::get().to(get_avg_gain)) // GET /api/options/analytics/avg-gain?time_range=
            .route("/analytics/avg-loss", web::get().to(get_avg_loss)) // GET /api/options/analytics/avg-loss?time_range=
            .route(
                "/analytics/biggest-winner",
                web::get().to(get_biggest_winner),
            ) // GET /api/options/analytics/biggest-winner?time_range=
            .route("/analytics/biggest-loser", web::get().to(get_biggest_loser)) // GET /api/options/analytics/biggest-loser?time_range=
            .route(
                "/analytics/avg-hold-time-winners",
                web::get().to(get_avg_hold_time_winners),
            ) // GET /api/options/analytics/avg-hold-time-winners?time_range=
            .route(
                "/analytics/avg-hold-time-losers",
                web::get().to(get_avg_hold_time_losers),
            ) // GET /api/options/analytics/avg-hold-time-losers?time_range=
            .route(
                "/analytics/risk-reward-ratio",
                web::get().to(get_risk_reward_ratio),
            ) // GET /api/options/analytics/risk-reward-ratio?time_range=
            .route(
                "/analytics/trade-expectancy",
                web::get().to(get_trade_expectancy),
            ) // GET /api/options/analytics/trade-expectancy?time_range=
            .route(
                "/analytics/avg-position-size",
                web::get().to(get_avg_position_size),
            ) // GET /api/options/analytics/avg-position-size?time_range=
            .route("/analytics/net-pnl", web::get().to(get_net_pnl)), // GET /api/options/analytics/net-pnl?time_range=
    );
}
