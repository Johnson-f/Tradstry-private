use actix_web::{HttpRequest, HttpResponse, Result, web};
use log::{error, info};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::service::ai_service::model_connection::ModelSelector;
use crate::service::notebook_engine::ai::{NotebookAIRequest, NotebookAIService};
use crate::turso::auth::validate_supabase_jwt_token;
use crate::turso::config::SupabaseConfig;
use crate::turso::SupabaseClaims;
use crate::websocket::ConnectionManager;

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
            error!("[NotebookAI] JWT validation failed: {}", e);
            actix_web::error::ErrorUnauthorized("Invalid or expired token")
        })
}

#[derive(Debug, Serialize)]
pub struct AIResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
}

/// Process an AI request for notebook content
pub async fn process_ai_request(
    ws_manager: web::Data<Arc<Mutex<ConnectionManager>>>,
    model_selector: web::Data<Arc<Mutex<ModelSelector>>>,
    supabase_config: web::Data<SupabaseConfig>,
    payload: web::Json<NotebookAIRequest>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = &claims.sub;

    info!(
        "[NotebookAI] Processing {:?} request for user {}",
        payload.action, user_id
    );

    let ai_service = NotebookAIService::new(model_selector.get_ref().clone());
    let ws_manager_arc = ws_manager.get_ref().clone();

    // Spawn the AI processing task
    let request = payload.into_inner();
    let user_id_owned = user_id.clone();
    let _request_id = request.request_id.clone();

    tokio::spawn(async move {
        match ai_service
            .process_request(&user_id_owned, request, ws_manager_arc)
            .await
        {
            Ok(response) => {
                info!(
                    "[NotebookAI] Successfully processed request {} for user {}",
                    response.request_id, user_id_owned
                );
            }
            Err(e) => {
                error!(
                    "[NotebookAI] Failed to process request for user {}: {}",
                    user_id_owned, e
                );
            }
        }
    });

    // Return immediately with accepted status
    Ok(HttpResponse::Accepted().json(AIResponse::<()> {
        success: true,
        message: "AI request accepted, results will be streamed via WebSocket".to_string(),
        data: None,
    }))
}

/// Configure notebook AI routes
pub fn configure_notebook_ai_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/notebook/ai")
            .route("", web::post().to(process_ai_request))
            .route("/process", web::post().to(process_ai_request)),
    );
}
