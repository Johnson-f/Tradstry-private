use actix_web::{HttpResponse, Result, web};
use serde::Serialize;

use crate::{
    service::market_engine::{
        earnings_calendar, earnings_transcripts, financials, historical, holders, hours, indices,
        movers, news, quotes, search as search_svc, sectors,
    },
    turso::AppState,
};

#[derive(Debug, Serialize)]
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
    pub fn error(err: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(err),
        }
    }
}

pub async fn get_hours(_app_state: web::Data<AppState>) -> Result<HttpResponse> {
    match hours::get_hours().await {
        Ok(res) => Ok(HttpResponse::Ok().json(ApiResponse::success(res))),
        Err(e) => Ok(HttpResponse::BadGateway().json(ApiResponse::<()>::error(e.to_string()))),
    }
}

#[derive(serde::Deserialize)]
pub struct QuotesQuery {
    symbols: Option<String>,
}

pub async fn get_quotes_handler(
    _app_state: web::Data<AppState>,
    query: web::Query<QuotesQuery>,
) -> Result<HttpResponse> {
    let symbols: Vec<String> = query
        .symbols
        .as_deref()
        .unwrap_or("")
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| s.trim().to_uppercase())
        .collect();
    match quotes::get_quotes(&symbols).await {
        Ok(res) => Ok(HttpResponse::Ok().json(ApiResponse::success(res))),
        Err(e) => Ok(HttpResponse::BadGateway().json(ApiResponse::<()>::error(e.to_string()))),
    }
}

pub async fn get_simple_quotes_handler(
    _app_state: web::Data<AppState>,
    query: web::Query<QuotesQuery>,
) -> Result<HttpResponse> {
    let symbols: Vec<String> = query
        .symbols
        .as_deref()
        .unwrap_or("")
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| s.trim().to_uppercase())
        .collect();
    let symbol_refs: Vec<&str> = symbols.iter().map(|s| s.as_str()).collect();
    match quotes::get_simple_quotes(&symbol_refs).await {
        Ok(res) => Ok(HttpResponse::Ok().json(ApiResponse::success(res))),
        Err(e) => Ok(HttpResponse::BadGateway().json(ApiResponse::<()>::error(e.to_string()))),
    }
}

#[derive(serde::Deserialize)]
pub struct SimilarQuery {
    symbol: String,
}

pub async fn get_similar_handler(
    _app_state: web::Data<AppState>,
    query: web::Query<SimilarQuery>,
) -> Result<HttpResponse> {
    match quotes::get_similar(&query.symbol).await {
        Ok(res) => Ok(HttpResponse::Ok().json(ApiResponse::success(res))),
        Err(e) => Ok(HttpResponse::BadGateway().json(ApiResponse::<()>::error(e.to_string()))),
    }
}

#[derive(serde::Deserialize)]
pub struct LogoQuery {
    symbol: String,
}

pub async fn get_logo_handler(
    _app_state: web::Data<AppState>,
    query: web::Query<LogoQuery>,
) -> Result<HttpResponse> {
    match quotes::get_logo(&query.symbol).await {
        Ok(res) => Ok(HttpResponse::Ok().json(ApiResponse::success(res))),
        Err(e) => Ok(HttpResponse::BadGateway().json(ApiResponse::<()>::error(e.to_string()))),
    }
}

#[derive(serde::Deserialize)]
pub struct DetailedQuoteQuery {
    symbol: String,
}

pub async fn get_detailed_quote_handler(
    _app_state: web::Data<AppState>,
    query: web::Query<DetailedQuoteQuery>,
) -> Result<HttpResponse> {
    match quotes::get_detailed_quote(&query.symbol).await {
        Ok(res) => Ok(HttpResponse::Ok().json(ApiResponse::success(res))),
        Err(e) => Ok(HttpResponse::BadGateway().json(ApiResponse::<()>::error(e.to_string()))),
    }
}

#[derive(serde::Deserialize)]
pub struct HistoricalQuery {
    symbol: String,
    range: Option<String>,
    interval: Option<String>,
}

pub async fn get_historical_handler(
    _app_state: web::Data<AppState>,
    query: web::Query<HistoricalQuery>,
) -> Result<HttpResponse> {
    match historical::get_historical(
        &query.symbol,
        query.range.as_deref(),
        query.interval.as_deref(),
    )
    .await
    {
        Ok(res) => Ok(HttpResponse::Ok().json(ApiResponse::success(res))),
        Err(e) => Ok(HttpResponse::BadGateway().json(ApiResponse::<()>::error(e.to_string()))),
    }
}

pub async fn get_movers_handler(_app_state: web::Data<AppState>) -> Result<HttpResponse> {
    match movers::get_movers().await {
        Ok(res) => Ok(HttpResponse::Ok().json(ApiResponse::success(res))),
        Err(e) => Ok(HttpResponse::BadGateway().json(ApiResponse::<()>::error(e.to_string()))),
    }
}

#[derive(serde::Deserialize)]
pub struct MoversCountQuery {
    count: Option<u32>,
}

pub async fn get_gainers_handler(
    _app_state: web::Data<AppState>,
    query: web::Query<MoversCountQuery>,
) -> Result<HttpResponse> {
    match movers::get_gainers(query.count).await {
        Ok(res) => Ok(HttpResponse::Ok().json(ApiResponse::success(res))),
        Err(e) => Ok(HttpResponse::BadGateway().json(ApiResponse::<()>::error(e.to_string()))),
    }
}

pub async fn get_losers_handler(
    _app_state: web::Data<AppState>,
    query: web::Query<MoversCountQuery>,
) -> Result<HttpResponse> {
    match movers::get_losers(query.count).await {
        Ok(res) => Ok(HttpResponse::Ok().json(ApiResponse::success(res))),
        Err(e) => Ok(HttpResponse::BadGateway().json(ApiResponse::<()>::error(e.to_string()))),
    }
}

pub async fn get_most_active_handler(
    _app_state: web::Data<AppState>,
    query: web::Query<MoversCountQuery>,
) -> Result<HttpResponse> {
    match movers::get_most_active(query.count).await {
        Ok(res) => Ok(HttpResponse::Ok().json(ApiResponse::success(res))),
        Err(e) => Ok(HttpResponse::BadGateway().json(ApiResponse::<()>::error(e.to_string()))),
    }
}

#[derive(serde::Deserialize)]
pub struct NewsQuery {
    symbol: Option<String>,
    limit: Option<u32>,
}

pub async fn get_news_handler(
    _app_state: web::Data<AppState>,
    query: web::Query<NewsQuery>,
) -> Result<HttpResponse> {
    match news::get_news(query.symbol.as_deref(), query.limit).await {
        Ok(res) => Ok(HttpResponse::Ok().json(ApiResponse::success(res))),
        Err(e) => Ok(HttpResponse::BadGateway().json(ApiResponse::<()>::error(e.to_string()))),
    }
}

pub async fn get_indices_handler(_app_state: web::Data<AppState>) -> Result<HttpResponse> {
    match indices::get_indices().await {
        Ok(res) => Ok(HttpResponse::Ok().json(ApiResponse::success(res))),
        Err(e) => Ok(HttpResponse::BadGateway().json(ApiResponse::<()>::error(e.to_string()))),
    }
}

pub async fn get_sectors_handler(_app_state: web::Data<AppState>) -> Result<HttpResponse> {
    match sectors::get_sectors().await {
        Ok(res) => Ok(HttpResponse::Ok().json(ApiResponse::success(res))),
        Err(e) => Ok(HttpResponse::BadGateway().json(ApiResponse::<()>::error(e.to_string()))),
    }
}

#[derive(serde::Deserialize)]
pub struct SearchQuery {
    q: String,
    hits: Option<u32>,
    yahoo: Option<bool>,
}

pub async fn search_handler(
    _app_state: web::Data<AppState>,
    query: web::Query<SearchQuery>,
) -> Result<HttpResponse> {
    match search_svc::search(&query.q, query.hits, query.yahoo).await {
        Ok(res) => Ok(HttpResponse::Ok().json(ApiResponse::success(res))),
        Err(e) => Ok(HttpResponse::BadGateway().json(ApiResponse::<()>::error(e.to_string()))),
    }
}

#[derive(serde::Deserialize)]
pub struct FinancialsQuery {
    symbol: String,
    statement: Option<String>,
    frequency: Option<String>,
}

pub async fn get_financials_handler(
    _app_state: web::Data<AppState>,
    query: web::Query<FinancialsQuery>,
) -> Result<HttpResponse> {
    match financials::get_financials(
        &query.symbol,
        query.statement.as_deref(),
        query.frequency.as_deref(),
    )
    .await
    {
        Ok(res) => Ok(HttpResponse::Ok().json(ApiResponse::success(res))),
        Err(e) => Ok(HttpResponse::BadGateway().json(ApiResponse::<()>::error(e.to_string()))),
    }
}

#[derive(serde::Deserialize)]
pub struct EarningsTranscriptQuery {
    symbol: String,
    quarter: Option<String>,
    year: Option<i32>,
}

pub async fn get_earnings_transcript_handler(
    _app_state: web::Data<AppState>,
    query: web::Query<EarningsTranscriptQuery>,
) -> Result<HttpResponse> {
    match earnings_transcripts::get_earnings_transcript(
        &query.symbol,
        query.quarter.as_deref(),
        query.year,
    )
    .await
    {
        Ok(res) => Ok(HttpResponse::Ok().json(ApiResponse::success(res))),
        Err(e) => Ok(HttpResponse::BadGateway().json(ApiResponse::<()>::error(e.to_string()))),
    }
}

#[derive(serde::Deserialize)]
pub struct HoldersQuery {
    symbol: String,
    holder_type: Option<String>,
}

pub async fn get_holders_handler(
    _app_state: web::Data<AppState>,
    query: web::Query<HoldersQuery>,
) -> Result<HttpResponse> {
    match holders::get_holders(&query.symbol, query.holder_type.as_deref()).await {
        Ok(res) => Ok(HttpResponse::Ok().json(ApiResponse::success(res))),
        Err(e) => Ok(HttpResponse::BadGateway().json(ApiResponse::<()>::error(e.to_string()))),
    }
}

#[derive(serde::Deserialize)]
pub struct EarningsCalendarQuery {
    pub from_date: Option<String>,
    pub to_date: Option<String>,
}

pub async fn get_earnings_calendar_handler(
    _app_state: web::Data<AppState>,
    query: web::Query<EarningsCalendarQuery>,
) -> Result<HttpResponse> {
    match earnings_calendar::fetch_earnings_calendar(
        query.from_date.as_deref(),
        query.to_date.as_deref(),
        false,
    )
    .await
    {
        Ok(res) => Ok(HttpResponse::Ok().json(ApiResponse::success(res))),
        Err(e) => Ok(HttpResponse::BadGateway().json(ApiResponse::<()>::error(e.to_string()))),
    }
}

pub fn configure_market_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/api/market/hours", web::get().to(get_hours))
        .route("/api/market/quotes", web::get().to(get_quotes_handler))
        .route(
            "/api/market/simple-quotes",
            web::get().to(get_simple_quotes_handler),
        )
        .route("/api/market/similar", web::get().to(get_similar_handler))
        .route("/api/market/logo", web::get().to(get_logo_handler))
        .route(
            "/api/market/detailed-quote",
            web::get().to(get_detailed_quote_handler),
        )
        .route(
            "/api/market/historical",
            web::get().to(get_historical_handler),
        )
        .route("/api/market/movers", web::get().to(get_movers_handler))
        .route("/api/market/gainers", web::get().to(get_gainers_handler))
        .route("/api/market/losers", web::get().to(get_losers_handler))
        .route(
            "/api/market/actives",
            web::get().to(get_most_active_handler),
        )
        .route("/api/market/news", web::get().to(get_news_handler))
        .route("/api/market/indices", web::get().to(get_indices_handler))
        .route("/api/market/sectors", web::get().to(get_sectors_handler))
        .route("/api/market/search", web::get().to(search_handler))
        .route(
            "/api/market/financials",
            web::get().to(get_financials_handler),
        )
        .route(
            "/api/market/earnings-transcript",
            web::get().to(get_earnings_transcript_handler),
        )
        .route(
            "/api/market/earnings-calendar",
            web::get().to(get_earnings_calendar_handler),
        )
        .route("/api/market/holders", web::get().to(get_holders_handler));
}
