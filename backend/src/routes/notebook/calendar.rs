use actix_web::{HttpRequest, HttpResponse, Result, web};
use chrono::{DateTime, Utc};
use log::{error, info};
use serde::{Deserialize, Serialize};

use crate::models::notebook::{
    CalendarConnection, CalendarEvent, CreateEventRequest, EventQuery, UpdateEventRequest,
};
use crate::service::notebook_engine::calendar::CalendarSyncService;
use crate::turso::auth::validate_supabase_jwt_token;
use crate::turso::config::SupabaseConfig;
use crate::turso::{AppState, SupabaseClaims};

fn extract_token_from_request(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("authorization")?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(|s| s.to_string())
}

async fn get_authenticated_user(
    req: &HttpRequest,
    supabase_config: &SupabaseConfig,
) -> Result<SupabaseClaims, actix_web::Error> {
    let token = extract_token_from_request(req)
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("Missing authorization token"))?;

    validate_supabase_jwt_token(&token, supabase_config)
        .await
        .map_err(|e| {
            error!("[Calendar] JWT validation failed: {}", e);
            actix_web::error::ErrorUnauthorized("Invalid or expired token")
        })
}

// ==================== Response Types ====================

#[derive(Debug, Serialize)]
pub struct AuthUrlResponse {
    pub success: bool,
    pub url: Option<String>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ConnectionResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<CalendarConnection>,
}

#[derive(Debug, Serialize)]
pub struct ConnectionListResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<Vec<CalendarConnection>>,
}

#[derive(Debug, Serialize)]
pub struct EventResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<CalendarEvent>,
}

#[derive(Debug, Serialize)]
pub struct EventListResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<Vec<CalendarEvent>>,
}

#[derive(Debug, Serialize)]
pub struct SyncResponse {
    pub success: bool,
    pub message: String,
    pub synced_connections: usize,
    pub failed_connections: usize,
    pub synced_events: usize,
}

// ==================== Request Types ====================

#[derive(Debug, Deserialize)]
pub struct OAuthCallbackRequest {
    pub code: String,
    pub state: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ToggleConnectionRequest {
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct EventQueryParams {
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub source: Option<String>,
    pub connection_id: Option<String>,
    pub is_reminder: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ==================== OAuth Handlers ====================

/// Get Google OAuth authorization URL
pub async fn get_auth_url(req: HttpRequest, supabase_config: web::Data<SupabaseConfig>) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    
    let calendar_service = match CalendarSyncService::from_env() {
        Ok(s) => s,
        Err(e) => {
            error!("[Calendar] Failed to create calendar service: {}", e);
            return Ok(HttpResponse::InternalServerError().json(AuthUrlResponse {
                success: false,
                url: None,
                message: "Calendar service not configured".to_string(),
            }));
        }
    };

    // Use user_id as state for security
    match calendar_service.get_auth_url(&claims.sub) {
        Ok(url) => Ok(HttpResponse::Ok().json(AuthUrlResponse {
            success: true,
            url: Some(url),
            message: "Authorization URL generated".to_string(),
        })),
        Err(e) => {
            error!("[Calendar] Failed to generate auth URL: {}", e);
            Ok(HttpResponse::InternalServerError().json(AuthUrlResponse {
                success: false,
                url: None,
                message: format!("Failed to generate auth URL: {}", e),
            }))
        }
    }
}

/// Handle OAuth callback
pub async fn oauth_callback(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    payload: web::Json<OAuthCallbackRequest>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    let calendar_service = match CalendarSyncService::from_env() {
        Ok(s) => s,
        Err(e) => {
            error!("[Calendar] Failed to create calendar service: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ConnectionResponse {
                success: false,
                message: "Calendar service not configured".to_string(),
                data: None,
            }));
        }
    };

    match calendar_service.handle_oauth_callback(&conn, &payload.code).await {
        Ok(connection) => {
            info!("[Calendar] OAuth callback successful for user {}", claims.sub);
            Ok(HttpResponse::Ok().json(ConnectionResponse {
                success: true,
                message: "Calendar connected successfully".to_string(),
                data: Some(connection),
            }))
        }
        Err(e) => {
            error!("[Calendar] OAuth callback failed: {}", e);
            Ok(HttpResponse::BadRequest().json(ConnectionResponse {
                success: false,
                message: format!("Failed to connect calendar: {}", e),
                data: None,
            }))
        }
    }
}

// ==================== Connection Handlers ====================

/// Get all calendar connections
pub async fn get_connections(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    match CalendarConnection::find_all(&conn).await {
        Ok(connections) => Ok(HttpResponse::Ok().json(ConnectionListResponse {
            success: true,
            message: format!("Retrieved {} connections", connections.len()),
            data: Some(connections),
        })),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ConnectionListResponse {
            success: false,
            message: format!("Failed to get connections: {}", e),
            data: None,
        })),
    }
}

/// Toggle connection enabled status
pub async fn toggle_connection(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<String>,
    payload: web::Json<ToggleConnectionRequest>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let connection_id = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    let calendar_service = match CalendarSyncService::from_env() {
        Ok(s) => s,
        Err(e) => {
            error!("[Calendar] Failed to create calendar service: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ConnectionResponse {
                success: false,
                message: "Calendar service not configured".to_string(),
                data: None,
            }));
        }
    };

    match calendar_service.toggle_connection(&conn, &connection_id, payload.enabled).await {
        Ok(connection) => Ok(HttpResponse::Ok().json(ConnectionResponse {
            success: true,
            message: format!("Connection {}", if payload.enabled { "enabled" } else { "disabled" }),
            data: Some(connection),
        })),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ConnectionResponse {
            success: false,
            message: format!("Failed to toggle connection: {}", e),
            data: None,
        })),
    }
}

/// Delete a calendar connection
pub async fn delete_connection(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<String>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let connection_id = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    let calendar_service = match CalendarSyncService::from_env() {
        Ok(s) => s,
        Err(e) => {
            error!("[Calendar] Failed to create calendar service: {}", e);
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "message": "Calendar service not configured"
            })));
        }
    };

    match calendar_service.delete_connection(&conn, &connection_id).await {
        Ok(true) => {
            info!("[Calendar] Connection {} deleted", connection_id);
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "message": "Connection deleted"
            })))
        }
        Ok(false) => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "message": "Connection not found"
        }))),
        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "message": format!("Failed to delete connection: {}", e)
        }))),
    }
}

// ==================== Sync Handlers ====================

/// Sync all enabled connections
pub async fn sync_all(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    let calendar_service = match CalendarSyncService::from_env() {
        Ok(s) => s,
        Err(e) => {
            error!("[Calendar] Failed to create calendar service: {}", e);
            return Ok(HttpResponse::InternalServerError().json(SyncResponse {
                success: false,
                message: "Calendar service not configured".to_string(),
                synced_connections: 0,
                failed_connections: 0,
                synced_events: 0,
            }));
        }
    };

    match calendar_service.sync_all(&conn).await {
        Ok(result) => {
            info!("[Calendar] Sync completed: {} connections, {} events", result.synced_connections, result.synced_events);
            Ok(HttpResponse::Ok().json(SyncResponse {
                success: true,
                message: "Sync completed".to_string(),
                synced_connections: result.synced_connections,
                failed_connections: result.failed_connections,
                synced_events: result.synced_events,
            }))
        }
        Err(e) => {
            error!("[Calendar] Sync failed: {}", e);
            Ok(HttpResponse::InternalServerError().json(SyncResponse {
                success: false,
                message: format!("Sync failed: {}", e),
                synced_connections: 0,
                failed_connections: 0,
                synced_events: 0,
            }))
        }
    }
}

/// Sync a specific connection
pub async fn sync_connection(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<String>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let connection_id = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    let calendar_service = match CalendarSyncService::from_env() {
        Ok(s) => s,
        Err(e) => {
            error!("[Calendar] Failed to create calendar service: {}", e);
            return Ok(HttpResponse::InternalServerError().json(SyncResponse {
                success: false,
                message: "Calendar service not configured".to_string(),
                synced_connections: 0,
                failed_connections: 0,
                synced_events: 0,
            }));
        }
    };

    // Get the connection first
    let connection = match CalendarConnection::find_by_id(&conn, &connection_id).await {
        Ok(c) => c,
        Err(_) => {
            return Ok(HttpResponse::NotFound().json(SyncResponse {
                success: false,
                message: "Connection not found".to_string(),
                synced_connections: 0,
                failed_connections: 0,
                synced_events: 0,
            }));
        }
    };

    match calendar_service.sync_connection(&conn, &connection).await {
        Ok(count) => {
            info!("[Calendar] Synced {} events for connection {}", count, connection_id);
            Ok(HttpResponse::Ok().json(SyncResponse {
                success: true,
                message: format!("Synced {} events", count),
                synced_connections: 1,
                failed_connections: 0,
                synced_events: count,
            }))
        }
        Err(e) => {
            error!("[Calendar] Sync failed for connection {}: {}", connection_id, e);
            Ok(HttpResponse::InternalServerError().json(SyncResponse {
                success: false,
                message: format!("Sync failed: {}", e),
                synced_connections: 0,
                failed_connections: 1,
                synced_events: 0,
            }))
        }
    }
}

// ==================== Event Handlers ====================

/// Get all events with optional filters
pub async fn get_events(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    query: web::Query<EventQueryParams>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    let event_query = EventQuery {
        start_time: query.start_time,
        end_time: query.end_time,
        source: query.source.as_ref().and_then(|s| crate::models::notebook::EventSource::from_str(s).ok()),
        connection_id: query.connection_id.clone(),
        is_reminder: query.is_reminder,
        status: None,
        limit: query.limit,
        offset: query.offset,
    };

    match CalendarEvent::find_all(&conn, Some(event_query)).await {
        Ok(events) => Ok(HttpResponse::Ok().json(EventListResponse {
            success: true,
            message: format!("Retrieved {} events", events.len()),
            data: Some(events),
        })),
        Err(e) => Ok(HttpResponse::InternalServerError().json(EventListResponse {
            success: false,
            message: format!("Failed to get events: {}", e),
            data: None,
        })),
    }
}

/// Get a single event by ID
pub async fn get_event(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<String>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let event_id = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    match CalendarEvent::find_by_id(&conn, &event_id).await {
        Ok(event) => Ok(HttpResponse::Ok().json(EventResponse {
            success: true,
            message: "Event retrieved".to_string(),
            data: Some(event),
        })),
        Err(e) => Ok(HttpResponse::NotFound().json(EventResponse {
            success: false,
            message: format!("Event not found: {}", e),
            data: None,
        })),
    }
}

/// Create a local event/reminder
pub async fn create_event(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    payload: web::Json<CreateEventRequest>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    match CalendarEvent::create(&conn, payload.into_inner()).await {
        Ok(event) => {
            info!("[Calendar] Event created: {}", event.id);
            Ok(HttpResponse::Created().json(EventResponse {
                success: true,
                message: "Event created".to_string(),
                data: Some(event),
            }))
        }
        Err(e) => {
            error!("[Calendar] Failed to create event: {}", e);
            Ok(HttpResponse::InternalServerError().json(EventResponse {
                success: false,
                message: format!("Failed to create event: {}", e),
                data: None,
            }))
        }
    }
}

/// Update an event
pub async fn update_event(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<String>,
    payload: web::Json<UpdateEventRequest>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let event_id = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    match CalendarEvent::update(&conn, &event_id, payload.into_inner()).await {
        Ok(event) => Ok(HttpResponse::Ok().json(EventResponse {
            success: true,
            message: "Event updated".to_string(),
            data: Some(event),
        })),
        Err(e) => Ok(HttpResponse::InternalServerError().json(EventResponse {
            success: false,
            message: format!("Failed to update event: {}", e),
            data: None,
        })),
    }
}

/// Delete an event
pub async fn delete_event(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<String>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let event_id = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    match CalendarEvent::delete(&conn, &event_id).await {
        Ok(true) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Event deleted"
        }))),
        Ok(false) => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "message": "Event not found"
        }))),
        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "message": format!("Failed to delete event: {}", e)
        }))),
    }
}

// ==================== Route Configuration ====================

pub fn configure_calendar_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/notebook/calendar")
            // OAuth routes
            .route("/auth/url", web::get().to(get_auth_url))
            .route("/auth/callback", web::post().to(oauth_callback))
            // Connection routes
            .route("/connections", web::get().to(get_connections))
            .route("/connections/{id}", web::patch().to(toggle_connection))
            .route("/connections/{id}", web::delete().to(delete_connection))
            // Sync routes
            .route("/sync", web::post().to(sync_all))
            .route("/sync/{id}", web::post().to(sync_connection))
            // Event routes
            .route("/events", web::get().to(get_events))
            .route("/events", web::post().to(create_event))
            .route("/events/{id}", web::get().to(get_event))
            .route("/events/{id}", web::put().to(update_event))
            .route("/events/{id}", web::delete().to(delete_event)),
    );
}
