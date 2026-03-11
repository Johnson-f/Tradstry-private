use super::combine::combine_core_metrics;
use super::options::calculate_options_core_metrics;
use super::stocks::calculate_stocks_core_metrics;
use crate::models::analytics::CoreMetrics;
use crate::models::stock::stocks::TimeRange;
use anyhow::Result;
use libsql::Connection;

/// Calculate core trading metrics from stocks and options tables
pub async fn calculate_core_metrics(
    conn: &Connection,
    time_range: &TimeRange,
) -> Result<CoreMetrics> {
    let (time_condition, time_params) = time_range.to_sql_condition();

    // Calculate stocks metrics
    let stocks_metrics = calculate_stocks_core_metrics(conn, &time_condition, &time_params).await?;

    // Calculate options metrics
    let options_metrics =
        calculate_options_core_metrics(conn, &time_condition, &time_params).await?;

    // Combine metrics from both tables
    let combined_metrics = combine_core_metrics(stocks_metrics, options_metrics);

    Ok(combined_metrics)
}
