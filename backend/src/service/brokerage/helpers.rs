use anyhow::Result;
use chrono::Utc;
use libsql::Connection;
use log::{error, info, warn};
use uuid::Uuid;

use crate::service::brokerage::client::SnapTradeClient;

/// Helper function to get existing account ID by snaptrade_account_id
pub async fn get_existing_account_id(
    conn: &Connection,
    connection_id: &str,
    snaptrade_account_id: &str,
) -> Option<String> {
    let stmt = match conn
        .prepare("SELECT id FROM brokerage_accounts WHERE connection_id = ? AND snaptrade_account_id = ?")
        .await
    {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to prepare account query: {}", e);
            return None;
        }
    };

    let mut rows = match stmt
        .query(libsql::params![connection_id, snaptrade_account_id])
        .await
    {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to query account: {}", e);
            return None;
        }
    };

    if let Ok(Some(row)) = rows.next().await
        && let Ok(id) = row.get::<String>(0)
    {
        return Some(id);
    }
    None
}

/// Helper function to get existing holding ID by account_id and symbol
pub async fn get_existing_holding_id(
    conn: &Connection,
    account_id: &str,
    symbol: &str,
) -> Option<String> {
    let stmt = match conn
        .prepare("SELECT id FROM brokerage_holdings WHERE account_id = ? AND symbol = ?")
        .await
    {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to prepare holding query: {}", e);
            return None;
        }
    };

    let mut rows = match stmt.query(libsql::params![account_id, symbol]).await {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to query holding: {}", e);
            return None;
        }
    };

    if let Ok(Some(row)) = rows.next().await
        && let Ok(id) = row.get::<String>(0)
    {
        return Some(id);
    }
    None
}

/// Helper function to check if transaction already exists
pub async fn transaction_exists(
    conn: &Connection,
    account_id: &str,
    snaptrade_transaction_id: &str,
) -> bool {
    let stmt = match conn
        .prepare("SELECT 1 FROM brokerage_transactions WHERE account_id = ? AND snaptrade_transaction_id = ? LIMIT 1")
        .await
    {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to prepare transaction check query: {}", e);
            return false;
        }
    };

    let mut rows = match stmt
        .query(libsql::params![account_id, snaptrade_transaction_id])
        .await
    {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to query transaction: {}", e);
            return false;
        }
    };

    matches!(rows.next().await, Ok(Some(_)))
}

/// Get or create SnapTrade user credentials
pub async fn get_or_create_snaptrade_user(
    conn: &Connection,
    user_id: &str,
    snaptrade_client: &SnapTradeClient,
) -> Result<(String, String)> {
    use log::{info, warn};

    // Check if user already has a SnapTrade user
    let stmt = conn
        .prepare("SELECT snaptrade_user_id, snaptrade_user_secret FROM brokerage_connections WHERE user_id = ? LIMIT 1")
        .await?;

    let mut result = stmt.query(libsql::params![user_id]).await?;

    if let Some(row) = result.next().await? {
        let snaptrade_user_id: String = row.get(0)?;
        let snaptrade_user_secret: String = row.get(1)?;
        return Ok((snaptrade_user_id, snaptrade_user_secret));
    }

    // Create new SnapTrade user
    let create_user_req = serde_json::json!({
        "user_id": user_id
    });

    let response = snaptrade_client
        .call_go_service(
            "POST",
            "/api/v1/users",
            Some(&create_user_req),
            user_id,
            None,
        )
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());

        // If user already exists (400), check if we have credentials stored
        if status.as_u16() == 400 {
            // Check again if credentials were stored (race condition check)
            let stmt = conn
                .prepare("SELECT snaptrade_user_id, snaptrade_user_secret FROM brokerage_connections WHERE user_id = ? LIMIT 1")
                .await?;

            let mut result = stmt.query(libsql::params![user_id]).await?;

            if let Some(row) = result.next().await? {
                let snaptrade_user_id: String = row.get(0)?;
                let snaptrade_user_secret: String = row.get(1)?;
                return Ok((snaptrade_user_id, snaptrade_user_secret));
            }

            // User exists in SnapTrade but we don't have credentials stored
            info!(
                "User exists in SnapTrade but credentials not stored. Attempting to delete and recreate user: {}",
                user_id
            );

            // Try to delete the user
            let delete_req = serde_json::json!({
                "user_secret": user_id
            });

            let delete_response = snaptrade_client
                .call_go_service(
                    "DELETE",
                    &format!("/api/v1/users/{}", user_id),
                    Some(&delete_req),
                    user_id,
                    None,
                )
                .await;

            let deletion_queued = match delete_response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let delete_result: serde_json::Value =
                            resp.json().await.unwrap_or_default();
                        if let Some(status) = delete_result.get("status").and_then(|s| s.as_str()) {
                            if status == "deleted" {
                                info!(
                                    "SnapTrade user queued for deletion: {}. Detail: {}",
                                    user_id,
                                    delete_result
                                        .get("detail")
                                        .and_then(|d| d.as_str())
                                        .unwrap_or("")
                                );
                                true
                            } else {
                                warn!("Unexpected deletion status: {}", status);
                                false
                            }
                        } else {
                            info!("SnapTrade user deletion request sent: {}", user_id);
                            true
                        }
                    } else {
                        let delete_error = resp
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unknown error".to_string());
                        warn!(
                            "Failed to delete SnapTrade user (may require user_secret): {}. Will attempt to recreate anyway.",
                            delete_error
                        );
                        false
                    }
                }
                Err(e) => {
                    warn!(
                        "Error attempting to delete SnapTrade user: {}. Will attempt to recreate anyway.",
                        e
                    );
                    false
                }
            };

            // Wait for async deletion to process (if it was queued)
            if deletion_queued {
                info!("Waiting for SnapTrade user deletion to complete...");
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            }

            // Now try to create the user again
            let create_user_req_retry = serde_json::json!({
                "user_id": user_id
            });

            let retry_response = snaptrade_client
                .call_go_service(
                    "POST",
                    "/api/v1/users",
                    Some(&create_user_req_retry),
                    user_id,
                    None,
                )
                .await?;

            if retry_response.status().is_success() {
                let user_data: serde_json::Value = retry_response.json().await?;

                let snaptrade_user_id = user_data["user_id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Invalid response format"))?
                    .to_string();
                let snaptrade_user_secret = user_data["user_secret"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Invalid response format"))?
                    .to_string();

                // Store the new credentials immediately
                let now = Utc::now().to_rfc3339();
                conn.execute(
                    "INSERT OR REPLACE INTO brokerage_connections (id, user_id, snaptrade_user_id, snaptrade_user_secret, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
                    libsql::params![
                        Uuid::new_v4().to_string(),
                        user_id,
                        snaptrade_user_id.clone(),
                        snaptrade_user_secret.clone(),
                        "pending",
                        now.clone(),
                        now
                    ],
                ).await.ok();

                info!(
                    "Successfully recreated SnapTrade user with new credentials: {}",
                    user_id
                );
                return Ok((snaptrade_user_id, snaptrade_user_secret));
            } else {
                let retry_status_code = retry_response.status().as_u16();
                let retry_error_text = retry_response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());

                // If user still exists, deletion didn't work
                if retry_error_text.contains("already exist") || retry_status_code == 400 {
                    return Err(anyhow::anyhow!(
                        "Cannot delete existing SnapTrade user without credentials. Please contact support to reset your SnapTrade account, or wait a few minutes and try again. Error: {}",
                        retry_error_text
                    ));
                }

                return Err(anyhow::anyhow!(
                    "Failed to recreate SnapTrade user after deletion. Please try again in a few moments. Error: {}",
                    retry_error_text
                ));
            }
        }

        return Err(anyhow::anyhow!(
            "Failed to create SnapTrade user: {}",
            error_text
        ));
    }

    let user_data: serde_json::Value = response.json().await?;

    let snaptrade_user_id = user_data["user_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid response format"))?
        .to_string();
    let snaptrade_user_secret = user_data["user_secret"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid response format"))?
        .to_string();

    // Store user_id and user_secret immediately after successful registration
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT OR IGNORE INTO brokerage_connections (id, user_id, snaptrade_user_id, snaptrade_user_secret, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
        libsql::params![
            Uuid::new_v4().to_string(),
            user_id,
            snaptrade_user_id.clone(),
            snaptrade_user_secret.clone(),
            "pending",
            now.clone(),
            now
        ],
    ).await.ok();

    Ok((snaptrade_user_id, snaptrade_user_secret))
}
