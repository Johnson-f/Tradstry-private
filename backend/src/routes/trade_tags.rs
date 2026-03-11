use actix_web::{HttpMessage, HttpRequest, HttpResponse, Result, web};
use log::{debug, error, info, warn};
use serde::Serialize;

use crate::models::tags::{
    AddTagsToTradeRequest, CreateTagRequest, TagQuery, TradeTag, TradeTagAssociation,
    UpdateTagRequest,
};
use crate::turso::auth::{AuthError, validate_supabase_jwt_token};
use crate::turso::config::SupabaseConfig;
use crate::turso::{AppState, SupabaseClaims};

/// Parse JWT claims without full validation (for quick checks)
fn parse_jwt_claims(token: &str) -> Result<SupabaseClaims, AuthError> {
    use base64::{Engine as _, engine::general_purpose};

    info!("[TradeTags] Parsing JWT token, length: {}", token.len());

    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        error!(
            "[TradeTags] Invalid JWT format: expected 3 parts, got {}",
            parts.len()
        );
        return Err(AuthError::InvalidToken);
    }

    let payload_b64 = parts[1];
    let payload_bytes = general_purpose::URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|e| {
            error!("[TradeTags] Base64 decode error: {}", e);
            AuthError::InvalidToken
        })?;

    let claims: SupabaseClaims = serde_json::from_slice(&payload_bytes).map_err(|e| {
        error!("[TradeTags] JSON parsing error: {}", e);
        AuthError::InvalidToken
    })?;

    info!(
        "[TradeTags] Successfully parsed claims for user: {}",
        claims.sub
    );
    Ok(claims)
}

/// Extract JWT token from request headers
fn extract_token_from_request(req: &HttpRequest) -> Option<String> {
    let auth_header = req.headers().get("authorization");
    info!(
        "[TradeTags] Authorization header present: {}",
        auth_header.is_some()
    );

    if let Some(header_value) = auth_header {
        let header_str = header_value.to_str().ok()?;
        debug!(
            "[TradeTags] Authorization header value (first 30 chars): {}",
            if header_str.len() > 30 {
                &header_str[..30]
            } else {
                header_str
            }
        );

        if let Some(token) = header_str.strip_prefix("Bearer ") {
            info!("[TradeTags] Token extracted, length: {}", token.len());
            Some(token.to_string())
        } else {
            warn!("[TradeTags] Authorization header doesn't start with 'Bearer '");
            None
        }
    } else {
        warn!("[TradeTags] No authorization header found");
        None
    }
}

/// Extract and validate auth from request (similar to stocks/options routes)
async fn get_authenticated_user(
    req: &HttpRequest,
    supabase_config: &SupabaseConfig,
) -> Result<SupabaseClaims, actix_web::Error> {
    info!("[TradeTags] get_authenticated_user - Starting authentication");

    let token = extract_token_from_request(req).ok_or_else(|| {
        error!("[TradeTags] Missing authorization token");
        actix_web::error::ErrorUnauthorized("Missing authorization token")
    })?;

    // Parse claims first (quick check)
    let claims = parse_jwt_claims(&token).map_err(|e| {
        error!("[TradeTags] Failed to parse JWT claims: {}", e);
        actix_web::error::ErrorUnauthorized("Invalid token format")
    })?;

    // Validate with Supabase
    info!("[TradeTags] Validating token with Supabase");
    validate_supabase_jwt_token(&token, supabase_config)
        .await
        .map_err(|e| {
            error!("[TradeTags] JWT validation failed: {}", e);
            actix_web::error::ErrorUnauthorized("Invalid or expired authentication token")
        })?;

    info!(
        "[TradeTags] ✓ User authenticated successfully: {}",
        claims.sub
    );
    Ok(claims)
}

/// Extract user ID from request (legacy function - tries extensions first, then validates token)
async fn get_user_id_from_request(
    req: &HttpRequest,
    supabase_config: &SupabaseConfig,
) -> Result<String, actix_web::Error> {
    info!(
        "[TradeTags] Extracting user ID from request - path: {}",
        req.path()
    );

    // First, try to get from extensions (if middleware already ran)
    if let Some(claims) = req.extensions().get::<SupabaseClaims>() {
        info!(
            "[TradeTags] ✓ Found SupabaseClaims in extensions - user: {}",
            claims.sub
        );
        return Ok(claims.sub.clone());
    }

    info!("[TradeTags] SupabaseClaims not in extensions, validating token directly");

    // If not in extensions, validate token directly (fallback for routes without middleware)
    let claims = get_authenticated_user(req, supabase_config).await?;
    Ok(claims.sub)
}

/// Response wrapper for tag operations
#[derive(Debug, Serialize)]
pub struct TagResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<TradeTag>,
}

/// Response wrapper for tag list operations
#[derive(Debug, Serialize)]
pub struct TagListResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<Vec<TradeTag>>,
}

/// Response wrapper for category list operations
#[derive(Debug, Serialize)]
pub struct CategoryListResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<Vec<String>>,
}

/// Get all categories
pub async fn get_categories(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    _req: HttpRequest,
) -> Result<HttpResponse> {
    info!("[TradeTags] GET /api/trade-tags/categories - Starting request");
    let user_id = match get_user_id_from_request(&_req, &supabase_config).await {
        Ok(id) => {
            info!(
                "[TradeTags] GET /api/trade-tags/categories - User authenticated: {}",
                id
            );
            id
        }
        Err(e) => {
            error!(
                "[TradeTags] GET /api/trade-tags/categories - Authentication failed: {}",
                e
            );
            return Err(e);
        }
    };

    info!(
        "[TradeTags] GET /api/trade-tags/categories - Getting database connection for user: {}",
        user_id
    );
    let conn = app_state
        .get_user_db_connection(&user_id)
        .await
        .map_err(|e| {
            error!("[TradeTags] GET /api/trade-tags/categories - Failed to get user database connection: {}", e);
            actix_web::error::ErrorInternalServerError("Database connection failed")
        })?
        .ok_or_else(|| {
            error!("[TradeTags] GET /api/trade-tags/categories - User database not found for user: {}", user_id);
            actix_web::error::ErrorNotFound("User database not found")
        })?;

    info!(
        "[TradeTags] GET /api/trade-tags/categories - Database connection established, fetching categories"
    );
    match TradeTag::get_categories(&conn).await {
        Ok(categories) => {
            info!(
                "[TradeTags] GET /api/trade-tags/categories - ✓ Successfully retrieved {} categories",
                categories.len()
            );
            Ok(HttpResponse::Ok().json(CategoryListResponse {
                success: true,
                message: "Categories retrieved successfully".to_string(),
                data: Some(categories),
            }))
        }
        Err(e) => {
            error!(
                "[TradeTags] GET /api/trade-tags/categories - Failed to get categories: {}",
                e
            );
            Ok(
                HttpResponse::InternalServerError().json(CategoryListResponse {
                    success: false,
                    message: format!("Failed to get categories: {}", e),
                    data: None,
                }),
            )
        }
    }
}

/// Get all tags (with optional category filter)
pub async fn get_tags(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    query: web::Query<TagQuery>,
    _req: HttpRequest,
) -> Result<HttpResponse> {
    info!(
        "[TradeTags] GET /api/trade-tags - Starting request, query: {:?}",
        query
    );
    let user_id = match get_user_id_from_request(&_req, &supabase_config).await {
        Ok(id) => {
            info!(
                "[TradeTags] GET /api/trade-tags - User authenticated: {}",
                id
            );
            id
        }
        Err(e) => {
            error!(
                "[TradeTags] GET /api/trade-tags - Authentication failed: {}",
                e
            );
            return Err(e);
        }
    };

    info!(
        "[TradeTags] GET /api/trade-tags - Getting database connection for user: {}",
        user_id
    );
    let conn = app_state
        .get_user_db_connection(&user_id)
        .await
        .map_err(|e| {
            error!(
                "[TradeTags] GET /api/trade-tags - Failed to get user database connection: {}",
                e
            );
            actix_web::error::ErrorInternalServerError("Database connection failed")
        })?
        .ok_or_else(|| {
            error!(
                "[TradeTags] GET /api/trade-tags - User database not found for user: {}",
                user_id
            );
            actix_web::error::ErrorNotFound("User database not found")
        })?;

    info!("[TradeTags] GET /api/trade-tags - Database connection established, fetching tags");
    match TradeTag::find_all(&conn, Some(query.into_inner())).await {
        Ok(tags) => {
            info!(
                "[TradeTags] GET /api/trade-tags - ✓ Successfully retrieved {} tags",
                tags.len()
            );
            Ok(HttpResponse::Ok().json(TagListResponse {
                success: true,
                message: "Tags retrieved successfully".to_string(),
                data: Some(tags),
            }))
        }
        Err(e) => {
            error!(
                "[TradeTags] GET /api/trade-tags - Failed to get tags: {}",
                e
            );
            Ok(HttpResponse::InternalServerError().json(TagListResponse {
                success: false,
                message: format!("Failed to get tags: {}", e),
                data: None,
            }))
        }
    }
}

/// Get a single tag by ID
pub async fn get_tag(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<String>,
    _req: HttpRequest,
) -> Result<HttpResponse> {
    info!("[TradeTags] GET /api/trade-tags/{{id}} - Starting request");
    let user_id = get_user_id_from_request(&_req, &supabase_config).await?;
    info!(
        "[TradeTags] GET /api/trade-tags/{{id}} - User authenticated: {}",
        user_id
    );
    let tag_id = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&user_id)
        .await
        .map_err(|e| {
            error!("Failed to get user database connection: {}", e);
            actix_web::error::ErrorInternalServerError("Database connection failed")
        })?
        .ok_or_else(|| {
            error!("User database not found for user: {}", user_id);
            actix_web::error::ErrorNotFound("User database not found")
        })?;

    match TradeTag::find_by_id(&conn, &tag_id).await {
        Ok(tag) => Ok(HttpResponse::Ok().json(TagResponse {
            success: true,
            message: "Tag retrieved successfully".to_string(),
            data: Some(tag),
        })),
        Err(e) => {
            error!("Failed to get tag: {}", e);
            Ok(HttpResponse::NotFound().json(TagResponse {
                success: false,
                message: format!("Tag not found: {}", e),
                data: None,
            }))
        }
    }
}

/// Create a new tag
pub async fn create_tag(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    payload: web::Json<CreateTagRequest>,
    _req: HttpRequest,
) -> Result<HttpResponse> {
    info!("[TradeTags] POST /api/trade-tags - Starting request");
    debug!("[TradeTags] Request payload: {:?}", payload);
    info!(
        "[TradeTags] Request details: category={}, name={}, color={:?}, description={:?}",
        payload.category, payload.name, payload.color, payload.description
    );

    let user_id = match get_user_id_from_request(&_req, &supabase_config).await {
        Ok(id) => {
            info!(
                "[TradeTags] POST /api/trade-tags - User authenticated: {}",
                id
            );
            id
        }
        Err(e) => {
            error!(
                "[TradeTags] POST /api/trade-tags - Authentication failed: {}",
                e
            );
            return Err(e);
        }
    };

    info!(
        "[TradeTags] Getting database connection for user: {}",
        user_id
    );
    let conn = match app_state.get_user_db_connection(&user_id).await {
        Ok(Some(conn)) => {
            info!(
                "[TradeTags] ✓ Database connection established for user: {}",
                user_id
            );
            conn
        }
        Ok(None) => {
            error!(
                "[TradeTags] ✗ User database not found for user: {}",
                user_id
            );
            return Ok(HttpResponse::NotFound().json(TagResponse {
                success: false,
                message: format!("User database not found for user: {}", user_id),
                data: None,
            }));
        }
        Err(e) => {
            error!(
                "[TradeTags] ✗ Failed to get user database connection: {}",
                e
            );
            return Ok(HttpResponse::InternalServerError().json(TagResponse {
                success: false,
                message: format!("Database connection failed: {}", e),
                data: None,
            }));
        }
    };

    // Check storage quota before creating
    app_state
        .storage_quota_service
        .check_storage_quota(&user_id, &conn)
        .await
        .map_err(|e| {
            error!(
                "[TradeTags] Storage quota check failed for user {}: {}",
                user_id, e
            );
            e
        })?;

    let create_request = payload.into_inner();
    info!(
        "[TradeTags] Calling TradeTag::create with: category={}, name={}",
        create_request.category, create_request.name
    );

    match TradeTag::create(&conn, create_request).await {
        Ok(tag) => {
            info!(
                "[TradeTags] ✓ Tag created successfully: id={}, category={}, name={}",
                tag.id, tag.category, tag.name
            );
            Ok(HttpResponse::Created().json(TagResponse {
                success: true,
                message: "Tag created successfully".to_string(),
                data: Some(tag),
            }))
        }
        Err(e) => {
            error!("[TradeTags] ✗ Failed to create tag: {}", e);
            error!("[TradeTags] Error chain: {:?}", e);
            // Log the error source if available
            if let Some(source) = e.source() {
                error!("[TradeTags] Error source: {}", source);
            }
            Ok(HttpResponse::InternalServerError().json(TagResponse {
                success: false,
                message: format!("Failed to create tag: {}", e),
                data: None,
            }))
        }
    }
}

/// Update a tag
pub async fn update_tag(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<String>,
    payload: web::Json<UpdateTagRequest>,
    _req: HttpRequest,
) -> Result<HttpResponse> {
    info!("[TradeTags] PUT /api/trade-tags/{{id}} - Starting request");
    let user_id = get_user_id_from_request(&_req, &supabase_config).await?;
    info!(
        "[TradeTags] PUT /api/trade-tags/{{id}} - User authenticated: {}",
        user_id
    );
    let tag_id = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&user_id)
        .await
        .map_err(|e| {
            error!("Failed to get user database connection: {}", e);
            actix_web::error::ErrorInternalServerError("Database connection failed")
        })?
        .ok_or_else(|| {
            error!("User database not found for user: {}", user_id);
            actix_web::error::ErrorNotFound("User database not found")
        })?;

    match TradeTag::update(&conn, &tag_id, payload.into_inner()).await {
        Ok(tag) => {
            info!("✓ Tag updated successfully: {}", tag.id);
            Ok(HttpResponse::Ok().json(TagResponse {
                success: true,
                message: "Tag updated successfully".to_string(),
                data: Some(tag),
            }))
        }
        Err(e) => {
            error!("Failed to update tag: {}", e);
            Ok(HttpResponse::InternalServerError().json(TagResponse {
                success: false,
                message: format!("Failed to update tag: {}", e),
                data: None,
            }))
        }
    }
}

/// Delete a tag
pub async fn delete_tag(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<String>,
    _req: HttpRequest,
) -> Result<HttpResponse> {
    info!("[TradeTags] DELETE /api/trade-tags/{{id}} - Starting request");
    let user_id = get_user_id_from_request(&_req, &supabase_config).await?;
    info!(
        "[TradeTags] DELETE /api/trade-tags/{{id}} - User authenticated: {}",
        user_id
    );
    let tag_id = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&user_id)
        .await
        .map_err(|e| {
            error!("Failed to get user database connection: {}", e);
            actix_web::error::ErrorInternalServerError("Database connection failed")
        })?
        .ok_or_else(|| {
            error!("User database not found for user: {}", user_id);
            actix_web::error::ErrorNotFound("User database not found")
        })?;

    match TradeTag::delete(&conn, &tag_id).await {
        Ok(deleted) => {
            if deleted {
                info!("✓ Tag deleted successfully: {}", tag_id);
                Ok(HttpResponse::Ok().json(serde_json::json!({
                    "success": true,
                    "message": "Tag deleted successfully"
                })))
            } else {
                Ok(HttpResponse::NotFound().json(serde_json::json!({
                    "success": false,
                    "message": "Tag not found"
                })))
            }
        }
        Err(e) => {
            error!("Failed to delete tag: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "message": format!("Failed to delete tag: {}", e)
            })))
        }
    }
}

/// Get tags for a stock trade
pub async fn get_stock_trade_tags(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<i64>,
    _req: HttpRequest,
) -> Result<HttpResponse> {
    info!("[TradeTags] GET /api/trades/stock/{{id}}/tags - Starting request");
    let user_id = get_user_id_from_request(&_req, &supabase_config).await?;
    info!(
        "[TradeTags] GET /api/trades/stock/{{id}}/tags - User authenticated: {}",
        user_id
    );
    let stock_trade_id = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&user_id)
        .await
        .map_err(|e| {
            error!("Failed to get user database connection: {}", e);
            actix_web::error::ErrorInternalServerError("Database connection failed")
        })?
        .ok_or_else(|| {
            error!("User database not found for user: {}", user_id);
            actix_web::error::ErrorNotFound("User database not found")
        })?;

    match TradeTagAssociation::get_tags_for_stock_trade(&conn, stock_trade_id).await {
        Ok(tags) => Ok(HttpResponse::Ok().json(TagListResponse {
            success: true,
            message: "Tags retrieved successfully".to_string(),
            data: Some(tags),
        })),
        Err(e) => {
            error!("Failed to get tags for stock trade: {}", e);
            Ok(HttpResponse::InternalServerError().json(TagListResponse {
                success: false,
                message: format!("Failed to get tags: {}", e),
                data: None,
            }))
        }
    }
}

/// Get tags for an option trade
pub async fn get_option_trade_tags(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<i64>,
    _req: HttpRequest,
) -> Result<HttpResponse> {
    info!("[TradeTags] GET /api/trades/option/{{id}}/tags - Starting request");
    let user_id = get_user_id_from_request(&_req, &supabase_config).await?;
    info!(
        "[TradeTags] GET /api/trades/option/{{id}}/tags - User authenticated: {}",
        user_id
    );
    let option_trade_id = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&user_id)
        .await
        .map_err(|e| {
            error!("Failed to get user database connection: {}", e);
            actix_web::error::ErrorInternalServerError("Database connection failed")
        })?
        .ok_or_else(|| {
            error!("User database not found for user: {}", user_id);
            actix_web::error::ErrorNotFound("User database not found")
        })?;

    match TradeTagAssociation::get_tags_for_option_trade(&conn, option_trade_id).await {
        Ok(tags) => Ok(HttpResponse::Ok().json(TagListResponse {
            success: true,
            message: "Tags retrieved successfully".to_string(),
            data: Some(tags),
        })),
        Err(e) => {
            error!("Failed to get tags for option trade: {}", e);
            Ok(HttpResponse::InternalServerError().json(TagListResponse {
                success: false,
                message: format!("Failed to get tags: {}", e),
                data: None,
            }))
        }
    }
}

/// Add tags to a stock trade
pub async fn add_tags_to_stock_trade(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<i64>,
    payload: web::Json<AddTagsToTradeRequest>,
    _req: HttpRequest,
) -> Result<HttpResponse> {
    info!("[TradeTags] POST /api/trades/stock/{{id}}/tags - Starting request");
    let user_id = get_user_id_from_request(&_req, &supabase_config).await?;
    info!(
        "[TradeTags] POST /api/trades/stock/{{id}}/tags - User authenticated: {}",
        user_id
    );
    let stock_trade_id = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&user_id)
        .await
        .map_err(|e| {
            error!("Failed to get user database connection: {}", e);
            actix_web::error::ErrorInternalServerError("Database connection failed")
        })?
        .ok_or_else(|| {
            error!("User database not found for user: {}", user_id);
            actix_web::error::ErrorNotFound("User database not found")
        })?;

    let mut added_count = 0;
    let mut skipped_count = 0;

    for tag_id in &payload.tag_ids {
        match TradeTagAssociation::add_tag_to_stock_trade(&conn, stock_trade_id, tag_id).await {
            Ok(true) => added_count += 1,
            Ok(false) => skipped_count += 1, // Already exists
            Err(e) => {
                error!("Failed to add tag {} to stock trade: {}", tag_id, e);
            }
        }
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": format!("Added {} tag(s), {} already existed", added_count, skipped_count),
        "added": added_count,
        "skipped": skipped_count
    })))
}

/// Add tags to an option trade
pub async fn add_tags_to_option_trade(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<i64>,
    payload: web::Json<AddTagsToTradeRequest>,
    _req: HttpRequest,
) -> Result<HttpResponse> {
    info!("[TradeTags] POST /api/trades/option/{{id}}/tags - Starting request");
    let user_id = get_user_id_from_request(&_req, &supabase_config).await?;
    info!(
        "[TradeTags] POST /api/trades/option/{{id}}/tags - User authenticated: {}",
        user_id
    );
    let option_trade_id = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&user_id)
        .await
        .map_err(|e| {
            error!("Failed to get user database connection: {}", e);
            actix_web::error::ErrorInternalServerError("Database connection failed")
        })?
        .ok_or_else(|| {
            error!("User database not found for user: {}", user_id);
            actix_web::error::ErrorNotFound("User database not found")
        })?;

    let mut added_count = 0;
    let mut skipped_count = 0;

    for tag_id in &payload.tag_ids {
        match TradeTagAssociation::add_tag_to_option_trade(&conn, option_trade_id, tag_id).await {
            Ok(true) => added_count += 1,
            Ok(false) => skipped_count += 1, // Already exists
            Err(e) => {
                error!("Failed to add tag {} to option trade: {}", tag_id, e);
            }
        }
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": format!("Added {} tag(s), {} already existed", added_count, skipped_count),
        "added": added_count,
        "skipped": skipped_count
    })))
}

/// Remove a tag from a stock trade
pub async fn remove_tag_from_stock_trade(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<(i64, String)>,
    _req: HttpRequest,
) -> Result<HttpResponse> {
    info!("[TradeTags] DELETE /api/trades/stock/{{id}}/tags/{{tag_id}} - Starting request");
    let user_id = get_user_id_from_request(&_req, &supabase_config).await?;
    info!(
        "[TradeTags] DELETE /api/trades/stock/{{id}}/tags/{{tag_id}} - User authenticated: {}",
        user_id
    );
    let (stock_trade_id, tag_id) = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&user_id)
        .await
        .map_err(|e| {
            error!("Failed to get user database connection: {}", e);
            actix_web::error::ErrorInternalServerError("Database connection failed")
        })?
        .ok_or_else(|| {
            error!("User database not found for user: {}", user_id);
            actix_web::error::ErrorNotFound("User database not found")
        })?;

    match TradeTagAssociation::remove_tag_from_stock_trade(&conn, stock_trade_id, &tag_id).await {
        Ok(true) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Tag removed successfully"
        }))),
        Ok(false) => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "message": "Tag association not found"
        }))),
        Err(e) => {
            error!("Failed to remove tag from stock trade: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "message": format!("Failed to remove tag: {}", e)
            })))
        }
    }
}

/// Remove a tag from an option trade
pub async fn remove_tag_from_option_trade(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<(i64, String)>,
    _req: HttpRequest,
) -> Result<HttpResponse> {
    info!("[TradeTags] DELETE /api/trades/option/{{id}}/tags/{{tag_id}} - Starting request");
    let user_id = get_user_id_from_request(&_req, &supabase_config).await?;
    info!(
        "[TradeTags] DELETE /api/trades/option/{{id}}/tags/{{tag_id}} - User authenticated: {}",
        user_id
    );
    let (option_trade_id, tag_id) = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&user_id)
        .await
        .map_err(|e| {
            error!("Failed to get user database connection: {}", e);
            actix_web::error::ErrorInternalServerError("Database connection failed")
        })?
        .ok_or_else(|| {
            error!("User database not found for user: {}", user_id);
            actix_web::error::ErrorNotFound("User database not found")
        })?;

    match TradeTagAssociation::remove_tag_from_option_trade(&conn, option_trade_id, &tag_id).await {
        Ok(true) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Tag removed successfully"
        }))),
        Ok(false) => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "message": "Tag association not found"
        }))),
        Err(e) => {
            error!("Failed to remove tag from option trade: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "message": format!("Failed to remove tag: {}", e)
            })))
        }
    }
}

/// Configure trade tags routes
pub fn configure_trade_tags_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/trade-tags")
            .route("/categories", web::get().to(get_categories))
            .route("", web::get().to(get_tags))
            .route("", web::post().to(create_tag))
            .route("/{id}", web::get().to(get_tag))
            .route("/{id}", web::put().to(update_tag))
            .route("/{id}", web::delete().to(delete_tag)),
    );
    cfg.service(
        web::scope("/api/trades/stock/{id}/tags")
            .route("", web::get().to(get_stock_trade_tags))
            .route("", web::post().to(add_tags_to_stock_trade))
            .route("/{tag_id}", web::delete().to(remove_tag_from_stock_trade)),
    );
    cfg.service(
        web::scope("/api/trades/option/{id}/tags")
            .route("", web::get().to(get_option_trade_tags))
            .route("", web::post().to(add_tags_to_option_trade))
            .route("/{tag_id}", web::delete().to(remove_tag_from_option_trade)),
    );
}
