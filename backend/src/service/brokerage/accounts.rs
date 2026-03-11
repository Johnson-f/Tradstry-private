use anyhow::{Context, Result};
use chrono::Utc;
use libsql::Connection;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

use crate::service::brokerage::client::SnapTradeClient;
use crate::service::brokerage::helpers::{
    get_existing_account_id, get_existing_holding_id, transaction_exists,
};
use crate::service::brokerage::transform;

#[derive(Serialize)]
pub struct SyncSummary {
    pub accounts_synced: usize,
    pub holdings_synced: usize,
    pub transactions_synced: usize,
    pub last_sync_at: String,
}

#[derive(Deserialize)]
pub struct SyncAccountsResponse {
    pub accounts: Vec<Value>,
    pub holdings: Vec<Value>,
    pub transactions: Vec<Value>,
}

/// List all accounts for a user
pub async fn list_accounts(conn: &Connection, user_id: &str) -> Result<Vec<Value>> {
    let stmt = conn
        .prepare("SELECT id, connection_id, snaptrade_account_id, account_number, account_name, account_type, balance, currency, institution_name, created_at, updated_at FROM brokerage_accounts WHERE connection_id IN (SELECT id FROM brokerage_connections WHERE user_id = ?) ORDER BY created_at DESC")
        .await
        .context("Database error")?;

    let mut result = stmt
        .query(libsql::params![user_id])
        .await
        .context("Database query error")?;

    let mut accounts = Vec::new();
    while let Some(row) = result.next().await? {
        let id: String = row.get(0)?;
        let connection_id: String = row.get(1)?;
        let snaptrade_account_id: String = row.get(2)?;
        let account_number: Option<String> = row.get(3)?;
        let account_name: Option<String> = row.get(4)?;
        let account_type: Option<String> = row.get(5)?;
        let balance: Option<f64> = row.get(6)?;
        let currency: Option<String> = row.get(7)?;
        let institution_name: Option<String> = row.get(8)?;
        let created_at: String = row.get(9)?;
        let updated_at: String = row.get(10)?;

        accounts.push(serde_json::json!({
            "id": id,
            "connection_id": connection_id,
            "snaptrade_account_id": snaptrade_account_id,
            "account_number": account_number,
            "account_name": account_name,
            "account_type": account_type,
            "balance": balance,
            "currency": currency,
            "institution_name": institution_name,
            "created_at": created_at,
            "updated_at": updated_at
        }));
    }

    Ok(accounts)
}

/// Get account detail from SnapTrade
pub async fn get_account_detail(
    conn: &Connection,
    account_id: &str,
    user_id: &str,
    snaptrade_client: &SnapTradeClient,
) -> Result<Value> {
    // Get user secret and verify account belongs to user
    let stmt = conn
        .prepare("SELECT snaptrade_user_secret, snaptrade_account_id FROM brokerage_accounts WHERE id = ? AND connection_id IN (SELECT id FROM brokerage_connections WHERE user_id = ?)")
        .await
        .context("Database error")?;

    let mut result = stmt
        .query(libsql::params![account_id, user_id])
        .await
        .context("Database query error")?;

    let (user_secret, snaptrade_account_id) = if let Some(row) = result.next().await? {
        let secret: String = row.get(0)?;
        let snaptrade_id: String = row.get(1)?;
        (secret, snaptrade_id)
    } else {
        return Err(anyhow::anyhow!("Account not found"));
    };

    // Call Go service to get account detail
    let response = snaptrade_client
        .call_go_service(
            "GET",
            &format!("/api/v1/accounts/{}", snaptrade_account_id),
            None::<&Value>,
            user_id,
            Some(&user_secret),
        )
        .await
        .context("Failed to call SnapTrade service")?;

    if !response.status().is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(anyhow::anyhow!("SnapTrade service error: {}", error_text));
    }

    let account_detail: Value = response.json().await.context("Failed to parse response")?;

    Ok(account_detail)
}

/// Sync accounts, holdings, and transactions from SnapTrade
pub async fn sync_accounts(
    conn: &Connection,
    user_id: &str,
    snaptrade_client: &SnapTradeClient,
) -> Result<SyncSummary> {
    info!("Syncing accounts for user: {}", user_id);

    // Get all connections for user
    let stmt = conn
        .prepare("SELECT id, snaptrade_user_secret FROM brokerage_connections WHERE user_id = ? AND status = 'connected'")
        .await
        .context("Database error")?;

    let mut result = stmt
        .query(libsql::params![user_id])
        .await
        .context("Database query error")?;

    let mut total_accounts = 0;
    let mut total_holdings = 0;
    let mut total_transactions = 0;

    while let Some(row) = result.next().await? {
        let connection_id: String = row.get(0)?;
        let user_secret: String = row.get(1)?;

        // Call Go service to sync
        let sync_req = serde_json::json!({
            "user_secret": user_secret
        });

        let response = match snaptrade_client
            .call_go_service(
                "POST",
                "/api/v1/accounts/sync",
                Some(&sync_req),
                user_id,
                None,
            )
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                error!(
                    "Failed to sync accounts for connection {}: {}",
                    connection_id, e
                );
                continue; // Continue with other connections
            }
        };

        if !response.status().is_success() {
            error!("Sync failed for connection {}", connection_id);
            continue; // Continue with other connections
        }

        let sync_data: SyncAccountsResponse = match response.json().await {
            Ok(data) => data,
            Err(e) => {
                error!(
                    "Failed to parse sync response for connection {}: {}",
                    connection_id, e
                );
                continue; // Continue with other connections
            }
        };

        // Store synced data in database
        for account in sync_data.accounts {
            if account.get("id").is_some() {
                let snaptrade_account_id = account.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let account_number = account.get("account_number").and_then(|v| v.as_str());
                let account_name = account.get("name").and_then(|v| v.as_str());
                let account_type = account.get("type").and_then(|v| v.as_str());
                let balance = account.get("balance").and_then(|v| v.as_f64());
                let currency = account
                    .get("currency")
                    .and_then(|v| v.as_str())
                    .unwrap_or("USD");
                let institution_name = account.get("institution_name").and_then(|v| v.as_str());

                // Check if account already exists to prevent duplicates
                let account_uuid =
                    match get_existing_account_id(conn, &connection_id, snaptrade_account_id).await
                    {
                        Some(existing_id) => {
                            info!(
                                "Account {} already exists, updating: {}",
                                snaptrade_account_id, existing_id
                            );
                            existing_id
                        }
                        None => {
                            let new_id = Uuid::new_v4().to_string();
                            info!(
                                "Creating new account: {} with ID: {}",
                                snaptrade_account_id, new_id
                            );
                            new_id
                        }
                    };

                let now = Utc::now().to_rfc3339();
                let raw_data = serde_json::to_string(&account).unwrap_or_default();

                conn.execute(
                    "INSERT OR REPLACE INTO brokerage_accounts (id, connection_id, snaptrade_account_id, account_number, account_name, account_type, balance, currency, institution_name, raw_data, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, COALESCE((SELECT created_at FROM brokerage_accounts WHERE id = ?), ?), ?)",
                    libsql::params![
                        account_uuid.clone(),
                        connection_id.clone(),
                        snaptrade_account_id,
                        account_number,
                        account_name,
                        account_type,
                        balance,
                        currency,
                        institution_name,
                        raw_data,
                        account_uuid.clone(),
                        now.clone(),
                        now
                    ],
                ).await.ok(); // Don't fail entire sync if one account fails

                total_accounts += 1;

                // Store holdings for this account
                for holding in &sync_data.holdings {
                    if let Some(holding_account_id) =
                        holding.get("account_id").and_then(|v| v.as_str())
                        && holding_account_id == snaptrade_account_id
                    {
                        let symbol = holding.get("symbol").and_then(|v| v.as_str()).unwrap_or("");
                        if symbol.is_empty() {
                            continue; // Skip holdings without symbols
                        }

                        let quantity = holding
                            .get("quantity")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0);
                        let average_cost = holding.get("average_cost").and_then(|v| v.as_f64());
                        let current_price = holding.get("current_price").and_then(|v| v.as_f64());
                        let market_value = holding.get("market_value").and_then(|v| v.as_f64());
                        let currency = holding
                            .get("currency")
                            .and_then(|v| v.as_str())
                            .unwrap_or("USD");
                        let raw_data = serde_json::to_string(holding).unwrap_or_default();

                        // Check if holding already exists to prevent duplicates
                        let holding_uuid =
                            match get_existing_holding_id(conn, &account_uuid, symbol).await {
                                Some(existing_id) => {
                                    info!(
                                        "Holding {} for account {} already exists, updating: {}",
                                        symbol, account_uuid, existing_id
                                    );
                                    existing_id
                                }
                                None => {
                                    let new_id = Uuid::new_v4().to_string();
                                    info!(
                                        "Creating new holding: {} for account: {}",
                                        symbol, account_uuid
                                    );
                                    new_id
                                }
                            };

                        let now = Utc::now().to_rfc3339();

                        conn.execute(
                                "INSERT OR REPLACE INTO brokerage_holdings (id, account_id, symbol, quantity, average_cost, current_price, market_value, currency, last_updated, raw_data) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                                libsql::params![
                                    holding_uuid,
                                    account_uuid.clone(),
                                    symbol,
                                    quantity,
                                    average_cost,
                                    current_price,
                                    market_value,
                                    currency,
                                    now,
                                    raw_data
                                ],
                            ).await.ok();

                        total_holdings += 1;
                    }
                }

                // Store transactions for this account
                for transaction in &sync_data.transactions {
                    if let Some(trans_account_id) =
                        transaction.get("account_id").and_then(|v| v.as_str())
                        && trans_account_id == snaptrade_account_id
                    {
                        let snaptrade_transaction_id =
                            transaction.get("id").and_then(|v| v.as_str()).unwrap_or("");
                        if snaptrade_transaction_id.is_empty() {
                            continue; // Skip transactions without IDs
                        }

                        // Check if transaction already exists to prevent duplicates
                        if transaction_exists(conn, &account_uuid, snaptrade_transaction_id).await {
                            info!(
                                "Transaction {} for account {} already exists, skipping",
                                snaptrade_transaction_id, account_uuid
                            );
                            continue;
                        }

                        let symbol = transaction.get("symbol").and_then(|v| v.as_str());
                        let transaction_type = transaction.get("type").and_then(|v| v.as_str());
                        let quantity = transaction.get("quantity").and_then(|v| v.as_f64());
                        let price = transaction.get("price").and_then(|v| v.as_f64());
                        let amount = transaction.get("amount").and_then(|v| v.as_f64());
                        let currency = transaction
                            .get("currency")
                            .and_then(|v| v.as_str())
                            .unwrap_or("USD");
                        let default_trade_date = Utc::now().to_rfc3339();
                        let trade_date = transaction
                            .get("date")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&default_trade_date);
                        let settlement_date =
                            transaction.get("settlement_date").and_then(|v| v.as_str());
                        let fees = transaction.get("fees").and_then(|v| v.as_f64());
                        let raw_data = serde_json::to_string(transaction).unwrap_or_default();

                        let transaction_uuid = Uuid::new_v4().to_string();
                        let transaction_now = Utc::now().to_rfc3339();

                        conn.execute(
                                "INSERT INTO brokerage_transactions (id, account_id, snaptrade_transaction_id, symbol, transaction_type, quantity, price, amount, currency, trade_date, settlement_date, fees, raw_data, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                                libsql::params![
                                    transaction_uuid,
                                    account_uuid.clone(),
                                    snaptrade_transaction_id,
                                    symbol,
                                    transaction_type,
                                    quantity,
                                    price,
                                    amount,
                                    currency,
                                    trade_date,
                                    settlement_date,
                                    fees,
                                    raw_data,
                                    transaction_now.clone(),
                                    transaction_now
                                ],
                            ).await.ok();

                        total_transactions += 1;
                    }
                }
            }
        }

        // Update last_sync_at
        let sync_now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE brokerage_connections SET last_sync_at = ?, updated_at = ? WHERE id = ?",
            libsql::params![sync_now.clone(), sync_now, connection_id],
        )
        .await
        .ok();
    }

    // Transform brokerage transactions to stocks/options trades
    info!("Starting transformation of brokerage transactions to trades");
    if let Err(e) = transform::migrate_add_brokerage_name_column(conn).await {
        warn!("Failed to migrate brokerage_name column: {}", e);
    }

    if let Err(e) = transform::transform_brokerage_transactions(conn, user_id, None).await {
        error!("Failed to transform brokerage transactions: {}", e);
        // Don't fail the entire sync if transformation fails
    }

    Ok(SyncSummary {
        accounts_synced: total_accounts,
        holdings_synced: total_holdings,
        transactions_synced: total_transactions,
        last_sync_at: Utc::now().to_rfc3339(),
    })
}

/// Get account equity positions
pub async fn get_account_positions(
    conn: &Connection,
    account_id: &str,
    user_id: &str,
    snaptrade_client: &SnapTradeClient,
) -> Result<Value> {
    // Get user secret and verify account belongs to user
    let stmt = conn
        .prepare("SELECT snaptrade_user_secret, snaptrade_account_id FROM brokerage_accounts WHERE id = ? AND connection_id IN (SELECT id FROM brokerage_connections WHERE user_id = ?)")
        .await
        .context("Database error")?;

    let mut result = stmt
        .query(libsql::params![account_id, user_id])
        .await
        .context("Database query error")?;

    let (user_secret, snaptrade_account_id) = if let Some(row) = result.next().await? {
        let secret: String = row.get(0)?;
        let snaptrade_id: String = row.get(1)?;
        (secret, snaptrade_id)
    } else {
        return Err(anyhow::anyhow!("Account not found"));
    };

    // Call Go service to get equity positions
    let response = snaptrade_client
        .call_go_service(
            "GET",
            &format!("/api/v1/accounts/{}/holdings", snaptrade_account_id),
            None::<&Value>,
            user_id,
            Some(&user_secret),
        )
        .await
        .context("Failed to call SnapTrade service")?;

    let status_code = response.status().as_u16();
    if !response.status().is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        // Handle empty positions gracefully - return empty array
        if status_code == 404 || status_code == 500 {
            return Ok(serde_json::json!({
                "positions": []
            }));
        }
        return Err(anyhow::anyhow!("SnapTrade service error: {}", error_text));
    }

    let positions_data: Value = response.json().await.context("Failed to parse response")?;

    Ok(positions_data)
}

/// Get account option positions
pub async fn get_account_option_positions(
    conn: &Connection,
    account_id: &str,
    user_id: &str,
    snaptrade_client: &SnapTradeClient,
) -> Result<Value> {
    // Get user secret and verify account belongs to user
    let stmt = conn
        .prepare("SELECT snaptrade_user_secret, snaptrade_account_id FROM brokerage_accounts WHERE id = ? AND connection_id IN (SELECT id FROM brokerage_connections WHERE user_id = ?)")
        .await
        .context("Database error")?;

    let mut result = stmt
        .query(libsql::params![account_id, user_id])
        .await
        .context("Database query error")?;

    let (user_secret, snaptrade_account_id) = if let Some(row) = result.next().await? {
        let secret: String = row.get(0)?;
        let snaptrade_id: String = row.get(1)?;
        (secret, snaptrade_id)
    } else {
        return Err(anyhow::anyhow!("Account not found"));
    };

    // Call Go service to get option positions
    let response = snaptrade_client
        .call_go_service(
            "GET",
            &format!("/api/v1/accounts/{}/holdings/options", snaptrade_account_id),
            None::<&Value>,
            user_id,
            Some(&user_secret),
        )
        .await
        .context("Failed to call SnapTrade service")?;

    let status_code = response.status().as_u16();
    if !response.status().is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        // Handle empty positions gracefully - return empty array
        if status_code == 404 || status_code == 500 {
            return Ok(serde_json::json!({
                "positions": []
            }));
        }
        return Err(anyhow::anyhow!("SnapTrade service error: {}", error_text));
    }

    let positions_data: Value = response.json().await.context("Failed to parse response")?;

    Ok(positions_data)
}

/// Get account transactions from SnapTrade
pub async fn get_account_transactions_from_snaptrade(
    conn: &Connection,
    account_id: &str,
    user_id: &str,
    query_params: &HashMap<String, String>,
    snaptrade_client: &SnapTradeClient,
) -> Result<Value> {
    // Get user secret and verify account belongs to user
    let stmt = conn
        .prepare("SELECT snaptrade_user_secret, snaptrade_account_id FROM brokerage_accounts WHERE id = ? AND connection_id IN (SELECT id FROM brokerage_connections WHERE user_id = ?)")
        .await
        .context("Database error")?;

    let mut result = stmt
        .query(libsql::params![account_id, user_id])
        .await
        .context("Database query error")?;

    let (user_secret, snaptrade_account_id) = if let Some(row) = result.next().await? {
        let secret: String = row.get(0)?;
        let snaptrade_id: String = row.get(1)?;
        (secret, snaptrade_id)
    } else {
        return Err(anyhow::anyhow!("Account not found"));
    };

    // Build query string with pagination params
    let mut query_string = String::new();
    if let Some(start_date) = query_params.get("start_date") {
        query_string.push_str(&format!("&start_date={}", start_date));
    }
    if let Some(end_date) = query_params.get("end_date") {
        query_string.push_str(&format!("&end_date={}", end_date));
    }
    if let Some(offset) = query_params.get("offset") {
        query_string.push_str(&format!("&offset={}", offset));
    }
    if let Some(limit) = query_params.get("limit") {
        query_string.push_str(&format!("&limit={}", limit));
    }

    let path = if query_string.is_empty() {
        format!("/api/v1/accounts/{}/transactions", snaptrade_account_id)
    } else {
        format!(
            "/api/v1/accounts/{}/transactions?{}",
            snaptrade_account_id,
            &query_string[1..]
        ) // Skip first &
    };

    // Call Go service to get transactions
    let response = snaptrade_client
        .call_go_service("GET", &path, None::<&Value>, user_id, Some(&user_secret))
        .await
        .context("Failed to call SnapTrade service")?;

    let status_code = response.status().as_u16();
    if !response.status().is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        // Handle empty transactions gracefully - return empty array
        if status_code == 404 || status_code == 500 {
            return Ok(serde_json::json!({
                "data": [],
                "pagination": {
                    "offset": query_params.get("offset").and_then(|s| s.parse::<i32>().ok()).unwrap_or(0),
                    "limit": query_params.get("limit").and_then(|s| s.parse::<i32>().ok()).unwrap_or(1000),
                    "total": 0
                }
            }));
        }
        return Err(anyhow::anyhow!("SnapTrade service error: {}", error_text));
    }

    let transactions_data: Value = response.json().await.context("Failed to parse response")?;

    Ok(transactions_data)
}
