//! Market indices data fetching functions using finance-query-core

use super::client_helper::get_yahoo_client;
use finance_query_core::YahooError;
use serde_json::Value;

/// Get market indices (S&P 500, Dow, Nasdaq, etc.)
pub async fn get_indices() -> Result<Value, YahooError> {
    let client = get_yahoo_client().await?;

    // Get indices symbols
    let indices_symbols = ["^GSPC".to_string(), "^DJI".to_string(), "^IXIC".to_string()];

    // Get quotes for indices
    let symbol_refs: Vec<&str> = indices_symbols.iter().map(|s| s.as_str()).collect();
    let quotes = client.get_simple_quotes(&symbol_refs).await?;

    Ok(quotes)
}
