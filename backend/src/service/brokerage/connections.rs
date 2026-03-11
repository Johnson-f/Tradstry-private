use anyhow::{Context, Result};
use chrono::Utc;
use libsql::Connection;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::service::brokerage::client::SnapTradeClient;
use crate::service::brokerage::helpers::{
    get_existing_account_id, get_existing_holding_id, get_or_create_snaptrade_user,
    transaction_exists,
};
use crate::service::brokerage::transform;

#[derive(Serialize, Deserialize)]
pub struct ConnectBrokerageResponse {
    pub redirect_url: String,
    pub connection_id: String,
}

#[derive(Serialize)]
pub struct SyncSummary {
    pub accounts_synced: usize,
    pub holdings_synced: usize,
    pub transactions_synced: usize,
    pub last_sync_at: String,
}

/// Initiate a brokerage connection
pub async fn initiate_connection(
    conn: &Connection,
    user_id: &str,
    brokerage_id: &str,
    connection_type: Option<&str>,
    snaptrade_client: &SnapTradeClient,
) -> Result<ConnectBrokerageResponse> {
    info!("Initiating connection for user: {}", user_id);

    // Get or create SnapTrade user
    let (snaptrade_user_id, snaptrade_user_secret) =
        get_or_create_snaptrade_user(conn, user_id, snaptrade_client).await?;

    // Call Go service to generate connection portal URL
    let initiate_req = serde_json::json!({
        "brokerage_id": brokerage_id,
        "connection_type": connection_type.unwrap_or("read"),
        "user_secret": snaptrade_user_secret
    });

    let response = snaptrade_client
        .call_go_service(
            "POST",
            "/api/v1/connections/initiate",
            Some(&initiate_req),
            user_id,
            None,
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

    let response_text = response
        .text()
        .await
        .context("Failed to read response text")?;

    info!("SnapTrade service response: {}", response_text);

    let data: ConnectBrokerageResponse = serde_json::from_str(&response_text).context(format!(
        "Failed to parse response JSON. Response was: {}",
        response_text
    ))?;

    info!(
        "Parsed connection response - redirect_url: {}, connection_id: {}",
        data.redirect_url, data.connection_id
    );

    // Store connection in database
    let connection_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO brokerage_connections (id, user_id, snaptrade_user_id, snaptrade_user_secret, connection_id, brokerage_name, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        libsql::params![
            connection_id,
            user_id,
            snaptrade_user_id,
            snaptrade_user_secret,
            data.connection_id.clone(),
            brokerage_id,
            "pending",
            now.clone(),
            now
        ],
    ).await.context("Failed to store connection")?;

    Ok(data)
}

/// Get connection status
pub async fn get_connection_status(
    conn: &Connection,
    connection_id: &str,
    user_id: &str,
    snaptrade_client: &SnapTradeClient,
) -> Result<Value> {
    // Get connection from database
    let stmt = conn
        .prepare("SELECT snaptrade_user_secret, connection_id FROM brokerage_connections WHERE id = ? AND user_id = ?")
        .await
        .context("Database error")?;

    let mut result = stmt
        .query(libsql::params![connection_id, user_id])
        .await
        .context("Database query error")?;

    let (user_secret, snaptrade_connection_id) = if let Some(row) = result.next().await? {
        let secret: String = row.get(0)?;
        let conn_id: Option<String> = row.get(1)?;
        (secret, conn_id)
    } else {
        return Err(anyhow::anyhow!("Connection not found"));
    };

    let snaptrade_connection_id =
        snaptrade_connection_id.ok_or_else(|| anyhow::anyhow!("Connection ID not found"))?;

    // Call Go service to check status
    let response = snaptrade_client
        .call_go_service(
            "GET",
            &format!("/api/v1/connections/{}/status", snaptrade_connection_id),
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

    let status_data: Value = response.json().await.context("Failed to parse response")?;

    // Update database if connection is completed
    if let Some(status_str) = status_data.get("status").and_then(|s| s.as_str())
        && status_str == "connected"
    {
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE brokerage_connections SET status = ?, updated_at = ? WHERE id = ?",
            libsql::params!["connected", now, connection_id],
        )
        .await
        .ok(); // Don't fail if update fails
    }

    Ok(status_data)
}

/// List all connections for a user
pub async fn list_connections(conn: &Connection, user_id: &str) -> Result<Vec<Value>> {
    let stmt = conn
        .prepare("SELECT id, connection_id, brokerage_name, status, last_sync_at, created_at, updated_at FROM brokerage_connections WHERE user_id = ? ORDER BY created_at DESC")
        .await
        .context("Database error")?;

    let mut result = stmt
        .query(libsql::params![user_id])
        .await
        .context("Database query error")?;

    let mut connections = Vec::new();
    while let Some(row) = result.next().await? {
        let id: String = row.get(0)?;
        let connection_id: Option<String> = row.get(1)?;
        let brokerage_name: String = row.get(2)?;
        let status: String = row.get(3)?;
        let last_sync_at: Option<String> = row.get(4)?;
        let created_at: String = row.get(5)?;
        let updated_at: String = row.get(6)?;

        connections.push(serde_json::json!({
            "id": id,
            "connection_id": connection_id,
            "brokerage_name": brokerage_name,
            "status": status,
            "last_sync_at": last_sync_at,
            "created_at": created_at,
            "updated_at": updated_at
        }));
    }

    Ok(connections)
}

/// Delete a connection
pub async fn delete_connection(
    conn: &Connection,
    connection_id: &str,
    user_id: &str,
    snaptrade_client: &SnapTradeClient,
) -> Result<()> {
    // Get connection details
    let stmt = conn
        .prepare("SELECT snaptrade_user_secret, connection_id FROM brokerage_connections WHERE id = ? AND user_id = ?")
        .await
        .context("Database error")?;

    let mut result = stmt
        .query(libsql::params![connection_id, user_id])
        .await
        .context("Database query error")?;

    let (user_secret, snaptrade_connection_id) = if let Some(row) = result.next().await? {
        let secret: String = row.get(0)?;
        let conn_id: Option<String> = row.get(1)?;
        (secret, conn_id)
    } else {
        return Err(anyhow::anyhow!("Connection not found"));
    };

    if let Some(snaptrade_conn_id) = snaptrade_connection_id {
        // Call Go service to delete from SnapTrade
        let _response = snaptrade_client
            .call_go_service(
                "DELETE",
                &format!("/api/v1/connections/{}", snaptrade_conn_id),
                None::<&Value>,
                user_id,
                Some(&user_secret),
            )
            .await
            .map_err(|e| {
                error!("Failed to delete from SnapTrade: {}", e);
                // Continue with database deletion even if SnapTrade deletion fails
            });
    }

    // Delete from database (cascade will handle accounts, transactions, holdings)
    conn.execute(
        "DELETE FROM brokerage_connections WHERE id = ? AND user_id = ?",
        libsql::params![connection_id, user_id],
    )
    .await
    .context("Failed to delete connection")?;

    Ok(())
}

/// Complete connection sync - sync accounts, holdings, and transactions
pub async fn complete_connection_sync(
    conn: &Connection,
    connection_id: &str,
    user_id: &str,
    snaptrade_client: &SnapTradeClient,
) -> Result<SyncSummary> {
    info!(
        "Completing connection sync for user: {}, connection: {}",
        user_id, connection_id
    );

    // Verify connection exists and get user secret, connection_id, and status
    let stmt = conn
        .prepare("SELECT snaptrade_user_secret, connection_id, status FROM brokerage_connections WHERE id = ? AND user_id = ?")
        .await
        .context("Database error")?;

    let mut result = stmt
        .query(libsql::params![connection_id, user_id])
        .await
        .context("Database query error")?;

    let (user_secret, snaptrade_connection_id, status) = if let Some(row) = result.next().await? {
        let secret: String = row.get(0)?;
        let conn_id: Option<String> = row.get(1)?;
        let conn_status: String = row.get(2)?;
        (secret, conn_id, conn_status)
    } else {
        return Err(anyhow::anyhow!("Connection not found"));
    };

    let snaptrade_connection_id =
        snaptrade_connection_id.ok_or_else(|| anyhow::anyhow!("Connection ID not found"))?;

    // Check actual connection status from SnapTrade if status is "pending"
    if status == "pending" {
        info!("Connection status is pending, checking actual status from SnapTrade");
        let status_response = snaptrade_client
            .call_go_service(
                "GET",
                &format!("/api/v1/connections/{}/status", snaptrade_connection_id),
                None::<&Value>,
                user_id,
                Some(&user_secret),
            )
            .await
            .context("Failed to check connection status")?;

        if status_response.status().is_success() {
            let status_data: Value = status_response
                .json()
                .await
                .context("Failed to parse status response")?;

            // Update database if connection is completed
            if let Some(status_str) = status_data.get("status").and_then(|s| s.as_str()) {
                if status_str == "connected" {
                    let now = Utc::now().to_rfc3339();
                    conn.execute(
                        "UPDATE brokerage_connections SET status = ?, updated_at = ? WHERE id = ?",
                        libsql::params!["connected", now, connection_id],
                    )
                    .await
                    .ok();
                    info!("Updated connection status from pending to connected");
                } else if status_str != "pending" {
                    return Err(anyhow::anyhow!(
                        "Connection is not ready. Current status: {}",
                        status_str
                    ));
                }
            }
        }
    } else if status != "connected" {
        return Err(anyhow::anyhow!(
            "Connection is not connected. Current status: {}",
            status
        ));
    }

    // Step 1: List all accounts for the user
    let list_req = serde_json::json!({
        "user_secret": user_secret
    });

    let accounts_response = snaptrade_client
        .call_go_service(
            "POST",
            "/api/v1/accounts/sync",
            Some(&list_req),
            user_id,
            None,
        )
        .await
        .context("Failed to list accounts")?;

    if !accounts_response.status().is_success() {
        let error_text = accounts_response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(anyhow::anyhow!("Failed to list accounts: {}", error_text));
    }

    #[derive(Deserialize)]
    struct SyncAccountsResponse {
        accounts: Vec<Value>,
        #[allow(dead_code)]
        holdings: Vec<Value>,
        #[allow(dead_code)]
        transactions: Vec<Value>,
    }

    let sync_data: SyncAccountsResponse = accounts_response
        .json()
        .await
        .context("Failed to parse accounts response")?;

    // Handle empty account list
    if sync_data.accounts.is_empty() {
        info!("No accounts found for user: {}", user_id);
        return Ok(SyncSummary {
            accounts_synced: 0,
            holdings_synced: 0,
            transactions_synced: 0,
            last_sync_at: Utc::now().to_rfc3339(),
        });
    }

    let mut total_accounts = 0;
    let mut total_holdings = 0;
    let mut total_transactions = 0;

    // Step 2: Store accounts and fetch/store positions and transactions for each account
    for account in sync_data.accounts {
        if let Some(account_obj) = account.as_object()
            && let Some(snaptrade_account_id) = account_obj.get("id").and_then(|v| v.as_str())
        {
            // Store account in database
            let account_number = account_obj.get("number").and_then(|v| v.as_str());
            let account_name = account_obj.get("name").and_then(|v| v.as_str());
            let account_type = account_obj
                .get("raw_type")
                .or_else(|| account_obj.get("type"))
                .and_then(|v| v.as_str());
            let balance_obj = account_obj.get("balance");
            let balance = balance_obj
                .and_then(|b| b.get("total"))
                .and_then(|t| t.get("amount"))
                .and_then(|v| v.as_f64());
            let currency = balance_obj
                .and_then(|b| b.get("total"))
                .and_then(|t| t.get("currency"))
                .and_then(|v| v.as_str())
                .unwrap_or("USD");
            let institution_name = account_obj.get("institution_name").and_then(|v| v.as_str());

            // Check if account already exists to prevent duplicates
            let account_uuid =
                match get_existing_account_id(conn, connection_id, snaptrade_account_id).await {
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
                        connection_id,
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
                ).await.ok();

            total_accounts += 1;
            info!("Stored account: {}", snaptrade_account_id);

            // Get equity positions
            let equity_response = snaptrade_client
                .call_go_service(
                    "GET",
                    &format!("/api/v1/accounts/{}/holdings", snaptrade_account_id),
                    None::<&Value>,
                    user_id,
                    Some(&user_secret),
                )
                .await
                .ok();

            if let Some(equity) = equity_response
                && equity.status().is_success()
                && let Ok(equity_data) = equity.json::<Value>().await
                && let Some(positions) = equity_data.get("positions").and_then(|v| v.as_array())
            {
                info!(
                    "Got {} equity positions for account: {}",
                    positions.len(),
                    snaptrade_account_id
                );

                // Store equity positions
                for position in positions {
                    if let Some(pos_obj) = position.as_object() {
                        let symbol_obj = pos_obj.get("symbol");
                        let symbol = symbol_obj
                            .and_then(|s| s.get("symbol"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        if symbol.is_empty() {
                            continue; // Skip holdings without symbols
                        }
                        let quantity = pos_obj.get("units").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let average_cost = pos_obj
                            .get("average_purchase_price")
                            .and_then(|v| v.as_f64());
                        let current_price = pos_obj.get("price").and_then(|v| v.as_f64());
                        let market_value = pos_obj.get("value").and_then(|v| v.as_f64());
                        let currency = pos_obj
                            .get("currency")
                            .and_then(|c| c.get("code"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("USD");
                        let raw_data = serde_json::to_string(position).unwrap_or_default();

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

                        let holding_now = Utc::now().to_rfc3339();

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
                                        holding_now,
                                        raw_data
                                    ],
                                ).await.ok();

                        total_holdings += 1;
                    }
                }
            }

            // Get option positions
            let options_response = snaptrade_client
                .call_go_service(
                    "GET",
                    &format!("/api/v1/accounts/{}/holdings/options", snaptrade_account_id),
                    None::<&Value>,
                    user_id,
                    Some(&user_secret),
                )
                .await
                .ok();

            if let Some(options) = options_response
                && options.status().is_success()
                && let Ok(options_data) = options.json::<Value>().await
                && let Some(positions) = options_data.get("positions").and_then(|v| v.as_array())
            {
                info!(
                    "Got {} option positions for account: {}",
                    positions.len(),
                    snaptrade_account_id
                );

                // Store option positions
                for position in positions {
                    if let Some(pos_obj) = position.as_object() {
                        let symbol_obj = pos_obj.get("symbol");
                        let symbol = symbol_obj
                            .and_then(|s| s.get("symbol"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        if symbol.is_empty() {
                            continue; // Skip holdings without symbols
                        }
                        let quantity = pos_obj.get("units").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let average_cost = pos_obj
                            .get("average_purchase_price")
                            .and_then(|v| v.as_f64());
                        let current_price = pos_obj.get("price").and_then(|v| v.as_f64());
                        let market_value = pos_obj.get("value").and_then(|v| v.as_f64());
                        let currency = pos_obj
                            .get("currency")
                            .and_then(|c| c.get("code"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("USD");
                        let raw_data = serde_json::to_string(position).unwrap_or_default();

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

                        let holding_now = Utc::now().to_rfc3339();

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
                                        holding_now,
                                        raw_data
                                    ],
                                ).await.ok();

                        total_holdings += 1;
                    }
                }
            }

            // Get transactions (with pagination - fetch first page)
            let transactions_response = snaptrade_client
                .call_go_service(
                    "GET",
                    &format!(
                        "/api/v1/accounts/{}/transactions?limit=1000&offset=0",
                        snaptrade_account_id
                    ),
                    None::<&Value>,
                    user_id,
                    Some(&user_secret),
                )
                .await
                .ok();

            if let Some(transactions) = transactions_response
                && transactions.status().is_success()
                && let Ok(trans_data) = transactions.json::<Value>().await
                && let Some(data_array) = trans_data.get("data").and_then(|v| v.as_array())
            {
                info!(
                    "Got {} transactions for account: {}",
                    data_array.len(),
                    snaptrade_account_id
                );

                // Store transactions
                for transaction in data_array {
                    if let Some(trans_obj) = transaction.as_object() {
                        let snaptrade_transaction_id =
                            trans_obj.get("id").and_then(|v| v.as_str()).unwrap_or("");
                        if snaptrade_transaction_id.is_empty() {
                            continue; // Skip transactions without IDs
                        }
                        let symbol_obj = trans_obj.get("symbol");
                        let symbol = symbol_obj
                            .and_then(|s| s.get("symbol"))
                            .and_then(|v| v.as_str());
                        let transaction_type = trans_obj.get("type").and_then(|v| v.as_str());
                        let quantity = trans_obj.get("units").and_then(|v| v.as_f64());
                        let price = trans_obj.get("price").and_then(|v| v.as_f64());
                        let amount = trans_obj.get("amount").and_then(|v| v.as_f64());
                        let currency_obj = trans_obj.get("currency");
                        let currency = currency_obj
                            .and_then(|c| c.get("code"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("USD");
                        let default_trade_date = Utc::now().to_rfc3339();
                        let trade_date = trans_obj
                            .get("trade_date")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&default_trade_date);
                        let settlement_date =
                            trans_obj.get("settlement_date").and_then(|v| v.as_str());
                        let fees = trans_obj.get("fee").and_then(|v| v.as_f64());
                        let raw_data = serde_json::to_string(&transaction).unwrap_or_default();

                        // Check if transaction already exists to prevent duplicates
                        if transaction_exists(conn, &account_uuid, snaptrade_transaction_id).await {
                            info!(
                                "Transaction {} for account {} already exists, skipping",
                                snaptrade_transaction_id, account_uuid
                            );
                            continue;
                        }

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
    }

    // Update last_sync_at for the connection
    let sync_now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE brokerage_connections SET last_sync_at = ?, updated_at = ? WHERE id = ?",
        libsql::params![sync_now.clone(), sync_now.clone(), connection_id],
    )
    .await
    .ok();

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
        last_sync_at: sync_now,
    })
}
