use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::client_helper::get_yahoo_client;
use finance_query_core::YahooError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchItem {
    pub symbol: String,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub exchange: Option<String>,
}

pub async fn search(
    query: &str,
    hits: Option<u32>,
    _yahoo: Option<bool>,
) -> Result<Vec<SearchItem>, YahooError> {
    let client = get_yahoo_client().await?;

    // Default hits to 10 if not specified
    let hits_value = hits.unwrap_or(10) as usize;

    // Use finance-query-core search method
    let result = client.search(query, hits_value).await?;

    // Parse the result from finance-query-core format
    // The result is a Value with quotes array
    if let Some(quotes) = result.get("quotes").and_then(|q| q.as_array()) {
        let items: Vec<SearchItem> = quotes
            .iter()
            .filter_map(|quote| {
                Some(SearchItem {
                    symbol: quote.get("symbol")?.as_str()?.to_string(),
                    name: quote
                        .get("longname")
                        .or_else(|| quote.get("shortname"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    kind: quote
                        .get("quoteType")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    exchange: quote
                        .get("exchange")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                })
            })
            .collect();
        Ok(items)
    } else {
        Ok(vec![])
    }
}
