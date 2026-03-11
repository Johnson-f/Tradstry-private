use crate::models::analytics::options::GroupingType;
use crate::models::analytics::{AnalyticsOptions, TimeSeriesInterval};
use crate::models::stock::stocks::TimeRange;
use crate::service::analytics_engine::AnalyticsEngine;
use crate::service::analytics_engine::core_metrics::{
    calculate_individual_option_trade_analytics, calculate_individual_stock_trade_analytics,
    calculate_symbol_analytics,
};
use crate::service::analytics_engine::performance_metrics::{
    DurationPerformanceResponse, calculate_duration_performance_metrics,
};
use crate::turso::{AppState, SupabaseClaims, config::SupabaseConfig};
use actix_web::{HttpRequest, HttpResponse, Result, web};
use base64::Engine;
use serde::{Deserialize, Serialize};

/// Parse JWT claims without full validation (for middleware)
fn parse_jwt_claims(token: &str) -> Result<SupabaseClaims, actix_web::Error> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(actix_web::error::ErrorUnauthorized("Invalid token format"));
    }

    let payload = parts[1];
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| actix_web::error::ErrorUnauthorized("Invalid token encoding"))?;

    let claims: SupabaseClaims = serde_json::from_slice(&decoded)
        .map_err(|_| actix_web::error::ErrorUnauthorized("Invalid token claims"))?;

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

/// Analytics service for providing comprehensive trading analytics
pub struct AnalyticsService {
    analytics_engine: AnalyticsEngine,
}

impl AnalyticsService {
    pub fn new() -> Self {
        Self {
            analytics_engine: AnalyticsEngine::new(),
        }
    }
}

/// Get authenticated user from request
async fn get_authenticated_user(
    req: &HttpRequest,
    _supabase_config: &SupabaseConfig,
) -> Result<String, actix_web::Error> {
    let token = extract_token_from_request(req).ok_or_else(|| {
        actix_web::error::ErrorUnauthorized("Missing or invalid authorization header")
    })?;

    let claims = parse_jwt_claims(&token)?;
    Ok(claims.sub)
}

/// Request parameters for analytics endpoints
#[derive(Debug, Deserialize)]
pub struct AnalyticsRequest {
    pub time_range: Option<String>,
    pub include_time_series: Option<bool>,
    pub time_series_interval: Option<String>,
    pub include_grouped_analytics: Option<bool>,
    pub grouping_types: Option<Vec<String>>,
    pub risk_free_rate: Option<f64>,
}

/// Response wrapper for analytics data
#[derive(Debug, Serialize)]
pub struct AnalyticsResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> AnalyticsResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

/// Get core analytics metrics (from core_metrics.rs)
pub async fn get_core_analytics(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    payload: Option<web::Json<AnalyticsRequest>>,
) -> Result<HttpResponse> {
    let user_id = get_authenticated_user(&req, &app_state.config.supabase).await?;

    let conn = app_state
        .get_user_db_connection(&user_id)
        .await?
        .ok_or_else(|| actix_web::error::ErrorBadRequest("User database not found"))?;

    let request = payload.as_deref();
    let time_range = parse_time_range(&request.and_then(|r| r.time_range.clone()));
    log::info!("Calculating core metrics for time range: {:?}", time_range);
    let analytics_service = AnalyticsService::new();

    match analytics_service
        .analytics_engine
        .calculate_core_metrics(&conn, &time_range)
        .await
    {
        Ok(metrics) => {
            log::info!(
                "Core metrics calculated - Total trades: {}, Winning: {}, Losing: {}, Net P&L: ${:.2}",
                metrics.total_trades,
                metrics.winning_trades,
                metrics.losing_trades,
                metrics.net_profit_loss
            );
            if metrics.total_trades == 0 {
                log::warn!(
                    "⚠️ Core metrics returned 0 trades. This usually means no closed trades match the time range filter (requires exit_price IS NOT NULL AND exit_date IS NOT NULL)"
                );
            }
            Ok(HttpResponse::Ok().json(AnalyticsResponse::success(metrics)))
        }
        Err(e) => {
            log::error!("Failed to calculate core metrics: {:?}", e);
            Ok(HttpResponse::InternalServerError()
                .json(AnalyticsResponse::<()>::error(e.to_string())))
        }
    }
}

/// Get risk analytics metrics (from risk_metrics.rs)
pub async fn get_risk_analytics(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    payload: Option<web::Json<AnalyticsRequest>>,
) -> Result<HttpResponse> {
    let user_id = get_authenticated_user(&req, &app_state.config.supabase).await?;

    let conn = app_state
        .get_user_db_connection(&user_id)
        .await?
        .ok_or_else(|| actix_web::error::ErrorBadRequest("User database not found"))?;

    let request = payload.as_deref();
    let time_range = parse_time_range(&request.and_then(|r| r.time_range.clone()));
    let options = parse_analytics_options_from_request(request);
    let analytics_service = AnalyticsService::new();

    match analytics_service
        .analytics_engine
        .calculate_risk_metrics(&conn, &time_range, &options)
        .await
    {
        Ok(metrics) => Ok(HttpResponse::Ok().json(AnalyticsResponse::success(metrics))),
        Err(e) => {
            Ok(HttpResponse::InternalServerError()
                .json(AnalyticsResponse::<()>::error(e.to_string())))
        }
    }
}

/// Combined performance response including duration performance
#[derive(Debug, Serialize)]
pub struct PerformanceAnalyticsResponse {
    pub performance_metrics: crate::models::analytics::PerformanceMetrics,
    pub duration_performance: DurationPerformanceResponse,
}

/// Get performance analytics metrics (from performance_metrics.rs)
/// Returns both PerformanceMetrics and DurationPerformanceResponse
pub async fn get_performance_analytics(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    payload: Option<web::Json<AnalyticsRequest>>,
) -> Result<HttpResponse> {
    let user_id = get_authenticated_user(&req, &app_state.config.supabase).await?;

    let conn = app_state
        .get_user_db_connection(&user_id)
        .await?
        .ok_or_else(|| actix_web::error::ErrorBadRequest("User database not found"))?;

    let request = payload.as_deref();
    let time_range = parse_time_range(&request.and_then(|r| r.time_range.clone()));
    let analytics_service = AnalyticsService::new();

    // Calculate both performance metrics and duration performance
    let performance_metrics_result = analytics_service
        .analytics_engine
        .calculate_performance_metrics(&conn, &time_range)
        .await;
    let duration_performance_result =
        calculate_duration_performance_metrics(&conn, &time_range).await;

    match (performance_metrics_result, duration_performance_result) {
        (Ok(performance_metrics), Ok(duration_performance)) => {
            let response = PerformanceAnalyticsResponse {
                performance_metrics,
                duration_performance,
            };
            Ok(HttpResponse::Ok().json(AnalyticsResponse::success(response)))
        }
        (Err(e), _) | (_, Err(e)) => {
            log::error!("Failed to calculate performance analytics: {:?}", e);
            Ok(HttpResponse::InternalServerError()
                .json(AnalyticsResponse::<()>::error(e.to_string())))
        }
    }
}

// Get time series analytics data (from time_series.rs)
// pub async fn get_time_series_analytics(
//     req: HttpRequest,
//     app_state: web::Data<AppState>,
//     payload: Option<web::Json<AnalyticsRequest>>,
// ) -> Result<HttpResponse> {
//     let user_id = get_authenticated_user(&req, &app_state.config.supabase).await?;

//     let conn = app_state
//         .get_user_db_connection(&user_id)
//         .await?
//         .ok_or_else(|| actix_web::error::ErrorBadRequest("User database not found"))?;

//     let request = payload.as_deref();
//     let time_range = parse_time_range(&request.and_then(|r| r.time_range.clone()));
//     let options = parse_analytics_options_from_request(request);
//     let analytics_service = AnalyticsService::new();

//     match analytics_service.analytics_engine.calculate_time_series_data(&conn, &time_range, &options).await {
//         Ok(data) => Ok(HttpResponse::Ok().json(AnalyticsResponse::success(data))),
//         Err(e) => Ok(HttpResponse::InternalServerError().json(AnalyticsResponse::<()>::error(e.to_string()))),
//     }
// }

/// Logging version of get_time_series_analytics
/// This version logs all the steps and data points to help with debugging
pub async fn get_time_series_analytics(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    payload: Option<web::Json<AnalyticsRequest>>,
) -> Result<HttpResponse> {
    log::info!("=== get_time_series_analytics called ===");

    // Log authentication attempt
    log::info!("Attempting to authenticate user");
    let user_id = match get_authenticated_user(&req, &app_state.config.supabase).await {
        Ok(id) => {
            log::info!("User authenticated successfully: {}", id);
            id
        }
        Err(e) => {
            log::error!("Authentication failed: {:?}", e);
            return Err(e);
        }
    };

    // Log database connection attempt
    log::info!(
        "Attempting to get database connection for user: {}",
        user_id
    );
    let conn = match app_state.get_user_db_connection(&user_id).await {
        Ok(Some(conn)) => {
            log::info!("Database connection obtained successfully");
            conn
        }
        Ok(None) => {
            log::error!("User database not found for user_id: {}", user_id);
            return Err(actix_web::error::ErrorBadRequest("User database not found"));
        }
        Err(e) => {
            log::error!("Failed to get database connection: {:?}", e);
            return Err(actix_web::error::ErrorInternalServerError(e));
        }
    };

    // Log request parsing
    log::info!("Parsing request payload");
    let request = payload.as_deref();
    log::debug!("Request payload: {:?}", request);

    let time_range = parse_time_range(&request.and_then(|r| r.time_range.clone()));
    log::info!("Parsed time range: {:?}", time_range);

    let options = parse_analytics_options_from_request(request);
    log::info!("Parsed analytics options: {:?}", options);

    // Log analytics service creation
    log::info!("Creating AnalyticsService");
    let analytics_service = AnalyticsService::new();

    // Log analytics calculation attempt
    log::info!("Starting time series data calculation");
    match analytics_service
        .analytics_engine
        .calculate_time_series_data(&conn, &time_range, &options)
        .await
    {
        Ok(data) => {
            let daily_pnl_count = data.daily_pnl.len();
            let weekly_pnl_count = data.weekly_pnl.len();
            let monthly_pnl_count = data.monthly_pnl.len();
            log::info!(
                "Time series data calculated successfully - Daily PnL: {} points, Weekly: {} points, Monthly: {} points",
                daily_pnl_count,
                weekly_pnl_count,
                monthly_pnl_count
            );
            log::info!("Total trades in time series: {}", data.total_trades);
            if daily_pnl_count == 0 {
                log::warn!(
                    "⚠️ Time series returned empty daily_pnl array. This usually means no closed trades match the time range filter."
                );
            }
            log::debug!(
                "Response data sample (first 3 daily points): {:?}",
                data.daily_pnl.iter().take(3).collect::<Vec<_>>()
            );
            Ok(HttpResponse::Ok().json(AnalyticsResponse::success(data)))
        }
        Err(e) => {
            log::error!("Failed to calculate time series data: {:?}", e);
            log::error!(
                "Error details - Type: {}, Message: {}",
                std::any::type_name_of_val(&e),
                e
            );
            Ok(HttpResponse::InternalServerError()
                .json(AnalyticsResponse::<()>::error(e.to_string())))
        }
    }
}

/// Get grouped analytics data (from grouping.rs)
pub async fn get_grouped_analytics(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    payload: Option<web::Json<AnalyticsRequest>>,
) -> Result<HttpResponse> {
    let user_id = get_authenticated_user(&req, &app_state.config.supabase).await?;

    let conn = app_state
        .get_user_db_connection(&user_id)
        .await?
        .ok_or_else(|| actix_web::error::ErrorBadRequest("User database not found"))?;

    let request = payload.as_deref();
    let time_range = parse_time_range(&request.and_then(|r| r.time_range.clone()));
    let options = parse_analytics_options_from_request(request);
    let analytics_service = AnalyticsService::new();

    match analytics_service
        .analytics_engine
        .calculate_grouped_analytics(&conn, &time_range, &options)
        .await
    {
        Ok(data) => Ok(HttpResponse::Ok().json(AnalyticsResponse::success(data))),
        Err(e) => {
            Ok(HttpResponse::InternalServerError()
                .json(AnalyticsResponse::<()>::error(e.to_string())))
        }
    }
}

/// Get comprehensive analytics (all metrics combined from core_metrics.rs, risk_metrics.rs, performance_metrics.rs, time_series.rs, grouping.rs)
pub async fn get_comprehensive_analytics(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    payload: Option<web::Json<AnalyticsRequest>>,
) -> Result<HttpResponse> {
    let user_id = get_authenticated_user(&req, &app_state.config.supabase).await?;

    let conn = app_state
        .get_user_db_connection(&user_id)
        .await?
        .ok_or_else(|| actix_web::error::ErrorBadRequest("User database not found"))?;

    let request = payload.as_deref();
    let time_range = parse_time_range(&request.and_then(|r| r.time_range.clone()));
    let options = parse_analytics_options_from_request(request);
    let analytics_service = AnalyticsService::new();

    match analytics_service
        .analytics_engine
        .calculate_comprehensive_analytics(&conn, &time_range, options)
        .await
    {
        Ok(data) => Ok(HttpResponse::Ok().json(AnalyticsResponse::success(data))),
        Err(e) => {
            Ok(HttpResponse::InternalServerError()
                .json(AnalyticsResponse::<()>::error(e.to_string())))
        }
    }
}

/// Request parameters for individual trade analytics
#[derive(Debug, Deserialize)]
pub struct IndividualTradeAnalyticsRequest {
    pub trade_id: i64,
    pub trade_type: String, // "stock" or "option"
}

/// Get analytics for an individual trade (stock or option)
pub async fn get_individual_trade_analytics(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    query: web::Query<IndividualTradeAnalyticsRequest>,
) -> Result<HttpResponse> {
    let user_id = get_authenticated_user(&req, &app_state.config.supabase).await?;

    let conn = app_state
        .get_user_db_connection(&user_id)
        .await?
        .ok_or_else(|| actix_web::error::ErrorBadRequest("User database not found"))?;

    match query.trade_type.as_str() {
        "stock" => match calculate_individual_stock_trade_analytics(&conn, query.trade_id).await {
            Ok(analytics) => Ok(HttpResponse::Ok().json(AnalyticsResponse::success(analytics))),
            Err(e) => Ok(HttpResponse::InternalServerError()
                .json(AnalyticsResponse::<()>::error(e.to_string()))),
        },
        "option" => {
            match calculate_individual_option_trade_analytics(&conn, query.trade_id).await {
                Ok(analytics) => Ok(HttpResponse::Ok().json(AnalyticsResponse::success(analytics))),
                Err(e) => Ok(HttpResponse::InternalServerError()
                    .json(AnalyticsResponse::<()>::error(e.to_string()))),
            }
        }
        _ => Ok(
            HttpResponse::BadRequest().json(AnalyticsResponse::<()>::error(
                "Invalid trade_type. Must be 'stock' or 'option'".to_string(),
            )),
        ),
    }
}

/// Request parameters for symbol analytics
#[derive(Debug, Deserialize)]
pub struct SymbolAnalyticsRequest {
    pub symbol: String,
    pub time_range: Option<String>,
}

/// Get analytics for a specific symbol across all trades
pub async fn get_symbol_analytics(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    query: web::Query<SymbolAnalyticsRequest>,
) -> Result<HttpResponse> {
    let user_id = get_authenticated_user(&req, &app_state.config.supabase).await?;

    let conn = app_state
        .get_user_db_connection(&user_id)
        .await?
        .ok_or_else(|| actix_web::error::ErrorBadRequest("User database not found"))?;

    let time_range = parse_time_range(&query.time_range);

    match calculate_symbol_analytics(&conn, &query.symbol, &time_range).await {
        Ok(analytics) => Ok(HttpResponse::Ok().json(AnalyticsResponse::success(analytics))),
        Err(e) => {
            Ok(HttpResponse::InternalServerError()
                .json(AnalyticsResponse::<()>::error(e.to_string())))
        }
    }
}

/// Parse time range from query parameter
fn parse_time_range(time_range_str: &Option<String>) -> TimeRange {
    match time_range_str {
        Some(range) => match range.as_str() {
            "7d" => TimeRange::SevenDays,
            "30d" => TimeRange::ThirtyDays,
            "90d" => TimeRange::NinetyDays,
            "1y" => TimeRange::OneYear,
            "ytd" => TimeRange::YearToDate,
            "all_time" => TimeRange::AllTime,
            _ => TimeRange::AllTime,
        },
        None => TimeRange::AllTime,
    }
}

/// Parse analytics options from request (works with both query and body)
fn parse_analytics_options(query: &AnalyticsRequest) -> AnalyticsOptions {
    let time_range = parse_time_range(&query.time_range);

    let time_series_interval = match query.time_series_interval.as_ref() {
        Some(interval) => match interval.as_str() {
            "daily" => TimeSeriesInterval::Daily,
            "weekly" => TimeSeriesInterval::Weekly,
            "monthly" => TimeSeriesInterval::Monthly,
            _ => TimeSeriesInterval::Daily,
        },
        None => TimeSeriesInterval::Daily,
    };

    let grouping_types = query
        .grouping_types
        .as_ref()
        .map(|types| {
            types
                .iter()
                .map(|t| match t.as_str() {
                    "symbol" => GroupingType::Symbol,
                    "strategy" => GroupingType::Strategy,
                    "trade_direction" => GroupingType::TradeDirection,
                    "time_period" => GroupingType::TimePeriod,
                    _ => GroupingType::Symbol,
                })
                .collect()
        })
        .unwrap_or_else(|| vec![GroupingType::Symbol]);

    AnalyticsOptions {
        time_range,
        include_time_series: query.include_time_series.unwrap_or(true),
        time_series_interval,
        include_grouped_analytics: query.include_grouped_analytics.unwrap_or(false),
        grouping_types,
        risk_free_rate: query.risk_free_rate.unwrap_or(0.02),
        confidence_levels: vec![0.95, 0.99],
    }
}

/// Parse analytics options from optional request payload
fn parse_analytics_options_from_request(request: Option<&AnalyticsRequest>) -> AnalyticsOptions {
    if let Some(req) = request {
        parse_analytics_options(req)
    } else {
        // Default options
        AnalyticsOptions {
            time_range: TimeRange::AllTime,
            include_time_series: true,
            time_series_interval: TimeSeriesInterval::Daily,
            include_grouped_analytics: false,
            grouping_types: vec![GroupingType::Symbol],
            risk_free_rate: 0.02,
            confidence_levels: vec![0.95, 0.99],
        }
    }
}

/// Configure analytics routes
pub fn configure_analytics_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/analytics")
            .route("/core", web::post().to(get_core_analytics))
            .route("/risk", web::post().to(get_risk_analytics))
            .route("/performance", web::post().to(get_performance_analytics))
            .route("/time-series", web::post().to(get_time_series_analytics))
            .route("/grouped", web::post().to(get_grouped_analytics))
            .route(
                "/comprehensive",
                web::post().to(get_comprehensive_analytics),
            )
            .route("/trade", web::get().to(get_individual_trade_analytics))
            .route("/symbol", web::get().to(get_symbol_analytics)),
    );
}
