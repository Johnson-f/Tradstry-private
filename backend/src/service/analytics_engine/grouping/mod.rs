// Module declarations
mod direction;
mod period;
mod strategy;
mod symbol;
mod utils;

// Re-exports (commented out until used)
// pub use symbol::calculate_symbol_grouped_analytics;
// pub use strategy::calculate_strategy_grouped_analytics;
// pub use direction::calculate_direction_grouped_analytics;
// pub use period::calculate_period_grouped_analytics;

use crate::models::analytics::{AnalyticsOptions, GroupedMetrics};
use crate::models::stock::stocks::TimeRange;
use anyhow::Result;
use libsql::Connection;
use std::collections::HashMap;

/// Calculate grouped analytics by symbol, strategy, or other criteria
pub async fn calculate_grouped_analytics(
    conn: &Connection,
    time_range: &TimeRange,
    options: &AnalyticsOptions,
) -> Result<HashMap<String, GroupedMetrics>> {
    let mut grouped_analytics = HashMap::new();

    for grouping_type in &options.grouping_types {
        match grouping_type {
            crate::models::analytics::options::GroupingType::Symbol => {
                let symbol_analytics =
                    symbol::calculate_symbol_grouped_analytics(conn, time_range).await?;
                grouped_analytics.extend(symbol_analytics);
            }
            crate::models::analytics::options::GroupingType::Strategy => {
                let strategy_analytics =
                    strategy::calculate_strategy_grouped_analytics(conn, time_range).await?;
                grouped_analytics.extend(strategy_analytics);
            }
            crate::models::analytics::options::GroupingType::TradeDirection => {
                let direction_analytics =
                    direction::calculate_direction_grouped_analytics(conn, time_range).await?;
                grouped_analytics.extend(direction_analytics);
            }
            crate::models::analytics::options::GroupingType::TimePeriod => {
                let period_analytics =
                    period::calculate_period_grouped_analytics(conn, time_range).await?;
                grouped_analytics.extend(period_analytics);
            }
        }
    }

    Ok(grouped_analytics)
}
