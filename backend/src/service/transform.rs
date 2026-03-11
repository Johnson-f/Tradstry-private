use anyhow::{Result, Context};
use libsql::Connection;
use log::{info, error};
use serde_json::Value;
use chrono::Utc;
use std::sync::Arc;
// VectorizationService removed - no longer using Upstash

/// Transaction data structure for matching
#[allow(dead_code)]
#[derive(Debug, Clone)]
struct TransactionData {
    id: String,
    snaptrade_transaction_id: String,
    account_id: String,
    symbol: String,
    trade_type: String, // "BUY" or "SELL"
    units: f64,
    price: f64,
    fee: f64,
    trade_date: String,
    brokerage_name: Option<String>,
    raw_data: Value,
    is_option: bool,
}

/// Transform brokerage transactions into stocks or options trades
/// NOTE: Automatic matching has been removed. Users should use the manual merge feature.
/// This function is kept for backward compatibility but does not perform automatic matching.
pub async fn transform_brokerage_transactions(
    _conn: &Connection,
    _user_id: &str,
    _vectorization_service: Option<()>, // Placeholder for backward compatibility
) -> Result<()> {
    info!("Automatic transformation is disabled. Please use the manual merge feature at /api/brokerage/transactions/merge");
    Ok(())
}

/// Check if a transaction has already been transformed
/// 
/// This prevents duplicate trades from being created when the sync runs multiple times.
/// We check by looking for trades that match the transaction's key characteristics:
/// - Same symbol
/// - Same trade date
/// - Same price
/// - Same quantity/shares
/// - Same brokerage name (if available)
#[allow(dead_code)]
async fn transaction_already_transformed(
    conn: &Connection,
    snaptrade_transaction_id: &str,
) -> Result<bool> {
    // First, get the transaction data to extract key fields for matching
    let stmt = conn
        .prepare(
            r#"
            SELECT raw_data
            FROM brokerage_transactions
            WHERE snaptrade_transaction_id = ?
            LIMIT 1
            "#
        )
        .await
        .context("Failed to prepare query for transaction check")?;

    let mut rows = stmt.query(libsql::params![snaptrade_transaction_id]).await
        .context("Failed to query transaction")?;

    if let Some(row) = rows.next().await? {
        let raw_data: Option<String> = row.get(0)?;

        // Parse raw_data to get transaction details
        if let Some(raw) = raw_data
            && let Ok(trans_data) = serde_json::from_str::<Value>(&raw) {
            let trans_symbol = trans_data
                .get("symbol")
                .and_then(|s| s.get("symbol"))
                .and_then(|v| v.as_str());
            let _trans_type = trans_data.get("type").and_then(|v| v.as_str());
            let trans_units = trans_data.get("units").and_then(|v| v.as_f64());
            let trans_price = trans_data.get("price").and_then(|v| v.as_f64());
            let trans_trade_date = trans_data.get("trade_date").and_then(|v| v.as_str());
            let trans_institution = trans_data.get("institution").and_then(|v| v.as_str());

            // Check if it's an option trade
            let is_option = trans_data
                .get("option_symbol")
                .and_then(|v| v.as_str())
                .map(|s| !s.is_empty())
                .unwrap_or(false)
                || trans_data
                    .get("option_type")
                    .and_then(|v| v.as_str())
                    .map(|s| !s.is_empty())
                    .unwrap_or(false);

            if let (Some(sym), Some(price), Some(units), Some(date)) = 
                (trans_symbol, trans_price, trans_units, trans_trade_date) {
                
                if is_option {
                    // Check options table for matching trade
                    let check_stmt = conn
                        .prepare(
                            r#"
                            SELECT COUNT(*) FROM options
                            WHERE symbol = ? 
                            AND entry_date = ?
                            AND entry_price = ?
                            AND quantity = ?
                            AND brokerage_name = COALESCE(?, brokerage_name)
                            AND is_deleted = 0
                            LIMIT 1
                            "#
                        )
                        .await?;

                    let mut check_rows = check_stmt.query(libsql::params![
                        sym, date, price, units as i32, trans_institution
                    ]).await?;

                    if let Some(check_row) = check_rows.next().await? {
                        let count: i64 = check_row.get(0)?;
                        if count > 0 {
                            return Ok(true);
                        }
                    }
                } else {
                    // Check stocks table for matching trade
                    let check_stmt = conn
                        .prepare(
                            r#"
                            SELECT COUNT(*) FROM stocks
                            WHERE symbol = ? 
                            AND entry_date = ?
                            AND entry_price = ?
                            AND number_shares = ?
                            AND brokerage_name = COALESCE(?, brokerage_name)
                            AND is_deleted = 0
                            LIMIT 1
                            "#
                        )
                        .await?;

                    let mut check_rows = check_stmt.query(libsql::params![
                        sym, date, price, units, trans_institution
                    ]).await?;

                    if let Some(check_row) = check_rows.next().await? {
                        let count: i64 = check_row.get(0)?;
                        if count > 0 {
                            return Ok(true);
                        }
                    }
                }
            }
        }
    }

    Ok(false) // Transaction not yet transformed
}

// Removed: calculate_match_confidence - automatic matching is no longer used

/// Helper function to parse trade date string to DateTime
#[allow(dead_code)]
fn parse_trade_date(date_str: &str) -> Result<chrono::DateTime<Utc>> {
    // Try RFC3339 format first
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(date_str) {
        return Ok(dt.with_timezone(&Utc));
    }
    // Try SQLite datetime format
    if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S") {
        return Ok(chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
    }
    // Try date-only format
    if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        let ndt = date.and_hms_opt(0, 0, 0)
            .ok_or_else(|| anyhow::anyhow!("invalid date"))?;
        return Ok(chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
    }
    Err(anyhow::anyhow!("Unsupported datetime format: {}", date_str))
}

// Removed: store_unmatched_transaction - automatic matching is no longer used

// Removed: match_and_transform_stocks - automatic matching is no longer used
// Users should use the manual merge feature instead

// Removed: create_merged_stock_trade - automatic matching is no longer used
// Use the manual merge endpoint instead
#[allow(clippy::too_many_arguments)]
pub async fn _create_merged_stock_trade_removed(
    conn: &Connection,
    symbol: &str,
    entry_price: f64,
    exit_price: f64,
    number_shares: f64,
    entry_date: String,
    exit_date: String,
    total_commissions: f64,
    brokerage_name: Option<String>,
    user_id: &str,
    _vectorization_service: Option<()>, // Placeholder for backward compatibility
) -> Result<i64> {
    let order_type = "MARKET";
    let stop_loss = entry_price * 0.95; // Default to 5% below entry price

    // Insert merged trade
    let insert_stmt = conn
        .prepare(
            r#"
            INSERT INTO stocks (
                symbol, trade_type, order_type, entry_price, exit_price, stop_loss, commissions,
                number_shares, entry_date, exit_date, brokerage_name, reviewed, is_deleted
            ) VALUES (?, 'BUY', ?, ?, ?, ?, ?, ?, ?, ?, ?, false, 0)
            RETURNING id
            "#,
        )
        .await
        .context("Failed to prepare merged stock insert statement")?;

    let mut rows = insert_stmt
        .query(libsql::params![
            symbol,
            order_type,
            entry_price,
            exit_price,
            stop_loss,
            total_commissions,
            number_shares,
            entry_date,
            exit_date,
            brokerage_name
        ])
        .await
        .context("Failed to insert merged stock trade")?;

    let stock_id: i64 = if let Some(row) = rows.next().await? {
        row.get(0)?
    } else {
        return Err(anyhow::anyhow!("Failed to get inserted stock ID"));
    };

    // Vectorization removed - trades are vectorized via TradeVectorService for mistakes/notes only

    Ok(stock_id)
}

/// Create an open stock position (unmatched BUY)
#[allow(clippy::too_many_arguments)]
pub async fn create_open_stock_trade(
    conn: &Connection,
    symbol: &str,
    entry_price: f64,
    number_shares: f64,
    entry_date: String,
    commissions: f64,
    brokerage_name: Option<String>,
    user_id: &str,
    _vectorization_service: Option<()>, // Placeholder for backward compatibility
) -> Result<i64> {
    let order_type = "MARKET";
    let stop_loss = entry_price * 0.95; // Default to 5% below entry price

    // Insert open position (no exit_price or exit_date)
    let insert_stmt = conn
        .prepare(
            r#"
            INSERT INTO stocks (
                symbol, trade_type, order_type, entry_price, stop_loss, commissions,
                number_shares, entry_date, brokerage_name, reviewed, is_deleted
            ) VALUES (?, 'BUY', ?, ?, ?, ?, ?, ?, ?, false, 0)
            RETURNING id
            "#,
        )
        .await
        .context("Failed to prepare open stock insert statement")?;

    let mut rows = insert_stmt
        .query(libsql::params![
            symbol,
            order_type,
            entry_price,
            stop_loss,
            commissions,
            number_shares,
            entry_date,
            brokerage_name
        ])
        .await
        .context("Failed to insert open stock trade")?;

    let stock_id: i64 = if let Some(row) = rows.next().await? {
        row.get(0)?
    } else {
        return Err(anyhow::anyhow!("Failed to get inserted stock ID"));
    };

    // Vectorization removed - trades are vectorized via TradeVectorService for mistakes/notes only

    Ok(stock_id)
}

/// Transform a brokerage transaction to a stock trade
/// This is kept for backward compatibility but is now primarily used for unmatched transactions
#[allow(dead_code)]
async fn transform_to_stock(
    conn: &Connection,
    transaction: &Value,
    _snaptrade_transaction_id: &str,
    user_id: &str,
    _vectorization_service: Option<()>, // Placeholder for backward compatibility
) -> Result<()> {
    // Extract data from transaction JSON
    let symbol = transaction
        .get("symbol")
        .and_then(|s| s.get("symbol"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing symbol in transaction"))?;

    let trade_type = transaction
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing type in transaction"))?;

    // Validate trade_type
    if trade_type != "BUY" && trade_type != "SELL" {
        return Err(anyhow::anyhow!("Invalid trade_type: {}", trade_type));
    }

    let units = transaction
        .get("units")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| anyhow::anyhow!("Missing units in transaction"))?;

    let price = transaction
        .get("price")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| anyhow::anyhow!("Missing price in transaction"))?;

    let fee = transaction
        .get("fee")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let trade_date = transaction
        .get("trade_date")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing trade_date in transaction"))?;

    let brokerage_name = transaction
        .get("institution")
        .and_then(|v| v.as_str());

    // Set default values for required fields that might be missing
    let order_type = "MARKET"; // Default to MARKET since we don't have this info
    let stop_loss = price * 0.95; // Default to 5% below entry price (user will review)

    // Insert into stocks table and get the inserted ID
    let insert_stmt = conn
        .prepare(
            r#"
            INSERT INTO stocks (
                symbol, trade_type, order_type, entry_price, stop_loss, commissions,
                number_shares, entry_date, brokerage_name, reviewed, is_deleted
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, false, 0)
            RETURNING id
            "#,
        )
        .await
        .context("Failed to prepare stock insert statement")?;

    let mut rows = insert_stmt
        .query(libsql::params![
            symbol,
            trade_type,
            order_type,
            price,
            stop_loss,
            fee,
            units,
            trade_date,
            brokerage_name
        ])
        .await
        .context("Failed to insert stock trade")?;

    let stock_id: i64 = if let Some(row) = rows.next().await? {
        row.get(0)?
    } else {
        return Err(anyhow::anyhow!("Failed to get inserted stock ID"));
    };

    // Vectorization removed - trades are vectorized via TradeVectorService for mistakes/notes only

    Ok(())
}

/// Transform a brokerage transaction to an option trade
#[allow(dead_code)]
async fn transform_to_option(
    conn: &Connection,
    transaction: &Value,
    _snaptrade_transaction_id: &str,
    user_id: &str,
    _vectorization_service: Option<()>, // Placeholder for backward compatibility
) -> Result<()> {
    // Extract data from transaction JSON
    let symbol = transaction
        .get("symbol")
        .and_then(|s| s.get("symbol"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing symbol in transaction"))?;

    let _trade_type = transaction
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing type in transaction"))?;

    let units = transaction
        .get("units")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0) as i32;

    let price = transaction
        .get("price")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| anyhow::anyhow!("Missing price in transaction"))?;

    let fee = transaction
        .get("fee")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let trade_date = transaction
        .get("trade_date")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing trade_date in transaction"))?;

    let brokerage_name = transaction
        .get("institution")
        .and_then(|v| v.as_str());

    // Extract option-specific data
    let option_type_str = transaction
        .get("option_type")
        .and_then(|v| v.as_str())
        .unwrap_or("Call");

    let option_type = match option_type_str.to_uppercase().as_str() {
        "PUT" | "PUTS" => "Put",
        _ => "Call",
    };

    // For options, we need to extract strike price and expiration from option_symbol
    // If option_symbol is not available, we'll use defaults that the user can update
    let _option_symbol = transaction
        .get("option_symbol")
        .and_then(|v| v.as_str());

    // Parse option symbol if available (format: SYMBOL YYMMDD C/P STRIKE)
    // For now, use defaults - user will need to review and update
    let strike_price = price; // Default to entry price, user will update
    let expiration_date = trade_date; // Default to trade date, user will update

    // Set defaults for required fields
    let premium = price * units as f64;
    let total_quantity = Some(units as f64);

    // Insert into options table and get the inserted ID
    let insert_stmt = conn
        .prepare(
            r#"
            INSERT INTO options (
                symbol, option_type, strike_price, expiration_date, entry_price,
                premium, commissions, entry_date, brokerage_name, reviewed, is_deleted, status, total_quantity
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, false, 0, 'open', ?)
            RETURNING id
            "#,
        )
        .await
        .context("Failed to prepare option insert statement")?;

    let mut rows = insert_stmt
        .query(libsql::params![
            symbol,
            option_type,
            strike_price,
            expiration_date,
            price,
            premium,
            fee,
            trade_date,
            brokerage_name,
            total_quantity
        ])
        .await
        .context("Failed to insert option trade")?;

    let option_id: i64 = if let Some(row) = rows.next().await? {
        row.get(0)?
    } else {
        return Err(anyhow::anyhow!("Failed to get inserted option ID"));
    };

    // Vectorization removed - trades are vectorized via TradeVectorService for mistakes/notes only

    Ok(())
}

/// Add brokerage_name column to existing stocks and options tables if it doesn't exist
pub async fn migrate_add_brokerage_name_column(conn: &Connection) -> Result<()> {
    info!("Migrating: Adding brokerage_name column to stocks and options tables");

    // Check and add to stocks table
    {
        let check_col = conn
            .prepare("SELECT COUNT(*) FROM pragma_table_info('stocks') WHERE name = 'brokerage_name'")
            .await?;
        let mut rows = check_col.query(libsql::params![]).await?;
        if let Some(row) = rows.next().await? {
            let count: i64 = row.get(0)?;
            if count == 0 {
                conn.execute("ALTER TABLE stocks ADD COLUMN brokerage_name TEXT", libsql::params![])
                    .await
                    .context("Failed to add brokerage_name to stocks table")?;
                info!("Added brokerage_name column to stocks table");
            }
        }
    }

    // Check and add to options table
    {
        let check_col = conn
            .prepare("SELECT COUNT(*) FROM pragma_table_info('options') WHERE name = 'brokerage_name'")
            .await?;
        let mut rows = check_col.query(libsql::params![]).await?;
        if let Some(row) = rows.next().await? {
            let count: i64 = row.get(0)?;
            if count == 0 {
                conn.execute("ALTER TABLE options ADD COLUMN brokerage_name TEXT", libsql::params![])
                    .await
                    .context("Failed to add brokerage_name to options table")?;
                info!("Added brokerage_name column to options table");
            }
        }
    }

    Ok(())
}