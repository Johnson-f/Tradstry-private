use actix_web::{HttpRequest, HttpResponse, Result as ActixResult, web};
use chrono::Utc;
use libsql::Connection;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::turso::{
    AppState,
    client::TursoClient,
    config::{SupabaseClaims, SupabaseConfig},
};

use crate::turso::auth::{get_supabase_user_id, validate_supabase_jwt_token};

use crate::models::options::option_trade::{CreateOptionRequest, OptionTrade, OptionType};
use crate::models::stock::stocks::{CreateStockRequest, OrderType, Stock, TradeType};
use crate::service::ai_service::TradeVectorization;
use crate::service::brokerage::{
    accounts, client::SnapTradeClient, connections, holdings, transactions,
};

/// Response wrapper
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

/// Request/Response types
#[derive(Deserialize)]
pub struct ConnectBrokerageRequest {
    pub brokerage_id: String,
    pub connection_type: Option<String>, // "read" or "trade"
    #[allow(dead_code)]
    pub redirect_uri: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ConnectBrokerageResponse {
    pub redirect_url: String,
    pub connection_id: String,
}

#[derive(Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SyncAccountsResponse {
    pub accounts: Vec<serde_json::Value>,
    pub holdings: Vec<serde_json::Value>,
    pub transactions: Vec<serde_json::Value>,
}

#[derive(Serialize)]
#[allow(dead_code)]
pub struct SyncSummary {
    pub accounts_synced: usize,
    pub holdings_synced: usize,
    pub transactions_synced: usize,
    pub last_sync_at: String,
}

/// Helper: Extract and validate auth from request
async fn get_authenticated_user(
    req: &HttpRequest,
    supabase_config: &SupabaseConfig,
) -> Result<SupabaseClaims, actix_web::Error> {
    let auth_header = req.headers().get("authorization");

    let token = if let Some(header_value) = auth_header {
        let header_str = header_value
            .to_str()
            .map_err(|_| actix_web::error::ErrorUnauthorized("Invalid authorization header"))?;

        if let Some(token) = header_str.strip_prefix("Bearer ") {
            token.to_string()
        } else {
            return Err(actix_web::error::ErrorUnauthorized("Missing Bearer token"));
        }
    } else {
        return Err(actix_web::error::ErrorUnauthorized(
            "Missing authorization header",
        ));
    };

    validate_supabase_jwt_token(&token, supabase_config)
        .await
        .map_err(|e| {
            error!("JWT validation failed: {}", e);
            actix_web::error::ErrorUnauthorized("Invalid or expired authentication token")
        })
}

/// Helper: Get user database connection
async fn get_user_db_connection(
    user_id: &str,
    turso_client: &Arc<TursoClient>,
) -> Result<Connection, actix_web::Error> {
    turso_client
        .get_user_database_connection(user_id)
        .await
        .map_err(|e| {
            error!("Error getting user database connection: {}", e);
            actix_web::error::ErrorInternalServerError("Database access error")
        })?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))
}

// get_or_create_snaptrade_user moved to service::brokerage::helpers

/// Route: Initiate brokerage connection
pub async fn initiate_connection(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    body: web::Json<ConnectBrokerageRequest>,
    supabase_config: web::Data<SupabaseConfig>,
) -> ActixResult<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = get_supabase_user_id(&claims);

    let conn = get_user_db_connection(&user_id, &app_state.turso_client).await?;

    let snaptrade_client = SnapTradeClient::new(app_state.config.snaptrade_service_url.clone())
        .map_err(|e| {
            error!("Failed to create SnapTrade client: {}", e);
            actix_web::error::ErrorInternalServerError("Service configuration error")
        })?;

    match connections::initiate_connection(
        &conn,
        &user_id,
        &body.brokerage_id,
        body.connection_type.as_deref(),
        &snaptrade_client,
    )
    .await
    {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::success(data))),
        Err(e) => {
            error!("Failed to initiate connection: {}", e);
            Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(&e.to_string())))
        }
    }
}

/// Route: Get connection status
pub async fn get_connection_status(
    req: HttpRequest,
    path: web::Path<String>,
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
) -> ActixResult<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = get_supabase_user_id(&claims);
    let connection_id = path.into_inner();

    let conn = get_user_db_connection(&user_id, &app_state.turso_client).await?;

    let snaptrade_client = SnapTradeClient::new(app_state.config.snaptrade_service_url.clone())
        .map_err(|e| {
            error!("Failed to create SnapTrade client: {}", e);
            actix_web::error::ErrorInternalServerError("Service configuration error")
        })?;

    match connections::get_connection_status(&conn, &connection_id, &user_id, &snaptrade_client)
        .await
    {
        Ok(status_data) => Ok(HttpResponse::Ok().json(ApiResponse::success(status_data))),
        Err(e) => {
            error!("Failed to get connection status: {}", e);
            if e.to_string().contains("not found") {
                Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error(&e.to_string())))
            } else {
                Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(&e.to_string())))
            }
        }
    }
}

/// Route: List connections
pub async fn list_connections(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
) -> ActixResult<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = get_supabase_user_id(&claims);

    let conn = get_user_db_connection(&user_id, &app_state.turso_client).await?;

    match connections::list_connections(&conn, &user_id).await {
        Ok(connections) => Ok(HttpResponse::Ok().json(ApiResponse::success(connections))),
        Err(e) => {
            error!("Failed to list connections: {}", e);
            Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(&e.to_string())))
        }
    }
}

/// Route: Delete connection
pub async fn delete_connection(
    req: HttpRequest,
    path: web::Path<String>,
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
) -> ActixResult<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = get_supabase_user_id(&claims);
    let connection_id = path.into_inner();

    let conn = get_user_db_connection(&user_id, &app_state.turso_client).await?;

    let snaptrade_client = SnapTradeClient::new(app_state.config.snaptrade_service_url.clone())
        .map_err(|e| {
            error!("Failed to create SnapTrade client: {}", e);
            actix_web::error::ErrorInternalServerError("Service configuration error")
        })?;

    match connections::delete_connection(&conn, &connection_id, &user_id, &snaptrade_client).await {
        Ok(_) => Ok(
            HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
            "success": true,
            "message": "Connection deleted successfully"
            }))),
        ),
        Err(e) => {
            error!("Failed to delete connection: {}", e);
            if e.to_string().contains("not found") {
                Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error(&e.to_string())))
            } else {
                Ok(HttpResponse::InternalServerError()
                    .json(ApiResponse::<()>::error(&e.to_string())))
            }
        }
    }
}

/// Route: List accounts
pub async fn list_accounts(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
) -> ActixResult<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = get_supabase_user_id(&claims);

    let conn = get_user_db_connection(&user_id, &app_state.turso_client).await?;

    match accounts::list_accounts(&conn, &user_id).await {
        Ok(accounts) => Ok(HttpResponse::Ok().json(ApiResponse::success(accounts))),
        Err(e) => {
            error!("Failed to list accounts: {}", e);
            Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(&e.to_string())))
        }
    }
}

/// Route: Get account detail
pub async fn get_account_detail(
    req: HttpRequest,
    path: web::Path<String>,
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
) -> ActixResult<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = get_supabase_user_id(&claims);
    let account_id = path.into_inner();

    let conn = get_user_db_connection(&user_id, &app_state.turso_client).await?;

    let snaptrade_client = SnapTradeClient::new(app_state.config.snaptrade_service_url.clone())
        .map_err(|e| {
            error!("Failed to create SnapTrade client: {}", e);
            actix_web::error::ErrorInternalServerError("Service configuration error")
        })?;

    match accounts::get_account_detail(&conn, &account_id, &user_id, &snaptrade_client).await {
        Ok(account_detail) => Ok(HttpResponse::Ok().json(ApiResponse::success(account_detail))),
        Err(e) => {
            error!("Failed to get account detail: {}", e);
            if e.to_string().contains("not found") {
                Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error(&e.to_string())))
            } else {
                Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(&e.to_string())))
            }
        }
    }
}

/// Route: Sync accounts
pub async fn sync_accounts(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
) -> ActixResult<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = get_supabase_user_id(&claims);

    let conn = get_user_db_connection(&user_id, &app_state.turso_client).await?;

    let snaptrade_client = SnapTradeClient::new(app_state.config.snaptrade_service_url.clone())
        .map_err(|e| {
            error!("Failed to create SnapTrade client: {}", e);
            actix_web::error::ErrorInternalServerError("Service configuration error")
        })?;

    match accounts::sync_accounts(&conn, &user_id, &snaptrade_client).await {
        Ok(summary) => Ok(HttpResponse::Ok().json(ApiResponse::success(summary))),
        Err(e) => {
            error!("Failed to sync accounts: {}", e);
            Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(&e.to_string())))
        }
    }
}

/// Route: Get transactions
pub async fn get_transactions(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    query: web::Query<HashMap<String, String>>,
) -> ActixResult<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = get_supabase_user_id(&claims);

    let conn = get_user_db_connection(&user_id, &app_state.turso_client).await?;

    let account_id = query.get("account_id").map(|s| s.as_str());
    let exclude_transformed = query
        .get("exclude_transformed")
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(false);

    match transactions::get_transactions(&conn, &user_id, account_id, exclude_transformed).await {
        Ok(transactions) => Ok(HttpResponse::Ok().json(ApiResponse::success(transactions))),
        Err(e) => {
            error!("Failed to get transactions: {}", e);
            Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(&e.to_string())))
        }
    }
}

/// Route: Get holdings
pub async fn get_holdings(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    query: web::Query<HashMap<String, String>>,
) -> ActixResult<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = get_supabase_user_id(&claims);

    let conn = get_user_db_connection(&user_id, &app_state.turso_client).await?;

    let account_id = query.get("account_id").map(|s| s.as_str());

    match holdings::get_holdings(&conn, &user_id, account_id).await {
        Ok(holdings) => Ok(HttpResponse::Ok().json(ApiResponse::success(holdings))),
        Err(e) => {
            error!("Failed to get holdings: {}", e);
            Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(&e.to_string())))
        }
    }
}

/// Route: Get account transactions from SnapTrade
pub async fn get_account_transactions(
    req: HttpRequest,
    path: web::Path<String>,
    query: web::Query<HashMap<String, String>>,
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
) -> ActixResult<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = get_supabase_user_id(&claims);
    let account_id = path.into_inner();

    let conn = get_user_db_connection(&user_id, &app_state.turso_client).await?;

    let snaptrade_client = SnapTradeClient::new(app_state.config.snaptrade_service_url.clone())
        .map_err(|e| {
            error!("Failed to create SnapTrade client: {}", e);
            actix_web::error::ErrorInternalServerError("Service configuration error")
        })?;

    match accounts::get_account_transactions_from_snaptrade(
        &conn,
        &account_id,
        &user_id,
        &query.into_inner(),
        &snaptrade_client,
    )
    .await
    {
        Ok(transactions_data) => {
            Ok(HttpResponse::Ok().json(ApiResponse::success(transactions_data)))
        }
        Err(e) => {
            error!("Failed to get account transactions: {}", e);
            if e.to_string().contains("not found") {
                Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error(&e.to_string())))
            } else {
                Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(&e.to_string())))
            }
        }
    }
}

/// Route: Get account equity positions
pub async fn get_account_positions(
    req: HttpRequest,
    path: web::Path<String>,
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
) -> ActixResult<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = get_supabase_user_id(&claims);
    let account_id = path.into_inner();

    let conn = get_user_db_connection(&user_id, &app_state.turso_client).await?;

    let snaptrade_client = SnapTradeClient::new(app_state.config.snaptrade_service_url.clone())
        .map_err(|e| {
            error!("Failed to create SnapTrade client: {}", e);
            actix_web::error::ErrorInternalServerError("Service configuration error")
        })?;

    match accounts::get_account_positions(&conn, &account_id, &user_id, &snaptrade_client).await {
        Ok(positions_data) => Ok(HttpResponse::Ok().json(ApiResponse::success(positions_data))),
        Err(e) => {
            error!("Failed to get account positions: {}", e);
            if e.to_string().contains("not found") {
                Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error(&e.to_string())))
            } else {
                Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(&e.to_string())))
            }
        }
    }
}

/// Route: Get account option positions
pub async fn get_account_option_positions(
    req: HttpRequest,
    path: web::Path<String>,
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
) -> ActixResult<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = get_supabase_user_id(&claims);
    let account_id = path.into_inner();

    let conn = get_user_db_connection(&user_id, &app_state.turso_client).await?;

    let snaptrade_client = SnapTradeClient::new(app_state.config.snaptrade_service_url.clone())
        .map_err(|e| {
            error!("Failed to create SnapTrade client: {}", e);
            actix_web::error::ErrorInternalServerError("Service configuration error")
        })?;

    match accounts::get_account_option_positions(&conn, &account_id, &user_id, &snaptrade_client)
        .await
    {
        Ok(positions_data) => Ok(HttpResponse::Ok().json(ApiResponse::success(positions_data))),
        Err(e) => {
            error!("Failed to get account option positions: {}", e);
            if e.to_string().contains("not found") {
                Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error(&e.to_string())))
            } else {
                Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(&e.to_string())))
            }
        }
    }
}

/// Route: Complete post-connection sync
/// This endpoint is called after user returns from portal
/// It automatically syncs all accounts, positions, and transactions
pub async fn complete_connection_sync(
    req: HttpRequest,
    path: web::Path<String>,
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
) -> ActixResult<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = get_supabase_user_id(&claims);
    let connection_id = path.into_inner();

    let conn = get_user_db_connection(&user_id, &app_state.turso_client).await?;

    let snaptrade_client = SnapTradeClient::new(app_state.config.snaptrade_service_url.clone())
        .map_err(|e| {
            error!("Failed to create SnapTrade client: {}", e);
            actix_web::error::ErrorInternalServerError("Service configuration error")
        })?;

    match connections::complete_connection_sync(&conn, &connection_id, &user_id, &snaptrade_client)
        .await
    {
        Ok(summary) => Ok(HttpResponse::Ok().json(ApiResponse::success(summary))),
        Err(e) => {
            error!("Failed to complete connection sync: {}", e);
            if e.to_string().contains("not found") {
                Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error(&e.to_string())))
            } else if e.to_string().contains("not ready") || e.to_string().contains("not connected")
            {
                Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(&e.to_string())))
            } else {
                Ok(HttpResponse::InternalServerError()
                    .json(ApiResponse::<()>::error(&e.to_string())))
            }
        }
    }
}

/// Unmatched transaction response model
#[derive(Serialize, Deserialize, Debug)]
#[allow(dead_code)]
pub struct UnmatchedTransactionResponse {
    pub id: String,
    pub user_id: String,
    pub transaction_id: String,
    pub snaptrade_transaction_id: String,
    pub symbol: String,
    pub trade_type: String,
    pub units: f64,
    pub price: f64,
    pub fee: f64,
    pub trade_date: String,
    pub brokerage_name: Option<String>,
    pub is_option: bool,
    pub difficulty_reason: Option<String>,
    pub confidence_score: Option<f64>,
    pub suggested_matches: Option<Vec<String>>,
    pub status: String,
    pub resolved_trade_id: Option<i64>,
    pub resolved_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Resolve unmatched transaction request
#[derive(Deserialize, Debug)]
pub struct ResolveUnmatchedRequest {
    pub matched_transaction_id: Option<String>, // ID of the transaction to match with (if merging)
    pub action: String,                         // "merge" or "create_open"
    pub entry_price: Option<f64>,               // Required if action is "create_open"
    pub entry_date: Option<String>,             // Required if action is "create_open"
}

/// Get all unmatched transactions for the authenticated user
pub async fn get_unmatched_transactions(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
) -> ActixResult<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = get_supabase_user_id(&claims);

    let conn = get_user_db_connection(&user_id, &app_state.turso_client).await?;

    match transactions::get_unmatched_transactions(&conn, &user_id).await {
        Ok(transactions) => Ok(HttpResponse::Ok().json(ApiResponse::success(transactions))),
        Err(e) => {
            error!("Failed to get unmatched transactions: {}", e);
            Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(&e.to_string())))
        }
    }
}

/// Resolve an unmatched transaction (merge with another or create open position)
pub async fn resolve_unmatched_transaction(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<ResolveUnmatchedRequest>,
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
) -> ActixResult<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = get_supabase_user_id(&claims);
    let unmatched_id = path.into_inner();

    let conn = get_user_db_connection(&user_id, &app_state.turso_client).await?;

    match transactions::resolve_unmatched_transaction(
        &conn,
        &unmatched_id,
        &user_id,
        &body.action,
        body.matched_transaction_id.as_deref(),
        body.entry_price,
        body.entry_date.clone(),
    )
    .await
    {
        Ok(trade_id) => Ok(
            HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
                "trade_id": trade_id,
                "message": "Transaction resolved successfully"
            }))),
        ),
        Err(e) => {
            error!("Failed to resolve unmatched transaction: {}", e);
            let status = if e.to_string().contains("not found") {
                actix_web::http::StatusCode::NOT_FOUND
            } else if e.to_string().contains("required") || e.to_string().contains("Invalid") {
                actix_web::http::StatusCode::BAD_REQUEST
            } else {
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
            };
            Ok(HttpResponse::build(status).json(ApiResponse::<()>::error(&e.to_string())))
        }
    }
}

/// Ignore an unmatched transaction (mark as ignored)
pub async fn ignore_unmatched_transaction(
    req: HttpRequest,
    path: web::Path<String>,
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
) -> ActixResult<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = get_supabase_user_id(&claims);
    let unmatched_id = path.into_inner();

    let conn = get_user_db_connection(&user_id, &app_state.turso_client).await?;

    match transactions::ignore_unmatched_transaction(&conn, &unmatched_id, &user_id).await {
        Ok(_) => Ok(
            HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
            "message": "Transaction ignored successfully"
            }))),
        ),
        Err(e) => {
            error!("Failed to ignore unmatched transaction: {}", e);
            if e.to_string().contains("not found") {
                Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error(&e.to_string())))
            } else {
                Ok(HttpResponse::InternalServerError()
                    .json(ApiResponse::<()>::error(&e.to_string())))
            }
        }
    }
}

/// Get suggested matches for an unmatched transaction
pub async fn get_unmatched_suggestions(
    req: HttpRequest,
    path: web::Path<String>,
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
) -> ActixResult<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = get_supabase_user_id(&claims);
    let unmatched_id = path.into_inner();

    let conn = get_user_db_connection(&user_id, &app_state.turso_client).await?;

    match transactions::get_unmatched_suggestions(&conn, &unmatched_id, &user_id).await {
        Ok(suggestions) => Ok(HttpResponse::Ok().json(ApiResponse::success(suggestions))),
        Err(e) => {
            error!("Failed to get unmatched suggestions: {}", e);
            if e.to_string().contains("not found") {
                Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error(&e.to_string())))
            } else {
                Ok(HttpResponse::InternalServerError()
                    .json(ApiResponse::<()>::error(&e.to_string())))
            }
        }
    }
}

/// Request structure for merging transactions
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeTransactionsRequest {
    pub transaction_ids: Vec<String>,
    pub trade_type: String, // "stock" or "option"
    // Stock fields
    pub symbol: String,
    pub order_type: String,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
    pub initial_target: Option<f64>,
    pub profit_target: Option<f64>,
    pub trade_ratings: Option<i32>,
    pub reviewed: Option<bool>,
    pub mistakes: Option<String>,
    pub brokerage_name: Option<String>,
    // Option-specific fields
    #[allow(dead_code)]
    pub strategy_type: Option<String>,
    #[allow(dead_code)]
    pub trade_direction: Option<String>,
    pub option_type: Option<String>,
    pub strike_price: Option<f64>,
    pub expiration_date: Option<String>,
    #[allow(dead_code)]
    pub implied_volatility: Option<f64>,
}

/// Route: Merge brokerage transactions into a stock or option trade
pub async fn merge_transactions(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    trade_vectorization: web::Data<Arc<TradeVectorization>>,
    payload: web::Json<MergeTransactionsRequest>,
) -> ActixResult<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = get_supabase_user_id(&claims);

    let conn = get_user_db_connection(&user_id, &app_state.turso_client).await?;

    let request = payload.into_inner();

    if request.transaction_ids.is_empty() {
        return Ok(
            HttpResponse::BadRequest().json(ApiResponse::<()>::error("No transactions selected"))
        );
    }

    // Fetch selected transactions
    let placeholders = request
        .transaction_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT id, account_id, symbol, transaction_type, quantity, price, fees, trade_date, raw_data 
         FROM brokerage_transactions 
         WHERE id IN ({}) AND account_id IN (SELECT id FROM brokerage_accounts WHERE connection_id IN (SELECT id FROM brokerage_connections WHERE user_id = ?)) AND (is_transformed IS NULL OR is_transformed = 0)",
        placeholders
    );

    let stmt = conn.prepare(&sql).await.map_err(|e| {
        error!("Failed to prepare transaction query: {}", e);
        actix_web::error::ErrorInternalServerError("Database error")
    })?;

    // Build params: transaction IDs + user_id
    let mut params: Vec<&str> = request.transaction_ids.iter().map(|s| s.as_str()).collect();
    params.push(&user_id);

    let mut rows = stmt.query(params).await.map_err(|e| {
        error!("Failed to query transactions: {}", e);
        actix_web::error::ErrorInternalServerError("Database query error")
    })?;

    #[derive(Debug)]
    struct TransactionData {
        #[allow(dead_code)]
        id: String,
        #[allow(dead_code)]
        symbol: Option<String>,
        transaction_type: Option<String>,
        quantity: Option<f64>,
        price: Option<f64>,
        fees: Option<f64>,
        trade_date: String,
    }

    let mut transactions = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| {
        error!("Database row error: {}", e);
        actix_web::error::ErrorInternalServerError("Database row error")
    })? {
        transactions.push(TransactionData {
            id: row.get(0).map_err(|e| {
                error!("Failed to get id: {}", e);
                actix_web::error::ErrorInternalServerError("Database error")
            })?,
            symbol: row.get(2).map_err(|e| {
                error!("Failed to get symbol: {}", e);
                actix_web::error::ErrorInternalServerError("Database error")
            })?,
            transaction_type: row.get(3).map_err(|e| {
                error!("Failed to get transaction_type: {}", e);
                actix_web::error::ErrorInternalServerError("Database error")
            })?,
            quantity: row.get(4).map_err(|e| {
                error!("Failed to get quantity: {}", e);
                actix_web::error::ErrorInternalServerError("Database error")
            })?,
            price: row.get(5).map_err(|e| {
                error!("Failed to get price: {}", e);
                actix_web::error::ErrorInternalServerError("Database error")
            })?,
            fees: row.get(6).map_err(|e| {
                error!("Failed to get fees: {}", e);
                actix_web::error::ErrorInternalServerError("Database error")
            })?,
            trade_date: row.get(7).map_err(|e| {
                error!("Failed to get trade_date: {}", e);
                actix_web::error::ErrorInternalServerError("Database error")
            })?,
        });
    }

    if transactions.is_empty() {
        return Ok(
            HttpResponse::NotFound().json(ApiResponse::<()>::error("No valid transactions found"))
        );
    }

    // Separate BUY and SELL transactions
    let buys: Vec<&TransactionData> = transactions
        .iter()
        .filter(|t| t.transaction_type.as_deref() == Some("BUY"))
        .collect();
    let sells: Vec<&TransactionData> = transactions
        .iter()
        .filter(|t| t.transaction_type.as_deref() == Some("SELL"))
        .collect();

    // Calculate weighted averages
    let calculate_weighted_avg = |txns: &[&TransactionData]| -> (f64, f64, f64) {
        let mut total_value = 0.0;
        let mut total_quantity = 0.0;
        let mut total_fees = 0.0;

        for txn in txns {
            let qty = txn.quantity.unwrap_or(0.0);
            let price = txn.price.unwrap_or(0.0);
            total_value += price * qty;
            total_quantity += qty;
            total_fees += txn.fees.unwrap_or(0.0);
        }

        let avg_price = if total_quantity > 0.0 {
            total_value / total_quantity
        } else {
            0.0
        };

        (avg_price, total_quantity, total_fees)
    };

    let (entry_price, _entry_quantity, _entry_fees) = if !buys.is_empty() {
        calculate_weighted_avg(&buys)
    } else if !sells.is_empty() {
        calculate_weighted_avg(&sells)
    } else {
        return Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(
            "No BUY or SELL transactions found",
        )));
    };

    let (_exit_price_opt, _exit_quantity, _exit_fees) = if !sells.is_empty() {
        let (price, qty, fees) = calculate_weighted_avg(&sells);
        (Some(price), qty, fees)
    } else {
        (None, 0.0, 0.0)
    };

    // Determine dates
    let entry_date = buys
        .iter()
        .chain(sells.iter())
        .map(|t| &t.trade_date)
        .min()
        .ok_or_else(|| {
            error!("No trade dates found");
            actix_web::error::ErrorInternalServerError("No trade dates")
        })?;

    let exit_date = if !sells.is_empty() {
        sells.iter().map(|t| &t.trade_date).max()
    } else {
        None
    };

    // Parse dates
    let parse_date = |date_str: &str| -> Result<chrono::DateTime<Utc>, actix_web::Error> {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(date_str) {
            return Ok(dt.with_timezone(&Utc));
        }
        if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S") {
            return Ok(chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
        }
        if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            let ndt = date
                .and_hms_opt(0, 0, 0)
                .ok_or_else(|| actix_web::error::ErrorInternalServerError("Invalid date"))?;
            return Ok(chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
        }
        Err(actix_web::error::ErrorInternalServerError(
            "Unsupported date format",
        ))
    };

    let entry_date_parsed = parse_date(entry_date)?;
    let _exit_date_parsed = exit_date.map(|d| parse_date(d)).transpose()?;

    // Create trade based on type
    if request.trade_type == "stock" {
        let order_type = request
            .order_type
            .parse::<OrderType>()
            .map_err(|_| actix_web::error::ErrorBadRequest("Invalid order_type"))?;

        // Generate trade_group_id for linking related trades
        let trade_group_id = Uuid::new_v4().to_string();

        // Create position ID
        let position_id = Uuid::new_v4().to_string();

        // Sort transactions by date
        let mut all_transactions: Vec<(&TransactionData, usize)> = transactions
            .iter()
            .enumerate()
            .map(|(idx, t)| (t, idx))
            .collect();
        all_transactions.sort_by_key(|(t, _)| &t.trade_date);

        let mut created_trades = Vec::new();
        let mut parent_trade_id: Option<i64> = None;
        let mut sequence = 1;

        // Create trade records for each transaction
        for (txn, _) in all_transactions {
            let is_entry = txn.transaction_type.as_deref() == Some("BUY");
            let trade_type = if is_entry {
                TradeType::BUY
            } else {
                TradeType::SELL
            };

            let txn_date = parse_date(&txn.trade_date)?;
            let txn_price = txn.price.unwrap_or(0.0);
            let txn_quantity = txn.quantity.unwrap_or(0.0);
            let txn_fees = txn.fees.unwrap_or(0.0);

            let create_request = CreateStockRequest {
                symbol: request.symbol.clone(),
                trade_type,
                order_type: order_type.clone(),
                entry_price: if is_entry { txn_price } else { entry_price },
                exit_price: txn_price,
                stop_loss: request.stop_loss.unwrap_or(entry_price * 0.95),
                commissions: txn_fees,
                number_shares: txn_quantity,
                take_profit: request.take_profit,
                initial_target: request.initial_target,
                profit_target: request.profit_target,
                trade_ratings: request.trade_ratings,
                entry_date: if is_entry {
                    txn_date
                } else {
                    entry_date_parsed
                },
                exit_date: txn_date,
                reviewed: request.reviewed,
                mistakes: request.mistakes.clone(),
                brokerage_name: request.brokerage_name.clone(),
                trade_group_id: Some(trade_group_id.clone()),
                parent_trade_id,
                total_quantity: Some(txn_quantity),
                transaction_sequence: Some(sequence),
            };

            match Stock::create(&conn, create_request).await {
                Ok(stock) => {
                    if parent_trade_id.is_none() {
                        parent_trade_id = Some(stock.id);
                    }
                    created_trades.push(stock.id);

                    // Create/update position
                    let trans_type = if is_entry { "entry" } else { "exit" };
                    crate::service::position_service::update_position_quantity(
                        &conn,
                        &position_id,
                        "stock",
                        &request.symbol,
                        txn_quantity,
                        txn_price,
                        trans_type,
                        request.brokerage_name.as_deref(),
                        None, // option_type (not needed for stocks)
                        None, // strike_price (not needed for stocks)
                        None, // expiration_date (not needed for stocks)
                    )
                    .await
                    .map_err(|e| {
                        error!("Failed to update position: {}", e);
                        actix_web::error::ErrorInternalServerError("Failed to update position")
                    })?;

                    // Log transaction
                    let trans_log_id = Uuid::new_v4().to_string();
                    conn.execute(
                        "INSERT INTO position_transactions (id, position_id, position_type, trade_id, transaction_type, quantity, price, transaction_date) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                        libsql::params![
                            trans_log_id,
                            position_id.clone(),
                            "stock",
                            stock.id,
                            trans_type,
                            txn_quantity,
                            txn_price,
                            txn_date.to_rfc3339()
                        ]
                    ).await.map_err(|e| {
                        error!("Failed to log transaction: {}", e);
                        actix_web::error::ErrorInternalServerError("Failed to log transaction")
                    })?;

                    sequence += 1;
                }
                Err(e) => {
                    error!("Failed to create stock trade: {}", e);
                    return Ok(HttpResponse::InternalServerError()
                        .json(ApiResponse::<()>::error("Failed to create stock trade")));
                }
            }
        }

        // Mark transactions as transformed
        let update_sql = format!(
            "UPDATE brokerage_transactions SET is_transformed = 1 WHERE id IN ({})",
            placeholders
        );
        let update_params: Vec<&str> = request.transaction_ids.iter().map(|s| s.as_str()).collect();

        let update_stmt = conn.prepare(&update_sql).await.map_err(|e| {
            error!("Failed to prepare update statement: {}", e);
            actix_web::error::ErrorInternalServerError("Database error")
        })?;

        update_stmt.query(update_params).await.map_err(|e| {
            error!("Failed to update transactions: {}", e);
            actix_web::error::ErrorInternalServerError("Database error")
        })?;

        // Vectorize all created stock trades with structured format
        let trade_vectorization_clone = trade_vectorization.get_ref().clone();
        let user_id_vec = user_id.clone();
        let conn_clone = conn.clone();
        let created_trades_clone = created_trades.clone();

        tokio::spawn(async move {
            for trade_id in created_trades_clone {
                if let Err(e) = trade_vectorization_clone
                    .vectorize_trade(&user_id_vec, trade_id, "stock", &conn_clone)
                    .await
                {
                    error!("Failed to vectorize trade for stock {}: {}", trade_id, e);
                } else {
                    info!("Successfully vectorized trade for stock {}", trade_id);
                }
            }
        });

        info!(
            "Successfully merged {} transactions into {} stock trades (group: {})",
            request.transaction_ids.len(),
            created_trades.len(),
            trade_group_id
        );
        Ok(
            HttpResponse::Created().json(ApiResponse::success(serde_json::json!({
                "trade_group_id": trade_group_id,
                "created_trades": created_trades,
                "position_id": position_id
            }))),
        )
    } else if request.trade_type == "option" {
        let option_type_str = request.option_type.clone().ok_or_else(|| {
            actix_web::error::ErrorBadRequest("option_type required for option trades")
        })?;
        let option_type = option_type_str
            .parse::<OptionType>()
            .map_err(|_| actix_web::error::ErrorBadRequest("Invalid option_type"))?;
        let strike_price = request.strike_price.ok_or_else(|| {
            actix_web::error::ErrorBadRequest("strike_price required for option trades")
        })?;
        let expiration_date = request.expiration_date.clone().ok_or_else(|| {
            actix_web::error::ErrorBadRequest("expiration_date required for option trades")
        })?;
        let expiration_date_parsed = parse_date(&expiration_date)?;

        // Generate trade_group_id for linking related trades
        let trade_group_id = Uuid::new_v4().to_string();

        // Create position ID
        let position_id = Uuid::new_v4().to_string();

        // Sort transactions by date
        let mut all_transactions: Vec<(&TransactionData, usize)> = transactions
            .iter()
            .enumerate()
            .map(|(idx, t)| (t, idx))
            .collect();
        all_transactions.sort_by_key(|(t, _)| &t.trade_date);

        let mut created_trades = Vec::new();
        let mut parent_trade_id: Option<i64> = None;
        let mut sequence = 1;

        // Create trade records for each transaction
        for (txn, _) in all_transactions {
            let is_entry = txn.transaction_type.as_deref() == Some("BUY");

            let txn_date = parse_date(&txn.trade_date)?;
            let txn_price = txn.price.unwrap_or(0.0);
            let txn_quantity = txn.quantity.unwrap_or(0.0);
            let _txn_fees = txn.fees.unwrap_or(0.0);
            let premium = txn_price * txn_quantity;

            let create_request = CreateOptionRequest {
                symbol: request.symbol.clone(),
                option_type: option_type.clone(),
                strike_price,
                expiration_date: expiration_date_parsed,
                entry_price: if is_entry { txn_price } else { entry_price },
                exit_price: txn_price,
                premium,
                entry_date: if is_entry {
                    txn_date
                } else {
                    entry_date_parsed
                },
                exit_date: txn_date,
                initial_target: request.initial_target,
                profit_target: request.profit_target,
                trade_ratings: request.trade_ratings,
                reviewed: request.reviewed,
                mistakes: request.mistakes.clone(),
                brokerage_name: request.brokerage_name.clone(),
                trade_group_id: Some(trade_group_id.clone()),
                parent_trade_id,
                total_quantity: Some(txn_quantity),
                transaction_sequence: Some(sequence),
            };

            match OptionTrade::create(&conn, create_request).await {
                Ok(option) => {
                    if parent_trade_id.is_none() {
                        parent_trade_id = Some(option.id);
                    }
                    created_trades.push(option.id);

                    // Create/update position
                    let trans_type = if is_entry { "entry" } else { "exit" };
                    crate::service::position_service::update_position_quantity(
                        &conn,
                        &position_id,
                        "option",
                        &request.symbol,
                        txn_quantity,
                        txn_price,
                        trans_type,
                        request.brokerage_name.as_deref(),
                        Some(option_type_str.as_str()),
                        Some(strike_price),
                        Some(expiration_date.as_str()),
                    )
                    .await
                    .map_err(|e| {
                        error!("Failed to update position: {}", e);
                        actix_web::error::ErrorInternalServerError("Failed to update position")
                    })?;

                    // Log transaction
                    let trans_log_id = Uuid::new_v4().to_string();
                    conn.execute(
                        "INSERT INTO position_transactions (id, position_id, position_type, trade_id, transaction_type, quantity, price, transaction_date) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                        libsql::params![
                            trans_log_id,
                            position_id.clone(),
                            "option",
                            option.id,
                            trans_type,
                            txn_quantity,
                            txn_price,
                            txn_date.to_rfc3339()
                        ]
                    ).await.map_err(|e| {
                        error!("Failed to log transaction: {}", e);
                        actix_web::error::ErrorInternalServerError("Failed to log transaction")
                    })?;

                    sequence += 1;
                }
                Err(e) => {
                    error!("Failed to create option trade: {}", e);
                    return Ok(HttpResponse::InternalServerError()
                        .json(ApiResponse::<()>::error("Failed to create option trade")));
                }
            }
        }

        // Mark transactions as transformed
        let update_sql = format!(
            "UPDATE brokerage_transactions SET is_transformed = 1 WHERE id IN ({})",
            placeholders
        );
        let update_params: Vec<&str> = request.transaction_ids.iter().map(|s| s.as_str()).collect();

        let update_stmt = conn.prepare(&update_sql).await.map_err(|e| {
            error!("Failed to prepare update statement: {}", e);
            actix_web::error::ErrorInternalServerError("Database error")
        })?;

        update_stmt.query(update_params).await.map_err(|e| {
            error!("Failed to update transactions: {}", e);
            actix_web::error::ErrorInternalServerError("Database error")
        })?;

        // Vectorize all created option trades with structured format
        let trade_vectorization_clone = trade_vectorization.get_ref().clone();
        let user_id_vec = user_id.clone();
        let conn_clone = conn.clone();
        let created_trades_clone = created_trades.clone();

        tokio::spawn(async move {
            for trade_id in created_trades_clone {
                if let Err(e) = trade_vectorization_clone
                    .vectorize_trade(&user_id_vec, trade_id, "option", &conn_clone)
                    .await
                {
                    error!("Failed to vectorize trade for option {}: {}", trade_id, e);
                } else {
                    info!("Successfully vectorized trade for option {}", trade_id);
                }
            }
        });

        info!(
            "Successfully merged {} transactions into {} option trades (group: {})",
            request.transaction_ids.len(),
            created_trades.len(),
            trade_group_id
        );
        Ok(
            HttpResponse::Created().json(ApiResponse::success(serde_json::json!({
                "trade_group_id": trade_group_id,
                "created_trades": created_trades,
                "position_id": position_id
            }))),
        )
    } else {
        Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(
            "Invalid trade_type. Must be 'stock' or 'option'",
        )))
    }
}

/// Configure brokerage routes
pub fn configure_brokerage_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/brokerage")
            .route("/connections/initiate", web::post().to(initiate_connection))
            .route("/connections", web::get().to(list_connections))
            .route(
                "/connections/{id}/status",
                web::get().to(get_connection_status),
            )
            .route(
                "/connections/{id}/complete",
                web::post().to(complete_connection_sync),
            )
            .route("/connections/{id}", web::delete().to(delete_connection))
            .route("/accounts", web::get().to(list_accounts))
            .route("/accounts/{id}/detail", web::get().to(get_account_detail))
            .route(
                "/accounts/{id}/positions",
                web::get().to(get_account_positions),
            )
            .route(
                "/accounts/{id}/positions/options",
                web::get().to(get_account_option_positions),
            )
            .route(
                "/accounts/{id}/transactions",
                web::get().to(get_account_transactions),
            )
            .route("/accounts/sync", web::post().to(sync_accounts))
            .route("/transactions", web::get().to(get_transactions))
            .route("/holdings", web::get().to(get_holdings))
            .route(
                "/unmatched-transactions",
                web::get().to(get_unmatched_transactions),
            )
            .route(
                "/unmatched-transactions/{id}/resolve",
                web::post().to(resolve_unmatched_transaction),
            )
            .route(
                "/unmatched-transactions/{id}/ignore",
                web::post().to(ignore_unmatched_transaction),
            )
            .route(
                "/unmatched-transactions/{id}/suggestions",
                web::get().to(get_unmatched_suggestions),
            )
            .route("/transactions/merge", web::post().to(merge_transactions)),
    ); // Semi colon 
}
