use crate::models::ai::reports::{ReportRequest, SaveReportRequest};
use crate::turso::{AppState, config::SupabaseConfig};
use crate::websocket::ConnectionManager;
use actix_web::{HttpRequest, HttpResponse, Result, web};
use actix_web_httpauth::middleware::HttpAuthentication;
use log::{error, info};
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

// Import jwt_validator from main module and rate limit middleware
use crate::jwt_validator;
use crate::middleware::rate_limit::rate_limit_middleware;

/// Authenticate user and get user ID
async fn get_authenticated_user(
    req: &HttpRequest,
    supabase_config: &SupabaseConfig,
) -> Result<String> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("Missing Authorization header"))?
        .to_str()
        .map_err(|_| actix_web::error::ErrorUnauthorized("Invalid Authorization header"))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("Invalid token format"))?;

    let claims = crate::turso::auth::validate_supabase_jwt_token(token, supabase_config)
        .await
        .map_err(|e| {
            error!("JWT validation failed: {}", e);
            actix_web::error::ErrorUnauthorized("Invalid or expired authentication token")
        })?;

    Ok(claims.sub)
}

/// Get user's database connection with authentication
async fn get_user_database_connection(
    req: &HttpRequest,
    turso_client: &crate::turso::client::TursoClient,
    supabase_config: &SupabaseConfig,
) -> Result<libsql::Connection> {
    let user_id = get_authenticated_user(req, supabase_config).await?;

    let conn = turso_client
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

/// Generate a comprehensive trading report
pub async fn generate_report(
    req: HttpRequest,
    report_request: web::Json<ReportRequest>,
    app_state: web::Data<AppState>,
    ws_manager: web::Data<Arc<Mutex<ConnectionManager>>>,
) -> Result<HttpResponse> {
    info!("Generating AI report");

    let conn =
        get_user_database_connection(&req, &app_state.turso_client, &app_state.config.supabase)
            .await?;
    let user_id = get_authenticated_user(&req, &app_state.config.supabase).await?;

    match app_state
        .ai_reports_service
        .generate_report(
            &conn,
            &user_id,
            report_request.into_inner(),
            Some(ws_manager.get_ref().clone()),
        )
        .await
    {
        Ok(reports) => {
            info!(
                "Successfully generated {} reports for user: {}",
                reports.len(),
                user_id
            );
            Ok(HttpResponse::Created().json(ApiResponse::success(reports)))
        }
        Err(e) => {
            error!("Failed to generate report for user {}: {}", user_id, e);
            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to generate report".to_string(),
                )),
            )
        }
    }
}

/// Persist a streamed report payload to the database
pub async fn save_report(
    req: HttpRequest,
    payload: web::Json<SaveReportRequest>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse> {
    info!("Saving streamed AI report");

    let conn =
        get_user_database_connection(&req, &app_state.turso_client, &app_state.config.supabase)
            .await?;
    let user_id = get_authenticated_user(&req, &app_state.config.supabase).await?;

    match app_state
        .ai_reports_service
        .save_streamed_report(&conn, &user_id, payload.into_inner())
        .await
    {
        Ok(id) => {
            info!("Saved AI report {} for user: {}", id, user_id);
            Ok(HttpResponse::Created().json(ApiResponse::success(json!({ "id": id }))))
        }
        Err(e) => {
            error!("Failed to save AI report for user {}: {}", user_id, e);
            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to save report".to_string(),
                )),
            )
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Fetch report history
pub async fn get_history(
    req: HttpRequest,
    query: web::Query<HistoryQuery>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse> {
    info!("Fetching AI report history");

    let conn =
        get_user_database_connection(&req, &app_state.turso_client, &app_state.config.supabase)
            .await?;
    let user_id = get_authenticated_user(&req, &app_state.config.supabase).await?;

    match app_state
        .ai_reports_service
        .get_history(&conn, query.limit, query.offset)
        .await
    {
        Ok(history) => {
            info!("Fetched {} reports for user: {}", history.len(), user_id);
            Ok(HttpResponse::Ok().json(ApiResponse::success(history)))
        }
        Err(e) => {
            error!("Failed to fetch report history for user {}: {}", user_id, e);
            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to fetch report history".to_string(),
                )),
            )
        }
    }
}

/// Fetch single report by ID
pub async fn get_history_by_id(
    req: HttpRequest,
    path: web::Path<String>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let report_id = path.into_inner();
    info!("Fetching AI report {}", report_id);

    let conn =
        get_user_database_connection(&req, &app_state.turso_client, &app_state.config.supabase)
            .await?;
    let user_id = get_authenticated_user(&req, &app_state.config.supabase).await?;

    match app_state
        .ai_reports_service
        .get_history_by_id(&conn, &report_id)
        .await
    {
        Ok(Some(report)) => {
            info!("Found report {} for user: {}", report_id, user_id);
            Ok(HttpResponse::Ok().json(ApiResponse::success(report)))
        }
        Ok(None) => {
            info!("Report {} not found for user: {}", report_id, user_id);
            Ok(HttpResponse::NotFound()
                .json(ApiResponse::<()>::error("Report not found".to_string())))
        }
        Err(e) => {
            error!(
                "Failed to fetch report {} for user {}: {}",
                report_id, user_id, e
            );
            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to fetch report".to_string(),
                )),
            )
        }
    }
}

/// Configure AI reports routes
pub fn configure_ai_reports_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/reports")
            .wrap(HttpAuthentication::bearer(jwt_validator))
            .wrap(actix_web::middleware::from_fn(rate_limit_middleware))
            .route("/generate", web::post().to(generate_report))
            .route("/save", web::post().to(save_report))
            .route("/history", web::get().to(get_history))
            .route("/history/{id}", web::get().to(get_history_by_id)),
    );
}

/// API Response wrapper
#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
        }
    }

    pub fn error(message: String) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            message: Some(message),
        }
    }
}
