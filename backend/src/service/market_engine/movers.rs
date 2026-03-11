//! Market movers data fetching functions using finance-query-core

use super::client_helper::get_yahoo_client;
use finance_query_core::{MoverCount, YahooError};
use serde_json::Value;

/// Get market movers (gainers, losers, actives)
pub async fn get_movers() -> Result<Value, YahooError> {
    let client = get_yahoo_client().await?;

    // Get movers using the client
    let (actives, gainers, losers) = client.get_movers(MoverCount::TwentyFive).await?;

    // Convert to JSON structure
    Ok(serde_json::json!({
        "actives": actives,
        "gainers": gainers,
        "losers": losers
    }))
}

/// Get top gainers
pub async fn get_gainers(count: Option<u32>) -> Result<Value, YahooError> {
    let client = get_yahoo_client().await?;

    let mover_count = match count {
        Some(25) => MoverCount::TwentyFive,
        Some(50) => MoverCount::Fifty,
        Some(100) => MoverCount::Hundred,
        _ => MoverCount::TwentyFive,
    };

    let (_actives, gainers, _losers) = client.get_movers(mover_count).await?;

    // Convert gainers to JSON
    serde_json::to_value(gainers).map_err(|e| YahooError::ParseError(e.to_string()))
}

/// Get top losers
pub async fn get_losers(count: Option<u32>) -> Result<Value, YahooError> {
    let client = get_yahoo_client().await?;

    let mover_count = match count {
        Some(25) => MoverCount::TwentyFive,
        Some(50) => MoverCount::Fifty,
        Some(100) => MoverCount::Hundred,
        _ => MoverCount::TwentyFive,
    };

    let (_actives, _gainers, losers) = client.get_movers(mover_count).await?;

    // Convert losers to JSON
    serde_json::to_value(losers).map_err(|e| YahooError::ParseError(e.to_string()))
}

/// Get most active stocks
pub async fn get_most_active(count: Option<u32>) -> Result<Value, YahooError> {
    let client = get_yahoo_client().await?;

    let mover_count = match count {
        Some(25) => MoverCount::TwentyFive,
        Some(50) => MoverCount::Fifty,
        Some(100) => MoverCount::Hundred,
        _ => MoverCount::TwentyFive,
    };

    let (actives, _gainers, _losers) = client.get_movers(mover_count).await?;

    // Convert actives to JSON
    serde_json::to_value(actives).map_err(|e| YahooError::ParseError(e.to_string()))
}
