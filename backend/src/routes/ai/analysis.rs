use crate::models::ai::analysis::{
    AnalysisError, AnalysisRecord, AnalysisRequest, SaveAnalysisRequest, SaveAnalysisResponse,
};
use crate::service::ai_service::interface::analysis_service::TradeAnalysisService;
use crate::turso::config::SupabaseConfig;
use crate::websocket::ConnectionManager;
use actix_web::{HttpRequest, HttpResponse, Result as ActixResult, web};
use log::error;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Authenticate user and get user ID
async fn get_authenticated_user(
    req: &HttpRequest,
    supabase_config: &SupabaseConfig,
) -> Result<String, actix_web::Error> {
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

/// POST /api/analysis/generate
/// Generate full trading analysis bundle based on period (no focus selection)
pub async fn generate_analysis(
    req: HttpRequest,
    request: web::Json<AnalysisRequest>,
    supabase_config: web::Data<SupabaseConfig>,
    analysis_service: web::Data<Arc<TradeAnalysisService>>,
    ws_manager: web::Data<Arc<Mutex<ConnectionManager>>>,
) -> ActixResult<HttpResponse> {
    let user_id = get_authenticated_user(&req, &supabase_config).await?;

    let result = analysis_service
        .analyze_full(
            &user_id,
            request.period.clone(),
            Some(ws_manager.get_ref().clone()),
        )
        .await;

    match result {
        Ok(analysis) => Ok(HttpResponse::Ok().json(json!({
            "success": true,
            "data": analysis
        }))),
        Err(e) => {
            // Convert AnalysisError to appropriate HTTP response
            if let Some(analysis_err) = e.downcast_ref::<AnalysisError>() {
                let status = match analysis_err {
                    AnalysisError::InsufficientTrades { .. } => {
                        actix_web::http::StatusCode::BAD_REQUEST
                    }
                    AnalysisError::NoTrades { .. } => actix_web::http::StatusCode::NOT_FOUND,
                    AnalysisError::QdrantUnavailable { .. } => {
                        actix_web::http::StatusCode::SERVICE_UNAVAILABLE
                    }
                    AnalysisError::AIGeneration { .. } => {
                        actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
                    }
                    AnalysisError::InvalidPeriod { .. } => actix_web::http::StatusCode::BAD_REQUEST,
                };
                Ok(HttpResponse::build(status).json(json!({
                    "success": false,
                    "error": analysis_err.to_string(),
                    "error_type": match analysis_err {
                        AnalysisError::InsufficientTrades { .. } => "insufficient_trades",
                        AnalysisError::NoTrades { .. } => "no_trades",
                        AnalysisError::QdrantUnavailable { .. } => "qdrant_unavailable",
                        AnalysisError::AIGeneration { .. } => "ai_generation",
                        AnalysisError::InvalidPeriod { .. } => "invalid_period",
                    }
                })))
            } else {
                Ok(HttpResponse::InternalServerError().json(json!({
                    "success": false,
                    "error": e.to_string()
                })))
            }
        }
    }
}

/// POST /api/analysis/save
/// Save rendered analysis content to user database
pub async fn save_analysis(
    req: HttpRequest,
    request: web::Json<SaveAnalysisRequest>,
    supabase_config: web::Data<SupabaseConfig>,
    analysis_service: web::Data<Arc<TradeAnalysisService>>,
) -> ActixResult<HttpResponse> {
    let user_id = get_authenticated_user(&req, &supabase_config).await?;

    let payload = request.into_inner();
    let id = analysis_service
        .save_analysis_record(&user_id, payload.time_range, payload.title, payload.content)
        .await
        .map_err(|e| {
            log::error!("Failed to save analysis record: {}", e);
            actix_web::error::ErrorInternalServerError("Failed to save analysis")
        })?;

    Ok(HttpResponse::Ok().json(SaveAnalysisResponse { id }))
}

#[derive(Debug, Deserialize)]
pub struct AnalysisHistoryQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// GET /api/analysis/history
/// Fetch stored analysis records for the authenticated user
pub async fn list_analysis_history(
    req: HttpRequest,
    query: web::Query<AnalysisHistoryQuery>,
    supabase_config: web::Data<SupabaseConfig>,
    analysis_service: web::Data<Arc<TradeAnalysisService>>,
) -> ActixResult<HttpResponse> {
    let user_id = get_authenticated_user(&req, &supabase_config).await?;

    let records: Vec<AnalysisRecord> = analysis_service
        .fetch_analysis_records(&user_id, query.limit, query.offset)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch analysis history: {}", e);
            actix_web::error::ErrorInternalServerError("Failed to fetch analysis history")
        })?;

    Ok(HttpResponse::Ok().json(json!({
        "success": true,
        "data": records
    })))
}

/// GET /api/analysis/history/{id}
/// Fetch a single analysis record by id for the authenticated user
pub async fn get_analysis_history_item(
    req: HttpRequest,
    path: web::Path<String>,
    supabase_config: web::Data<SupabaseConfig>,
    analysis_service: web::Data<Arc<TradeAnalysisService>>,
) -> ActixResult<HttpResponse> {
    let user_id = get_authenticated_user(&req, &supabase_config).await?;
    let record_id = path.into_inner();

    let record_opt = analysis_service
        .fetch_analysis_record_by_id(&user_id, &record_id)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch analysis record {}: {}", record_id, e);
            actix_web::error::ErrorInternalServerError("Failed to fetch analysis record")
        })?;

    match record_opt {
        Some(record) => Ok(HttpResponse::Ok().json(json!({
            "success": true,
            "data": record
        }))),
        None => Ok(HttpResponse::NotFound().json(json!({
            "success": false,
            "error": "Analysis record not found"
        }))),
    }
}

pub fn configure_ai_analysis_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/analysis")
            .route("/generate", web::post().to(generate_analysis))
            .route("/save", web::post().to(save_analysis))
            .route("/history", web::get().to(list_analysis_history))
            .route("/history/{id}", web::get().to(get_analysis_history_item)),
    );
}
