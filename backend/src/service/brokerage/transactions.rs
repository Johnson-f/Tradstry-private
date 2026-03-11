use crate::service::brokerage::transform;
use anyhow::{Context, Result};
use chrono::Utc;
use libsql::Connection;
use log::info;
use serde_json::Value;

/// Get transactions for a user
pub async fn get_transactions(
    conn: &Connection,
    user_id: &str,
    account_id: Option<&str>,
    exclude_transformed: bool,
) -> Result<Vec<Value>> {
    let mut sql = "SELECT id, account_id, snaptrade_transaction_id, symbol, transaction_type, quantity, price, amount, currency, trade_date, settlement_date, fees, created_at, is_transformed, raw_data FROM brokerage_transactions WHERE account_id IN (SELECT id FROM brokerage_accounts WHERE connection_id IN (SELECT id FROM brokerage_connections WHERE user_id = ?))".to_string();

    if account_id.is_some() {
        sql.push_str(" AND account_id = ?");
    }

    if exclude_transformed {
        sql.push_str(" AND (is_transformed IS NULL OR is_transformed = 0)");
    }

    sql.push_str(" ORDER BY trade_date DESC LIMIT 100");

    let stmt = conn.prepare(&sql).await.context("Database error")?;

    let mut result = if let Some(acc_id) = account_id {
        stmt.query(libsql::params![user_id, acc_id]).await
    } else {
        stmt.query(libsql::params![user_id]).await
    }
    .context("Database query error")?;

    let mut transactions = Vec::new();
    while let Some(row) = result.next().await? {
        let id: String = row.get(0)?;
        let account_id: String = row.get(1)?;
        let snaptrade_transaction_id: String = row.get(2)?;
        let symbol: Option<String> = row.get(3)?;
        let transaction_type: Option<String> = row.get(4)?;
        let quantity: Option<f64> = row.get(5)?;
        let price: Option<f64> = row.get(6)?;
        let amount: Option<f64> = row.get(7)?;
        let currency: Option<String> = row.get(8)?;
        let trade_date: String = row.get(9)?;
        let settlement_date: Option<String> = row.get(10)?;
        let fees: Option<f64> = row.get(11)?;
        let created_at: String = row.get(12)?;
        let is_transformed: Option<i64> = row.get(13)?;
        let raw_data: Option<String> = row.get(14)?;

        transactions.push(serde_json::json!({
            "id": id,
            "account_id": account_id,
            "snaptrade_transaction_id": snaptrade_transaction_id,
            "symbol": symbol,
            "transaction_type": transaction_type,
            "quantity": quantity,
            "price": price,
            "amount": amount,
            "currency": currency,
            "trade_date": trade_date,
            "settlement_date": settlement_date,
            "fees": fees,
            "created_at": created_at,
            "is_transformed": is_transformed.map(|v| v != 0),
            "raw_data": raw_data
        }));
    }

    Ok(transactions)
}

/// Get transactions for a specific account
#[allow(dead_code)]
pub async fn get_account_transactions(
    conn: &Connection,
    account_id: &str,
    user_id: &str,
) -> Result<Vec<Value>> {
    let stmt = conn
        .prepare("SELECT id, account_id, snaptrade_transaction_id, symbol, transaction_type, quantity, price, amount, currency, trade_date, settlement_date, fees, created_at, is_transformed, raw_data FROM brokerage_transactions WHERE account_id = ? AND account_id IN (SELECT id FROM brokerage_accounts WHERE connection_id IN (SELECT id FROM brokerage_connections WHERE user_id = ?)) ORDER BY trade_date DESC LIMIT 1000")
        .await
        .context("Database error")?;

    let mut result = stmt
        .query(libsql::params![account_id, user_id])
        .await
        .context("Database query error")?;

    let mut transactions = Vec::new();
    while let Some(row) = result.next().await? {
        let id: String = row.get(0)?;
        let account_id: String = row.get(1)?;
        let snaptrade_transaction_id: String = row.get(2)?;
        let symbol: Option<String> = row.get(3)?;
        let transaction_type: Option<String> = row.get(4)?;
        let quantity: Option<f64> = row.get(5)?;
        let price: Option<f64> = row.get(6)?;
        let amount: Option<f64> = row.get(7)?;
        let currency: Option<String> = row.get(8)?;
        let trade_date: String = row.get(9)?;
        let settlement_date: Option<String> = row.get(10)?;
        let fees: Option<f64> = row.get(11)?;
        let created_at: String = row.get(12)?;
        let is_transformed: Option<i64> = row.get(13)?;
        let raw_data: Option<String> = row.get(14)?;

        transactions.push(serde_json::json!({
            "id": id,
            "account_id": account_id,
            "snaptrade_transaction_id": snaptrade_transaction_id,
            "symbol": symbol,
            "transaction_type": transaction_type,
            "quantity": quantity,
            "price": price,
            "amount": amount,
            "currency": currency,
            "trade_date": trade_date,
            "settlement_date": settlement_date,
            "fees": fees,
            "created_at": created_at,
            "is_transformed": is_transformed.map(|v| v != 0),
            "raw_data": raw_data
        }));
    }

    Ok(transactions)
}

/// Get unmatched transactions
pub async fn get_unmatched_transactions(conn: &Connection, user_id: &str) -> Result<Vec<Value>> {
    let stmt = conn
        .prepare(
            r#"
            SELECT id, user_id, transaction_id, snaptrade_transaction_id, symbol,
                   trade_type, units, price, fee, trade_date, brokerage_name,
                   is_option, difficulty_reason, confidence_score, suggested_matches,
                   status, resolved_trade_id, resolved_at, created_at, updated_at
            FROM unmatched_transactions
            WHERE user_id = ? AND status = 'pending'
            ORDER BY trade_date DESC, created_at DESC
            "#,
        )
        .await
        .context("Failed to prepare query")?;

    let mut rows = stmt
        .query(libsql::params![user_id])
        .await
        .context("Failed to query unmatched transactions")?;

    let mut transactions = Vec::new();
    while let Some(row) = rows.next().await? {
        transactions.push(serde_json::json!({
            "id": row.get::<String>(0)?,
            "user_id": row.get::<String>(1)?,
            "transaction_id": row.get::<String>(2)?,
            "snaptrade_transaction_id": row.get::<String>(3)?,
            "symbol": row.get::<Option<String>>(4)?,
            "trade_type": row.get::<String>(5)?,
            "units": row.get::<f64>(6)?,
            "price": row.get::<f64>(7)?,
            "fee": row.get::<f64>(8)?,
            "trade_date": row.get::<String>(9)?,
            "brokerage_name": row.get::<Option<String>>(10)?,
            "is_option": row.get::<bool>(11)?,
            "difficulty_reason": row.get::<Option<String>>(12)?,
            "confidence_score": row.get::<Option<f64>>(13)?,
            "suggested_matches": row.get::<Option<String>>(14)?,
            "status": row.get::<String>(15)?,
            "resolved_trade_id": row.get::<Option<i64>>(16)?,
            "resolved_at": row.get::<Option<String>>(17)?,
            "created_at": row.get::<String>(18)?,
            "updated_at": row.get::<String>(19)?,
        }));
    }

    Ok(transactions)
}

/// Resolve unmatched transaction
///
/// Actions:
/// - "merge": Merging should be done via the merge_transactions endpoint
/// - "create_open": Creates an open stock position from a BUY transaction
pub async fn resolve_unmatched_transaction(
    conn: &Connection,
    unmatched_id: &str,
    user_id: &str,
    action: &str,
    matched_transaction_id: Option<&str>,
    entry_price: Option<f64>,
    entry_date: Option<String>,
) -> Result<i64> {
    // Get the unmatched transaction
    let stmt = conn
        .prepare(
            r#"
            SELECT id, symbol, trade_type, units, price, fee, trade_date,
                   brokerage_name, raw_data, is_option
            FROM unmatched_transactions
            WHERE id = ? AND user_id = ? AND status = 'pending'
            "#,
        )
        .await
        .context("Failed to prepare query for unmatched transaction")?;

    let mut rows = stmt
        .query(libsql::params![unmatched_id, user_id])
        .await
        .context("Failed to query unmatched transaction")?;

    let row = rows
        .next()
        .await?
        .ok_or_else(|| anyhow::anyhow!("Unmatched transaction not found or already resolved"))?;

    let symbol: String = row
        .get(1)
        .context("Failed to get symbol from unmatched transaction")?;
    let trade_type: String = row
        .get(2)
        .context("Failed to get trade_type from unmatched transaction")?;
    let units: f64 = row
        .get(3)
        .context("Failed to get units from unmatched transaction")?;
    let _price: f64 = row
        .get(4)
        .context("Failed to get price from unmatched transaction")?;
    let fee: f64 = row
        .get(5)
        .context("Failed to get fee from unmatched transaction")?;
    let _trade_date: String = row
        .get(6)
        .context("Failed to get trade_date from unmatched transaction")?;
    let brokerage_name: Option<String> = row
        .get(7)
        .context("Failed to get brokerage_name from unmatched transaction")?;
    let _raw_data: String = row
        .get(8)
        .context("Failed to get raw_data from unmatched transaction")?;
    let is_option: bool = row
        .get::<Option<i64>>(9)
        .context("Failed to get is_option from unmatched transaction")?
        .map(|v| v != 0)
        .unwrap_or(false);

    let trade_id = match action {
        "merge" => {
            // Merge action should redirect to merge_transactions endpoint
            if matched_transaction_id.is_none() {
                return Err(anyhow::anyhow!(
                    "matched_transaction_id required for merge action"
                ));
            }

            // Note: Manual merge feature is now handled by the merge_transactions endpoint
            // This code path is for resolving unmatched transactions, which should use Stock::create directly
            return Err(anyhow::anyhow!(
                "Please use the merge_transactions endpoint for merging trades"
            ));
        }
        "create_open" => {
            // Create an open position
            let entry_price = entry_price
                .ok_or_else(|| anyhow::anyhow!("entry_price required for create_open action"))?;
            let entry_date = entry_date
                .ok_or_else(|| anyhow::anyhow!("entry_date required for create_open action"))?;

            // Validate trade type
            if trade_type != "BUY" {
                return Err(anyhow::anyhow!(
                    "Only BUY transactions can be created as open positions"
                ));
            }

            // Validate not an option (for now)
            if is_option {
                return Err(anyhow::anyhow!("Option open positions not yet implemented"));
            }

            info!(
                "Creating open stock position for unmatched transaction {}: {} shares of {} at ${}",
                unmatched_id, units, symbol, entry_price
            );

            // Create the open stock trade
            let trade_id = transform::create_open_stock_trade(
                conn,
                &symbol,
                entry_price,
                units,
                entry_date,
                fee,
                brokerage_name,
                user_id,
                None, // vectorization_service_opt
            )
            .await
            .context("Failed to create open stock trade")?;

            // Mark the unmatched transaction as resolved
            let now = Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE unmatched_transactions SET status = 'resolved', resolved_trade_id = ?, resolved_at = ?, updated_at = ? WHERE id = ? AND user_id = ?",
                libsql::params![trade_id, now.clone(), now.clone(), unmatched_id, user_id],
            ).await
            .context("Failed to update unmatched transaction status")?;

            info!(
                "Successfully resolved unmatched transaction {} to trade ID {}",
                unmatched_id, trade_id
            );
            trade_id
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Invalid action. Must be 'merge' or 'create_open'"
            ));
        }
    };

    Ok(trade_id)
}

/// Ignore unmatched transaction
pub async fn ignore_unmatched_transaction(
    conn: &Connection,
    unmatched_id: &str,
    user_id: &str,
) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE unmatched_transactions SET status = 'ignored', updated_at = ? WHERE id = ? AND user_id = ?",
        libsql::params!["ignored", now, unmatched_id, user_id],
    ).await.context("Failed to ignore transaction")?;

    Ok(())
}

/// Get suggestions for unmatched transaction
pub async fn get_unmatched_suggestions(
    conn: &Connection,
    unmatched_id: &str,
    user_id: &str,
) -> Result<Vec<Value>> {
    use anyhow::Context;

    // Get the unmatched transaction
    let stmt = conn
        .prepare(
            r#"
            SELECT symbol, trade_type
            FROM unmatched_transactions
            WHERE id = ? AND user_id = ?
            "#,
        )
        .await
        .context("Failed to prepare query")?;

    let mut rows = stmt
        .query(libsql::params![unmatched_id, user_id])
        .await
        .context("Failed to query unmatched transaction")?;

    let row = rows
        .next()
        .await?
        .ok_or_else(|| anyhow::anyhow!("Unmatched transaction not found"))?;

    let symbol: String = row.get(0)?;
    let trade_type: String = row.get(1)?;

    // Find potential matches (opposite trade type, same symbol)
    let opposite_type = if trade_type == "BUY" { "SELL" } else { "BUY" };

    let match_stmt = conn
        .prepare(
            r#"
            SELECT id, symbol, trade_type, units, price, trade_date, brokerage_name,
                   confidence_score, difficulty_reason
            FROM unmatched_transactions
            WHERE user_id = ? AND symbol = ? AND trade_type = ? AND status = 'pending' AND id != ?
            ORDER BY trade_date ASC
            LIMIT 10
            "#,
        )
        .await
        .context("Failed to prepare match query")?;

    let mut match_rows = match_stmt
        .query(libsql::params![
            user_id,
            symbol,
            opposite_type,
            unmatched_id
        ])
        .await
        .context("Failed to query matches")?;

    let mut suggestions = Vec::new();
    while let Some(match_row) = match_rows.next().await? {
        suggestions.push(serde_json::json!({
            "id": match_row.get::<String>(0)?,
            "symbol": match_row.get::<String>(1)?,
            "trade_type": match_row.get::<String>(2)?,
            "units": match_row.get::<f64>(3)?,
            "price": match_row.get::<f64>(4)?,
            "trade_date": match_row.get::<String>(5)?,
            "brokerage_name": match_row.get::<Option<String>>(6)?,
            "confidence_score": match_row.get::<Option<f64>>(7)?,
            "difficulty_reason": match_row.get::<Option<String>>(8)?,
        }));
    }

    Ok(suggestions)
}
