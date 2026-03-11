use anyhow::{Context, Result};
use libsql::Connection;
use serde_json::Value;

/// Get holdings for a user
pub async fn get_holdings(
    conn: &Connection,
    user_id: &str,
    account_id: Option<&str>,
) -> Result<Vec<Value>> {
    let mut sql = "SELECT id, account_id, symbol, quantity, average_cost, current_price, market_value, currency, last_updated FROM brokerage_holdings WHERE account_id IN (SELECT id FROM brokerage_accounts WHERE connection_id IN (SELECT id FROM brokerage_connections WHERE user_id = ?))".to_string();

    if account_id.is_some() {
        sql.push_str(" AND account_id = ?");
    }

    sql.push_str(" ORDER BY symbol");

    let stmt = conn.prepare(&sql).await.context("Database error")?;

    let mut result = if let Some(acc_id) = account_id {
        stmt.query(libsql::params![user_id, acc_id]).await
    } else {
        stmt.query(libsql::params![user_id]).await
    }
    .context("Database query error")?;

    let mut holdings = Vec::new();
    while let Some(row) = result.next().await? {
        let id: String = row.get(0)?;
        let account_id: String = row.get(1)?;
        let symbol: String = row.get(2)?;
        let quantity: f64 = row.get(3)?;
        let average_cost: Option<f64> = row.get(4)?;
        let current_price: Option<f64> = row.get(5)?;
        let market_value: Option<f64> = row.get(6)?;
        let currency: Option<String> = row.get(7)?;
        let last_updated: Option<String> = row.get(8)?;

        holdings.push(serde_json::json!({
            "id": id,
            "account_id": account_id,
            "symbol": symbol,
            "quantity": quantity,
            "average_cost": average_cost,
            "current_price": current_price,
            "market_value": market_value,
            "currency": currency,
            "last_updated": last_updated
        }));
    }

    Ok(holdings)
}
