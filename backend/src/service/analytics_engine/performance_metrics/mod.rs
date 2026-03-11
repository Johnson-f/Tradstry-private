//! One of the most important files for the AI systems for spotting trading patterns

use crate::models::analytics::PerformanceMetrics;
use crate::models::stock::stocks::TimeRange;
use anyhow::Result;
use libsql::Connection;

pub mod behavioral;
pub mod duration;
mod options;
mod stocks;
mod utils;

// Re-export public API
// Commented out until used:
// pub use behavioral::{BehavioralPatterns, RiskBehaviorMetrics, TimingBehaviorMetrics, TradingFrequencyMetrics, ProfitabilityDistributionMetrics};
// pub use duration::DurationPerformanceMetrics;
pub use duration::{DurationPerformanceResponse, calculate_duration_performance_metrics};

/// Calculate performance metrics including hold times for winners and losers
pub async fn calculate_performance_metrics(
    conn: &Connection,
    time_range: &TimeRange,
) -> Result<PerformanceMetrics> {
    let (time_condition, time_params) = time_range.to_sql_condition();

    // Calculate stocks performance metrics
    let stocks_metrics =
        stocks::calculate_stocks_performance_metrics(conn, &time_condition, &time_params).await?;

    // Calculate options performance metrics
    let options_metrics =
        options::calculate_options_performance_metrics(conn, &time_condition, &time_params).await?;

    // Combine metrics from both tables
    let combined_metrics = combine_performance_metrics(stocks_metrics, options_metrics);

    Ok(combined_metrics)
}

/// Combine performance metrics from stocks and options tables
fn combine_performance_metrics(
    stocks: PerformanceMetrics,
    options: PerformanceMetrics,
) -> PerformanceMetrics {
    // Weighted averages based on position sizes
    let stocks_weight = if stocks.average_position_size > 0.0 && options.average_position_size > 0.0
    {
        stocks.average_position_size
            / (stocks.average_position_size + options.average_position_size)
    } else if stocks.average_position_size > 0.0 {
        1.0
    } else {
        0.0
    };

    let options_weight = 1.0 - stocks_weight;

    PerformanceMetrics {
        trade_expectancy: stocks.trade_expectancy * stocks_weight
            + options.trade_expectancy * options_weight,
        edge: stocks.edge * stocks_weight + options.edge * options_weight,
        average_hold_time_days: stocks.average_hold_time_days * stocks_weight
            + options.average_hold_time_days * options_weight,
        average_hold_time_winners_days: stocks.average_hold_time_winners_days * stocks_weight
            + options.average_hold_time_winners_days * options_weight,
        average_hold_time_losers_days: stocks.average_hold_time_losers_days * stocks_weight
            + options.average_hold_time_losers_days * options_weight,
        average_position_size: stocks.average_position_size * stocks_weight
            + options.average_position_size * options_weight,
        position_size_standard_deviation: stocks.position_size_standard_deviation * stocks_weight
            + options.position_size_standard_deviation * options_weight,
        position_size_variability: stocks.position_size_variability * stocks_weight
            + options.position_size_variability * options_weight,
        kelly_criterion: stocks.kelly_criterion * stocks_weight
            + options.kelly_criterion * options_weight,
        system_quality_number: stocks.system_quality_number * stocks_weight
            + options.system_quality_number * options_weight,
        payoff_ratio: stocks.payoff_ratio * stocks_weight + options.payoff_ratio * options_weight,
        average_r_multiple: stocks.average_r_multiple * stocks_weight
            + options.average_r_multiple * options_weight,
        r_multiple_standard_deviation: stocks.r_multiple_standard_deviation * stocks_weight
            + options.r_multiple_standard_deviation * options_weight,
        positive_r_multiple_count: stocks.positive_r_multiple_count
            + options.positive_r_multiple_count,
        negative_r_multiple_count: stocks.negative_r_multiple_count
            + options.negative_r_multiple_count,
        consistency_ratio: stocks.consistency_ratio * stocks_weight
            + options.consistency_ratio * options_weight,
        monthly_win_rate: stocks.monthly_win_rate * stocks_weight
            + options.monthly_win_rate * options_weight,
        quarterly_win_rate: stocks.quarterly_win_rate * stocks_weight
            + options.quarterly_win_rate * options_weight,
        average_slippage: 0.0, // Not available in current schema
        commission_impact_percentage: stocks.commission_impact_percentage * stocks_weight
            + options.commission_impact_percentage * options_weight,
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            trade_expectancy: 0.0,
            edge: 0.0,
            average_hold_time_days: 0.0,
            average_hold_time_winners_days: 0.0,
            average_hold_time_losers_days: 0.0,
            average_position_size: 0.0,
            position_size_standard_deviation: 0.0,
            position_size_variability: 0.0,
            kelly_criterion: 0.0,
            system_quality_number: 0.0,
            payoff_ratio: 0.0,
            average_r_multiple: 0.0,
            r_multiple_standard_deviation: 0.0,
            positive_r_multiple_count: 0,
            negative_r_multiple_count: 0,
            consistency_ratio: 0.0,
            monthly_win_rate: 0.0,
            quarterly_win_rate: 0.0,
            average_slippage: 0.0,
            commission_impact_percentage: 0.0,
        }
    }
}
