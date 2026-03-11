use crate::service::caching::CacheService;
use crate::turso::auth::{AuthError, validate_supabase_jwt_token};
use crate::turso::config::{SupabaseClaims, SupabaseConfig};
use crate::turso::schema::get_current_schema_version;
use crate::turso::{AppState, client::TursoClient};
use actix_multipart::Multipart;
use actix_web::{HttpRequest, HttpResponse, Result, web};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Request payload for user database initialization
#[derive(Debug, Deserialize)]
pub struct InitializeUserRequest {
    pub email: String,
    pub user_id: String,
}

/// Response payload for user database initialization
#[derive(Debug, Serialize)]
pub struct InitializeUserResponse {
    pub success: bool,
    pub message: String,
    pub database_url: Option<String>,
    pub database_token: Option<String>,
    pub schema_synced: Option<bool>,
    pub schema_version: Option<String>,
    pub cache_preloaded: Option<bool>,
    pub cache_status: Option<String>,
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

/// Initialize user database with trading schema and preload cache
pub async fn initialize_user_database(
    req: HttpRequest,
    payload: web::Json<InitializeUserRequest>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
    cache_service: web::Data<Arc<CacheService>>,
) -> Result<HttpResponse> {
    info!("=== Initialize User Database Called ===");
    info!("User: {} ({})", payload.email, payload.user_id);

    // Log that extractors worked
    info!("✓ TursoClient extracted successfully");
    info!("✓ SupabaseConfig extracted successfully");
    info!("✓ Request payload extracted successfully");

    // Get authenticated user with detailed error logging
    info!("Attempting authentication...");
    let claims = match get_authenticated_user(&req, &supabase_config).await {
        Ok(c) => {
            info!("✓ Authentication successful for user: {}", c.sub);
            c
        }
        Err(e) => {
            error!("✗ Authentication failed: {:?}", e);
            return Err(e);
        }
    };

    // Validate that the authenticated user matches the requested user_id
    if claims.sub != payload.user_id {
        warn!(
            "User ID mismatch: JWT sub={}, payload user_id={}",
            claims.sub, payload.user_id
        );
        return Ok(HttpResponse::Forbidden().json(InitializeUserResponse {
            success: false,
            message: "User ID mismatch - you can only initialize your own database".to_string(),
            database_url: None,
            database_token: None,
            schema_synced: None,
            schema_version: None,
            cache_preloaded: None,
            cache_status: Some("Not attempted - authentication failed".to_string()),
        }));
    }

    match create_user_database_internal(&turso_client, &payload.user_id, &payload.email).await {
        Ok((db_url, db_token, schema_synced, schema_version)) => {
            info!(
                "Successfully initialized database for user: {}",
                payload.email
            );

            // Preload user data into cache asynchronously
            let cache_service_clone = cache_service.get_ref().clone();
            let user_id_clone = payload.user_id.clone();

            tokio::spawn(async move {
                match turso_client
                    .get_user_database_connection(&user_id_clone)
                    .await
                {
                    Ok(Some(user_conn)) => {
                        info!("Starting cache preload for user: {}", user_id_clone);
                        match cache_service_clone
                            .preload_user_data(&user_conn, &user_id_clone)
                            .await
                        {
                            Ok(_) => {
                                info!(
                                    "✓ Cache preload completed successfully for user: {}",
                                    user_id_clone
                                );
                            }
                            Err(e) => {
                                error!("✗ Cache preload failed for user {}: {}", user_id_clone, e);
                            }
                        }
                    }
                    Ok(None) => {
                        warn!(
                            "No database connection available for cache preload for user: {}",
                            user_id_clone
                        );
                    }
                    Err(e) => {
                        error!(
                            "Failed to get database connection for cache preload for user {}: {}",
                            user_id_clone, e
                        );
                    }
                }
            });

            Ok(HttpResponse::Ok().json(InitializeUserResponse {
                success: true,
                message:
                    "User database initialized successfully. Cache preload started in background."
                        .to_string(),
                database_url: Some(db_url),
                database_token: Some(db_token),
                schema_synced: Some(schema_synced),
                schema_version: Some(schema_version),
                cache_preloaded: Some(false), // Will be true when background task completes
                cache_status: Some("Preloading in background".to_string()),
            }))
        }
        Err(e) => {
            error!(
                "Failed to initialize database for user {}: {}",
                payload.email, e
            );
            Ok(
                HttpResponse::InternalServerError().json(InitializeUserResponse {
                    success: false,
                    message: format!("Failed to initialize database: {}", e),
                    database_url: None,
                    database_token: None,
                    schema_synced: None,
                    schema_version: None,
                    cache_preloaded: None,
                    cache_status: Some(
                        "Not attempted - database initialization failed".to_string(),
                    ),
                }),
            )
        }
    }
}

/// Check if user database exists and is properly initialized
pub async fn check_user_database(
    req: HttpRequest,
    user_id: web::Path<String>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> Result<HttpResponse> {
    info!("Checking database status for user: {}", user_id);

    // Get authenticated user
    let claims = get_authenticated_user(&req, &supabase_config).await?;

    // Validate that the authenticated user matches the requested user_id
    if claims.sub != *user_id {
        warn!(
            "User ID mismatch: JWT sub={}, requested user_id={}",
            claims.sub, user_id
        );
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "You can only check your own database status"
        })));
    }

    match turso_client.get_user_database(&user_id).await {
        Ok(Some(mut db_entry)) => {
            info!("Database found for user: {}", user_id);

            // Check if schema needs sync by comparing registry version with app version
            let current_app_version = get_current_schema_version();
            let mut needs_sync = match &db_entry.schema_version {
                Some(v) => v != &current_app_version.version,
                None => true, // No version recorded means needs sync
            };

            // If out of date, attempt to sync immediately for the authenticated user
            if needs_sync {
                info!(
                    "Schema out of date for user {} (registry={:?}, app={}), syncing now",
                    user_id, db_entry.schema_version, current_app_version.version
                );

                match turso_client.sync_user_database_schema(&user_id).await {
                    Ok(_) => {
                        // Update registry record to the latest version
                        if let Err(e) = turso_client
                            .update_registry_schema_version(&user_id, &current_app_version.version)
                            .await
                        {
                            warn!(
                                "Schema synced but failed to update registry for user {}: {}",
                                user_id, e
                            );
                        } else {
                            db_entry.schema_version = Some(current_app_version.version.clone());
                            needs_sync = false;
                            info!("Schema synchronized for user {}", user_id);
                        }
                    }
                    Err(e) => {
                        warn!(
                            "Failed to synchronize schema for user {} during login check: {}",
                            user_id, e
                        );
                    }
                }
            }

            Ok(HttpResponse::Ok().json(serde_json::json!({
                "exists": true,
                "database_url": db_entry.db_url,
                "created_at": db_entry.created_at,
                "schema_version": db_entry.schema_version,
                "needs_sync": needs_sync,
                "current_app_version": current_app_version.version
            })))
        }
        Ok(None) => {
            info!("No database found for user: {}", user_id);
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "exists": false
            })))
        }
        Err(e) => {
            error!("Error checking database for user {}: {}", user_id, e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to check database status"
            })))
        }
    }
}

/// Get user database connection info (URL and token)
pub async fn get_user_database_info(
    req: HttpRequest,
    user_id: web::Path<String>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> Result<HttpResponse> {
    info!("Getting database info for user: {}", user_id);

    // Get authenticated user
    let claims = get_authenticated_user(&req, &supabase_config).await?;

    // Validate that the authenticated user matches the requested user_id
    if claims.sub != *user_id {
        warn!(
            "User ID mismatch: JWT sub={}, requested user_id={}",
            claims.sub, user_id
        );
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "You can only access your own database info"
        })));
    }

    match turso_client.get_user_database(&user_id).await {
        Ok(Some(db_entry)) => {
            // Generate a fresh token for the user's database
            match turso_client.create_database_token(&db_entry.db_name).await {
                Ok(fresh_token) => {
                    info!("Generated fresh token for user: {}", user_id);
                    Ok(HttpResponse::Ok().json(serde_json::json!({
                        "database_url": db_entry.db_url,
                        "database_token": fresh_token,
                        "database_name": db_entry.db_name
                    })))
                }
                Err(e) => {
                    error!("Failed to generate token for user {}: {}", user_id, e);
                    Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": "Failed to generate database token"
                    })))
                }
            }
        }
        Ok(None) => {
            info!("No database found for user: {}", user_id);
            Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": "User database not found"
            })))
        }
        Err(e) => {
            error!("Error getting database info for user {}: {}", user_id, e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to get database info"
            })))
        }
    }
}

/// Internal function to create user database and initialize schema
async fn create_user_database_internal(
    turso_client: &Arc<TursoClient>,
    user_id: &str,
    email: &str,
) -> Result<(String, String, bool, String), anyhow::Error> {
    let current_schema_version = get_current_schema_version();

    // Check if database already exists
    match turso_client.get_user_database(user_id).await? {
        Some(existing_db) => {
            info!("Database already exists for user: {}", user_id);

            // Generate a fresh token for existing database
            let fresh_token = turso_client
                .create_database_token(&existing_db.db_name)
                .await?;
            // Schema sync is handled by background process on backend startup
            return Ok((
                existing_db.db_url,
                fresh_token,
                true,
                current_schema_version.version,
            ));
        }
        None => {
            info!("Creating new database for user: {}", user_id);
        }
    }

    // Create new database for user
    let user_db_entry = turso_client.create_user_database(user_id, email).await?;

    // For new databases, schema is automatically initialized with the latest version
    info!(
        "Successfully created database for user: {} at {}",
        user_id, user_db_entry.db_url
    );
    Ok((
        user_db_entry.db_url,
        user_db_entry.db_token,
        true,
        current_schema_version.version,
    ))
}

/// Synchronize user database schema with current application schema
pub async fn sync_user_schema(
    req: HttpRequest,
    user_id: web::Path<String>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> Result<HttpResponse> {
    info!("Schema synchronization requested for user: {}", user_id);

    // Get authenticated user
    let claims = get_authenticated_user(&req, &supabase_config).await?;

    // Validate that the authenticated user matches the requested user_id
    if claims.sub != *user_id {
        warn!(
            "User ID mismatch: JWT sub={}, requested user_id={}",
            claims.sub, user_id
        );
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "You can only synchronize your own database schema"
        })));
    }

    // Check if user database exists
    match turso_client.get_user_database(&user_id).await {
        Ok(Some(_)) => {
            // Perform schema synchronization
            match turso_client.sync_user_database_schema(&user_id).await {
                Ok(_) => {
                    let current_version = get_current_schema_version();

                    // Update registry with new schema version
                    if let Err(e) = turso_client
                        .update_registry_schema_version(&user_id, &current_version.version)
                        .await
                    {
                        warn!(
                            "Schema synced but failed to update registry for user {}: {}",
                            user_id, e
                        );
                    }

                    info!("Schema synchronized successfully for user: {}", user_id);
                    Ok(HttpResponse::Ok().json(serde_json::json!({
                        "success": true,
                        "message": "Schema synchronized successfully",
                        "schema_version": current_version.version,
                        "synced_at": chrono::Utc::now().to_rfc3339()
                    })))
                }
                Err(e) => {
                    error!("Failed to synchronize schema for user {}: {}", user_id, e);
                    Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                        "success": false,
                        "error": "Failed to synchronize schema",
                        "details": e.to_string()
                    })))
                }
            }
        }
        Ok(None) => {
            info!("No database found for user: {}", user_id);
            Ok(HttpResponse::NotFound().json(serde_json::json!({
                "success": false,
                "error": "User database not found. Please initialize your database first."
            })))
        }
        Err(e) => {
            error!("Error checking database for user {}: {}", user_id, e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Failed to check database status"
            })))
        }
    }
}

/// Get current schema version for user database
pub async fn get_user_schema_version(
    req: HttpRequest,
    user_id: web::Path<String>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> Result<HttpResponse> {
    info!("Getting schema version for user: {}", user_id);

    // Get authenticated user
    let claims = get_authenticated_user(&req, &supabase_config).await?;

    // Validate that the authenticated user matches the requested user_id
    if claims.sub != *user_id {
        warn!(
            "User ID mismatch: JWT sub={}, requested user_id={}",
            claims.sub, user_id
        );
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "You can only check your own database schema version"
        })));
    }

    match turso_client.get_user_schema_version(&user_id).await {
        Ok(Some(version)) => {
            let current_version = get_current_schema_version();
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "user_schema_version": version.version,
                "user_schema_description": version.description,
                "user_schema_created_at": version.created_at,
                "current_app_version": current_version.version,
                "current_app_description": current_version.description,
                "is_up_to_date": version.version == current_version.version
            })))
        }
        Ok(None) => {
            let current_version = get_current_schema_version();
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "user_schema_version": null,
                "user_schema_description": "No schema version found (legacy database)",
                "user_schema_created_at": null,
                "current_app_version": current_version.version,
                "current_app_description": current_version.description,
                "is_up_to_date": false,
                "needs_sync": true
            })))
        }
        Err(e) => {
            error!("Error getting schema version for user {}: {}", user_id, e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to get schema version"
            })))
        }
    }
}

/// Simple test endpoint to verify routes are working
async fn test_endpoint() -> Result<HttpResponse> {
    info!("Test endpoint hit!");
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "User routes are working!",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

/// Request payload for user profile update
#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub nickname: Option<String>,
    pub display_name: Option<String>,
    pub timezone: Option<String>,
    pub currency: Option<String>,
    pub trading_experience_level: Option<String>,
    pub primary_trading_goal: Option<String>,
    pub asset_types: Option<String>, // JSON array as string
    pub trading_style: Option<String>,
}

/// Response payload for profile update
#[derive(Debug, Serialize)]
pub struct ProfileResponse {
    pub success: bool,
    pub message: String,
}

/// Get user profile
pub async fn get_profile(
    req: HttpRequest,
    user_id: web::Path<String>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> Result<HttpResponse> {
    info!("Getting profile for user: {}", user_id);

    // Get authenticated user
    let claims = get_authenticated_user(&req, &supabase_config).await?;

    // Validate that the authenticated user matches the requested user_id
    if claims.sub != *user_id {
        warn!(
            "User ID mismatch: JWT sub={}, requested user_id={}",
            claims.sub, user_id
        );
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "success": false,
            "error": "You can only view your own profile"
        })));
    }

    // Get user database connection
    match turso_client.get_user_database_connection(&user_id).await {
        Ok(Some(conn)) => {
            // Check if user_profile table exists before querying
            let table_exists = match conn
                .prepare(
                    "SELECT name FROM sqlite_master WHERE type='table' AND name='user_profile'",
                )
                .await
            {
                Ok(stmt) => match stmt.query(libsql::params![]).await {
                    Ok(mut rows) => rows.next().await.map(|r| r.is_some()).unwrap_or(false),
                    Err(_) => false,
                },
                Err(_) => false,
            };

            if !table_exists {
                warn!(
                    "user_profile table does not exist for user {}, returning empty profile",
                    user_id
                );
                // Return empty profile if table doesn't exist
                return Ok(HttpResponse::Ok().json(serde_json::json!({
                    "success": true,
                    "profile": {
                        "nickname": null,
                        "display_name": null,
                        "timezone": null,
                        "currency": null,
                        "trading_experience_level": null,
                        "primary_trading_goal": null,
                        "asset_types": null,
                        "trading_style": null,
                        "profile_picture_uuid": null,
                    }
                })));
            }

            // Try to query the profile table
            let stmt_result = conn.prepare(
                "SELECT nickname, display_name, timezone, currency, trading_experience_level, primary_trading_goal, asset_types, trading_style, profile_picture_uuid FROM user_profile LIMIT 1"
            ).await;

            let stmt = match stmt_result {
                Ok(stmt) => stmt,
                Err(e) => {
                    error!(
                        "Failed to prepare profile query for user {}: {}",
                        user_id, e
                    );
                    // If query preparation fails (e.g., column doesn't exist), return empty profile
                    return Ok(HttpResponse::Ok().json(serde_json::json!({
                        "success": true,
                        "profile": {
                            "nickname": null,
                            "display_name": null,
                            "timezone": null,
                            "currency": null,
                            "trading_experience_level": null,
                            "primary_trading_goal": null,
                            "asset_types": null,
                            "trading_style": null,
                            "profile_picture_uuid": null,
                        }
                    })));
                }
            };

            let mut rows = stmt.query(libsql::params![]).await.map_err(|e| {
                error!("Failed to query profile: {}", e);
                actix_web::error::ErrorInternalServerError("Database error")
            })?;

            if let Some(row) = rows.next().await.map_err(|e| {
                error!("Failed to read profile row: {}", e);
                actix_web::error::ErrorInternalServerError("Database error")
            })? {
                let profile = serde_json::json!({
                    "nickname": row.get::<Option<String>>(0).ok().flatten(),
                    "display_name": row.get::<Option<String>>(1).ok().flatten(),
                    "timezone": row.get::<Option<String>>(2).ok().flatten(),
                    "currency": row.get::<Option<String>>(3).ok().flatten(),
                    "trading_experience_level": row.get::<Option<String>>(4).ok().flatten(),
                    "primary_trading_goal": row.get::<Option<String>>(5).ok().flatten(),
                    "asset_types": row.get::<Option<String>>(6).ok().flatten(),
                    "trading_style": row.get::<Option<String>>(7).ok().flatten(),
                    "profile_picture_uuid": row.get::<Option<String>>(8).ok().flatten(),
                });

                Ok(HttpResponse::Ok().json(serde_json::json!({
                    "success": true,
                    "profile": profile
                })))
            } else {
                // Profile doesn't exist yet - return empty profile
                Ok(HttpResponse::Ok().json(serde_json::json!({
                    "success": true,
                    "profile": {
                        "nickname": null,
                        "display_name": null,
                        "timezone": null,
                        "currency": null,
                        "trading_experience_level": null,
                        "primary_trading_goal": null,
                        "asset_types": null,
                        "trading_style": null,
                        "profile_picture_uuid": null,
                    }
                })))
            }
        }
        Ok(None) => {
            info!("No database found for user: {}", user_id);
            Ok(HttpResponse::NotFound().json(serde_json::json!({
                "success": false,
                "error": "User database not found. Please initialize your database first."
            })))
        }
        Err(e) => {
            error!("Error getting profile for user {}: {}", user_id, e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Failed to get profile"
            })))
        }
    }
}

/// Update user profile
pub async fn update_profile(
    req: HttpRequest,
    user_id: web::Path<String>,
    payload: web::Json<UpdateProfileRequest>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> Result<HttpResponse> {
    info!("Updating profile for user: {}", user_id);

    // Get authenticated user
    let claims = get_authenticated_user(&req, &supabase_config).await?;

    // Validate that the authenticated user matches the requested user_id
    if claims.sub != *user_id {
        warn!(
            "User ID mismatch: JWT sub={}, requested user_id={}",
            claims.sub, user_id
        );
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "success": false,
            "error": "You can only update your own profile"
        })));
    }

    // Get user database connection
    match turso_client.get_user_database_connection(&user_id).await {
        Ok(Some(conn)) => {
            // Check if at least one field is provided
            let has_fields = payload.nickname.is_some()
                || payload.display_name.is_some()
                || payload.timezone.is_some()
                || payload.currency.is_some()
                || payload.trading_experience_level.is_some()
                || payload.primary_trading_goal.is_some()
                || payload.asset_types.is_some()
                || payload.trading_style.is_some();

            if !has_fields {
                return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                    "success": false,
                    "error": "No fields to update"
                })));
            }

            // Use INSERT OR REPLACE pattern - simpler approach
            // First check if profile exists
            let check_stmt = conn
                .prepare("SELECT COUNT(*) FROM user_profile")
                .await
                .map_err(|e| {
                    error!("Failed to prepare check statement: {}", e);
                    actix_web::error::ErrorInternalServerError("Database error")
                })?;

            let mut rows = check_stmt.query(libsql::params![]).await.map_err(|e| {
                error!("Failed to query profile count: {}", e);
                actix_web::error::ErrorInternalServerError("Database error")
            })?;

            let profile_exists = if let Some(row) = rows.next().await.map_err(|e| {
                error!("Failed to read profile count: {}", e);
                actix_web::error::ErrorInternalServerError("Database error")
            })? {
                row.get::<i64>(0).unwrap_or(0) > 0
            } else {
                false
            };

            if !profile_exists {
                // Check storage quota before creating new profile
                let app_state = req.app_data::<web::Data<AppState>>().ok_or_else(|| {
                    actix_web::error::ErrorInternalServerError("AppState not found")
                })?;

                app_state
                    .storage_quota_service
                    .check_storage_quota(&user_id, &conn)
                    .await
                    .map_err(|e| {
                        error!("Storage quota check failed for user {}: {}", user_id, e);
                        e
                    })?;

                // Insert new profile with provided values
                info!("Inserting new profile");
                conn.execute(
                    r#"
                    INSERT INTO user_profile (nickname, display_name, timezone, currency, trading_experience_level, primary_trading_goal, asset_types, trading_style)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                    libsql::params![
                        payload.nickname.as_deref(),
                        payload.display_name.as_deref(),
                        payload.timezone.as_deref().unwrap_or("UTC"),
                        payload.currency.as_deref().unwrap_or("USD"),
                        payload.trading_experience_level.as_deref(),
                        payload.primary_trading_goal.as_deref(),
                        payload.asset_types.as_deref(),
                        payload.trading_style.as_deref(),
                    ]
                ).await.map_err(|e| {
                    error!("Failed to insert profile: {}", e);
                    let error_msg = if e.to_string().contains("no column named") {
                        format!("Database schema error: {}. Please try again or contact support if the issue persists.", e)
                    } else {
                        format!("Database error: {}", e)
                    };
                    actix_web::error::ErrorInternalServerError(error_msg)
                })?;
            } else {
                // Update existing profile - update each field individually if provided
                info!("Updating existing profile");
                if let Some(ref v) = payload.nickname {
                    conn.execute(
                        "UPDATE user_profile SET nickname = ?, updated_at = CURRENT_TIMESTAMP",
                        libsql::params![v.clone()],
                    )
                    .await
                    .ok();
                }
                if let Some(ref v) = payload.display_name {
                    conn.execute(
                        "UPDATE user_profile SET display_name = ?, updated_at = CURRENT_TIMESTAMP",
                        libsql::params![v.clone()],
                    )
                    .await
                    .ok();
                }
                if let Some(ref v) = payload.timezone {
                    conn.execute(
                        "UPDATE user_profile SET timezone = ?, updated_at = CURRENT_TIMESTAMP",
                        libsql::params![v.clone()],
                    )
                    .await
                    .ok();
                }
                if let Some(ref v) = payload.currency {
                    conn.execute(
                        "UPDATE user_profile SET currency = ?, updated_at = CURRENT_TIMESTAMP",
                        libsql::params![v.clone()],
                    )
                    .await
                    .ok();
                }
                if let Some(ref v) = payload.trading_experience_level {
                    conn.execute("UPDATE user_profile SET trading_experience_level = ?, updated_at = CURRENT_TIMESTAMP", libsql::params![v.clone()]).await.ok();
                }
                if let Some(ref v) = payload.primary_trading_goal {
                    conn.execute("UPDATE user_profile SET primary_trading_goal = ?, updated_at = CURRENT_TIMESTAMP", libsql::params![v.clone()]).await.ok();
                }
                if let Some(ref v) = payload.asset_types {
                    conn.execute(
                        "UPDATE user_profile SET asset_types = ?, updated_at = CURRENT_TIMESTAMP",
                        libsql::params![v.clone()],
                    )
                    .await
                    .ok();
                }
                if let Some(ref v) = payload.trading_style {
                    conn.execute(
                        "UPDATE user_profile SET trading_style = ?, updated_at = CURRENT_TIMESTAMP",
                        libsql::params![v.clone()],
                    )
                    .await
                    .ok();
                }
            }

            info!("Profile updated successfully for user: {}", user_id);
            Ok(HttpResponse::Ok().json(ProfileResponse {
                success: true,
                message: "Profile updated successfully".to_string(),
            }))
        }
        Ok(None) => {
            info!("No database found for user: {}", user_id);
            Ok(HttpResponse::NotFound().json(serde_json::json!({
                "success": false,
                "error": "User database not found. Please initialize your database first."
            })))
        }
        Err(e) => {
            error!("Error updating profile for user {}: {}", user_id, e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Failed to update profile"
            })))
        }
    }
}

/// Upload profile picture
pub async fn upload_profile_picture(
    req: HttpRequest,
    user_id: web::Path<String>,
    _turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
    _payload: Multipart,
) -> Result<HttpResponse> {
    info!("Uploading profile picture for user: {}", user_id);

    // Get authenticated user
    let claims = get_authenticated_user(&req, &supabase_config).await?;

    // Validate that the authenticated user matches the requested user_id
    if claims.sub != *user_id {
        warn!(
            "User ID mismatch: JWT sub={}, requested user_id={}",
            claims.sub, user_id
        );
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "success": false,
            "error": "You can only upload your own profile picture"
        })));
    }

    // Image upload has been removed; return a clear response.
    Ok(HttpResponse::NotImplemented().json(serde_json::json!({
        "success": false,
        "error": "Profile picture upload is currently disabled"
    })))
}

/// Get storage usage for authenticated user
pub async fn get_storage_usage(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
) -> Result<HttpResponse> {
    info!("Getting storage usage for user");

    // Get authenticated user
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = &claims.sub;

    // Get user's database connection
    let conn = app_state
        .get_user_db_connection(user_id)
        .await
        .map_err(|e| {
            error!(
                "Failed to get database connection for user {}: {}",
                user_id, e
            );
            actix_web::error::ErrorInternalServerError("Database connection failed")
        })?
        .ok_or_else(|| {
            error!("No database found for user: {}", user_id);
            actix_web::error::ErrorNotFound("User database not found")
        })?;

    // Get storage usage from quota service
    match app_state
        .storage_quota_service
        .get_storage_usage(user_id, &conn)
        .await
    {
        Ok(usage) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "data": {
                "used_bytes": usage.used_bytes,
                "limit_bytes": usage.limit_bytes,
                "used_mb": usage.used_mb,
                "limit_mb": usage.limit_mb,
                "remaining_bytes": usage.remaining_bytes,
                "remaining_mb": usage.remaining_mb,
                "percentage_used": usage.percentage_used
            }
        }))),
        Err(e) => {
            error!("Failed to get storage usage for user {}: {}", user_id, e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Failed to get storage usage"
            })))
        }
    }
}

/// Delete user account (irreversible)
/// This deletes all user data including Turso database, Supabase Storage, vectors, and auth account
pub async fn delete_account(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
) -> Result<HttpResponse> {
    info!("Account deletion requested");

    // Get authenticated user
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = &claims.sub;

    info!("Deleting account for user: {}", user_id);

    // Verify user_id matches authenticated user (security check)
    if claims.sub != *user_id {
        error!(
            "User ID mismatch in account deletion: {} != {}",
            claims.sub, user_id
        );
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "success": false,
            "error": "Unauthorized: Cannot delete another user's account"
        })));
    }

    // Delete user account (all-or-nothing transaction)
    match app_state
        .account_deletion_service
        .delete_user_account(user_id)
        .await
    {
        Ok(_) => {
            info!("Successfully deleted account for user: {}", user_id);
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "message": "Account deleted successfully"
            })))
        }
        Err(e) => {
            error!("Failed to delete account for user {}: {}", user_id, e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": format!("Failed to delete account: {}", e)
            })))
        }
    }
}

/// Configure user routes
pub fn configure_user_routes(cfg: &mut web::ServiceConfig) {
    info!("Setting up /api/user routes");
    cfg.service(
        web::scope("/api/user")
            .route("/test", web::get().to(test_endpoint))
            .route("/initialize", web::post().to(initialize_user_database))
            .route("/check/{user_id}", web::get().to(check_user_database))
            .route(
                "/database-info/{user_id}",
                web::get().to(get_user_database_info),
            )
            .route("/sync-schema/{user_id}", web::post().to(sync_user_schema))
            .route(
                "/schema-version/{user_id}",
                web::get().to(get_user_schema_version),
            )
            .route("/profile/{user_id}", web::get().to(get_profile))
            .route("/profile/{user_id}", web::put().to(update_profile))
            .route(
                "/profile/picture/{user_id}",
                web::post().to(upload_profile_picture),
            )
            .route("/storage", web::get().to(get_storage_usage))
            .route("/account", web::delete().to(delete_account)),
    );
}
