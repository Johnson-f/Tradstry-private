use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::client_helper::get_yahoo_client;
use finance_query_core::YahooError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketHours {
    pub status: String,
    pub reason: Option<String>,
    pub timestamp: String,
}

pub async fn get_hours() -> Result<MarketHours, YahooError> {
    let client = get_yahoo_client().await?;

    // Get market status for US market
    let market_status = client.get_market_status("us_market").await?;

    // Convert to MarketHours format
    let reason = if market_status.is_open() {
        None
    } else if market_status.is_pre_market() {
        Some("Pre-market hours".to_string())
    } else if market_status.is_after_hours() {
        Some("After-hours".to_string())
    } else {
        Some("Market is closed".to_string())
    };

    Ok(MarketHours {
        status: market_status.status.clone(),
        reason,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}
