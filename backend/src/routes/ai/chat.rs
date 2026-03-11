#![allow(dead_code)]

use crate::models::ai::chat::ChatRequest;
use crate::turso::AppState;
use crate::turso::auth::validate_supabase_jwt_token;
use crate::turso::config::SupabaseConfig;
use crate::websocket::{ConnectionManager, broadcast_chat_message, broadcast_chat_session};
use actix_web::{HttpRequest, HttpResponse, Result, web};
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Authenticate user and get user ID
async fn get_authenticated_user(
    req: &HttpRequest,
    supabase_config: &SupabaseConfig,
) -> Result<String> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .ok_or_else(|| {
            error!("Missing Authorization header");
            actix_web::error::ErrorUnauthorized("Missing Authorization header")
        })?
        .to_str()
        .map_err(|e| {
            error!("Invalid Authorization header format: {}", e);
            actix_web::error::ErrorUnauthorized("Invalid Authorization header")
        })?;

    let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
        error!("Invalid token format - missing Bearer prefix");
        actix_web::error::ErrorUnauthorized("Invalid token format")
    })?;

    info!("Validating JWT token for user authentication");
    let claims = validate_supabase_jwt_token(token, supabase_config)
        .await
        .map_err(|e| {
            error!("JWT validation failed: {}", e);
            actix_web::error::ErrorUnauthorized("Invalid or expired authentication token")
        })?;

    info!("Successfully authenticated user: {}", claims.sub);
    Ok(claims.sub)
}

/// Get user's database connection with authentication
#[allow(dead_code)]
async fn get_user_database_connection(
    req: &HttpRequest,
    app_state: &AppState,
) -> Result<libsql::Connection> {
    let user_id = get_authenticated_user(req, &app_state.config.supabase).await?;

    let conn = app_state
        .turso_client
        .get_user_database_connection(&user_id)
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

    Ok(conn)
}
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

/// Chat request for streaming
#[derive(Debug, Deserialize)]
pub struct StreamingChatRequest {
    pub message: String,
    pub session_id: Option<String>,
    pub include_context: Option<bool>,
    pub max_context_vectors: Option<usize>,
}

/// Session list query parameters
#[derive(Debug, Deserialize)]
pub struct SessionListQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// Update session title request
#[derive(Debug, Deserialize)]
pub struct UpdateSessionTitleRequest {
    pub title: String,
}

/// Send a chat message and get response
pub async fn send_chat_message(
    req: HttpRequest,
    payload: web::Json<ChatRequest>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse> {
    info!("Processing chat message");

    let conn = get_user_database_connection(&req, &app_state).await?;
    let user_id = get_authenticated_user(&req, &app_state.config.supabase).await?;

    let request = payload.into_inner();
    let use_tools = request.use_tools.unwrap_or(false);

    // Choose between tool-enabled or regular chat
    let result = if use_tools {
        info!("Using tool-enabled chat for user: {}", user_id);
        app_state
            .ai_chat_service
            .generate_response_with_tools(&user_id, request, &conn)
            .await
    } else {
        app_state
            .ai_chat_service
            .generate_response(&user_id, request, &conn)
            .await
    };

    match result {
        Ok(response) => {
            info!("Successfully generated chat response for user: {}", user_id);
            Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
        }
        Err(e) => {
            error!(
                "Failed to generate chat response for user {}: {}",
                user_id, e
            );
            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to generate chat response".to_string(),
                )),
            )
        }
    }
}

/// Send a streaming chat message
pub async fn send_streaming_chat_message(
    req: HttpRequest,
    payload: web::Json<StreamingChatRequest>,
    app_state: web::Data<AppState>,
    ws_manager: web::Data<Arc<Mutex<ConnectionManager>>>,
) -> Result<HttpResponse> {
    info!("Processing streaming chat message");

    let conn = get_user_database_connection(&req, &app_state).await?;
    let user_id = get_authenticated_user(&req, &app_state.config.supabase).await?;

    let chat_request = ChatRequest {
        message: payload.message.clone(),
        session_id: payload.session_id.clone(),
        include_context: payload.include_context,
        max_context_vectors: payload.max_context_vectors,
        use_tools: Some(true), // Enable tool usage for chat
    };

    // Use streaming with tools - executes tool calls first, then streams final response
    match app_state
        .ai_chat_service
        .generate_streaming_response_with_tools(&user_id, chat_request, &conn)
        .await
    {
        Ok((stream_receiver, session_id, message_id)) => {
            info!(
                "Successfully started streaming chat response for user: {}",
                user_id
            );

            // Broadcast message created event via WebSocket
            let message_data = serde_json::json!({
                "id": message_id,
                "session_id": session_id,
                "role": "user",
            });
            broadcast_chat_message(ws_manager.clone(), &user_id, "created", message_data).await;

            // Create streaming response with proper JSON format
            let ws_manager_clone = ws_manager.clone();
            let user_id_clone = user_id.clone();
            let message_id_clone = message_id.clone();

            let stream = futures_util::stream::unfold(
                (stream_receiver, false),
                move |(mut receiver, completed)| {
                    let ws_manager = ws_manager_clone.clone();
                    let user_id = user_id_clone.clone();
                    let message_id = message_id_clone.clone();

                    async move {
                        if completed {
                            return None;
                        }

                        match receiver.recv().await {
                            Some(token) => {
                                // Send newline-delimited JSON format that frontend expects
                                let chunk = format!(
                                    "{{\"chunk\":\"{}\"}}\n",
                                    token.replace("\"", "\\\"").replace("\n", "\\n")
                                );
                                Some((
                                    Ok::<web::Bytes, std::io::Error>(web::Bytes::from(chunk)),
                                    (receiver, false),
                                ))
                            }
                            None => {
                                // Stream completed - broadcast assistant message created
                                let assistant_message_data = serde_json::json!({
                                    "id": message_id,
                                    "role": "assistant",
                                });
                                broadcast_chat_message(
                                    ws_manager,
                                    &user_id,
                                    "created",
                                    assistant_message_data,
                                )
                                .await;
                                let _ = completed; // Mark as used
                                None
                            }
                        }
                    }
                },
            );

            Ok(HttpResponse::Ok()
                .content_type("application/x-ndjson")
                .append_header(("Cache-Control", "no-cache"))
                .append_header(("Connection", "keep-alive"))
                .streaming(stream))
        }
        Err(e) => {
            error!(
                "Failed to generate streaming chat response for user {}: {}",
                user_id, e
            );
            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to generate streaming chat response".to_string(),
                )),
            )
        }
    }
}

/// Get user's chat sessions
pub async fn get_chat_sessions(
    req: HttpRequest,
    query: web::Query<SessionListQuery>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse> {
    info!("Getting chat sessions for user");

    let conn = get_user_database_connection(&req, &app_state).await?;
    let user_id = get_authenticated_user(&req, &app_state.config.supabase).await?;

    match app_state
        .ai_chat_service
        .get_user_sessions(&conn, &user_id, query.limit, query.offset)
        .await
    {
        Ok(response) => {
            info!(
                "Successfully retrieved {} chat sessions for user: {}",
                response.total_count, user_id
            );
            Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
        }
        Err(e) => {
            error!("Failed to get chat sessions for user {}: {}", user_id, e);
            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to get chat sessions".to_string(),
                )),
            )
        }
    }
}

/// Get specific chat session with messages
pub async fn get_chat_session(
    req: HttpRequest,
    path: web::Path<String>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let session_id = path.into_inner();
    info!("Getting chat session: {}", session_id);

    let conn = get_user_database_connection(&req, &app_state).await?;
    let user_id = get_authenticated_user(&req, &app_state.config.supabase).await?;

    match app_state
        .ai_chat_service
        .get_session_details(&conn, &session_id, &user_id)
        .await
    {
        Ok(response) => {
            info!(
                "Successfully retrieved chat session {} for user: {}",
                session_id, user_id
            );
            Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
        }
        Err(e) => {
            error!(
                "Failed to get chat session {} for user {}: {}",
                session_id, user_id, e
            );
            Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error(
                "Chat session not found".to_string(),
            )))
        }
    }
}

/// Create a new chat session
pub async fn create_chat_session(
    req: HttpRequest,
    payload: web::Json<serde_json::Value>,
    app_state: web::Data<AppState>,
    ws_manager: web::Data<Arc<Mutex<ConnectionManager>>>,
) -> Result<HttpResponse> {
    info!("Creating new chat session");

    let conn = get_user_database_connection(&req, &app_state).await?;
    let user_id = get_authenticated_user(&req, &app_state.config.supabase).await?;

    // Check storage quota before creating
    app_state
        .storage_quota_service
        .check_storage_quota(&user_id, &conn)
        .await
        .map_err(|e| {
            error!("Storage quota check failed for user {}: {}", user_id, e);
            e
        })?;

    let title = payload
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    match app_state
        .ai_chat_service
        .create_session(&conn, &user_id, title)
        .await
    {
        Ok(session) => {
            info!(
                "Successfully created chat session {} for user: {}",
                session.id, user_id
            );

            // Broadcast session created event via WebSocket
            broadcast_chat_session(ws_manager, &user_id, "created", &session).await;

            Ok(HttpResponse::Created().json(ApiResponse::success(session)))
        }
        Err(e) => {
            error!("Failed to create chat session for user {}: {}", user_id, e);
            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to create chat session".to_string(),
                )),
            )
        }
    }
}

/// Update chat session title
pub async fn update_chat_session_title(
    req: HttpRequest,
    path: web::Path<String>,
    payload: web::Json<UpdateSessionTitleRequest>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let session_id = path.into_inner();
    info!("Updating chat session title: {}", session_id);

    let conn = get_user_database_connection(&req, &app_state).await?;
    let user_id = get_authenticated_user(&req, &app_state.config.supabase).await?;

    match app_state
        .ai_chat_service
        .update_session_title(&conn, &session_id, &user_id, payload.title.clone())
        .await
    {
        Ok(_) => {
            info!(
                "Successfully updated chat session title {} for user: {}",
                session_id, user_id
            );
            Ok(
                HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
                    "success": true,
                    "message": "Session title updated successfully"
                }))),
            )
        }
        Err(e) => {
            error!(
                "Failed to update chat session title {} for user {}: {}",
                session_id, user_id, e
            );
            Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error(
                "Chat session not found".to_string(),
            )))
        }
    }
}

/// Delete a chat session
pub async fn delete_chat_session(
    req: HttpRequest,
    path: web::Path<String>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let session_id = path.into_inner();
    info!("Deleting chat session: {}", session_id);

    let conn = get_user_database_connection(&req, &app_state).await?;
    let user_id = get_authenticated_user(&req, &app_state.config.supabase).await?;

    match app_state
        .ai_chat_service
        .delete_session(&conn, &session_id, &user_id)
        .await
    {
        Ok(_) => {
            info!(
                "Successfully deleted chat session {} for user: {}",
                session_id, user_id
            );
            Ok(
                HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
                    "success": true,
                    "message": "Session deleted successfully"
                }))),
            )
        }
        Err(e) => {
            error!(
                "Failed to delete chat session {} for user {}: {}",
                session_id, user_id, e
            );
            Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error(
                "Chat session not found".to_string(),
            )))
        }
    }
}

/// Fix message counts for all chat sessions
async fn fix_message_counts(
    req: HttpRequest,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse> {
    info!("Fixing message counts for all chat sessions");

    let conn = get_user_database_connection(&req, &app_state).await?;

    match app_state.ai_chat_service.fix_message_counts(&conn).await {
        Ok(updated_count) => {
            info!(
                "Successfully fixed message counts - {} sessions updated",
                updated_count
            );
            Ok(
                HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
                    "success": true,
                    "message": format!("Message counts fixed for {} sessions", updated_count),
                    "updated_sessions": updated_count
                }))),
            )
        }
        Err(e) => {
            error!("Failed to fix message counts: {}", e);
            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to fix message counts".to_string(),
                )),
            )
        }
    }
}

/// Configure AI chat routes
pub fn configure_ai_chat_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/ai/chat")
            .route("", web::post().to(send_chat_message))
            .route("/stream", web::post().to(send_streaming_chat_message))
            .route("/sessions", web::get().to(get_chat_sessions))
            .route("/sessions", web::post().to(create_chat_session))
            .route("/sessions/{id}", web::get().to(get_chat_session))
            .route(
                "/sessions/{id}/title",
                web::put().to(update_chat_session_title),
            )
            .route("/sessions/{id}", web::delete().to(delete_chat_session))
            .route("/fix-message-counts", web::post().to(fix_message_counts)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_success() {
        let response = ApiResponse::success("test data");
        assert!(response.success);
        assert_eq!(response.data, Some("test data"));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_api_response_error() {
        let response = ApiResponse::<()>::error("test error".to_string());
        assert!(!response.success);
        assert!(response.data.is_none());
        assert_eq!(response.error, Some("test error".to_string()));
    }
}
