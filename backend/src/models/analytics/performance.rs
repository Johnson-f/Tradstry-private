use serde::{Deserialize, Serialize};

/// Advanced performance metrics for trading system analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    // Trade expectancy and edge
    pub trade_expectancy: f64,
    pub edge: f64,

    // Hold time analysis
    pub average_hold_time_days: f64,
    pub average_hold_time_winners_days: f64,
    pub average_hold_time_losers_days: f64,

    // Position sizing metrics
    pub average_position_size: f64,
    pub position_size_standard_deviation: f64,
    pub position_size_variability: f64,

    // Advanced trading metrics
    pub kelly_criterion: f64,
    pub system_quality_number: f64,
    pub payoff_ratio: f64,

    // R-Multiple analysis
    pub average_r_multiple: f64,
    pub r_multiple_standard_deviation: f64,
    pub positive_r_multiple_count: u32,
    pub negative_r_multiple_count: u32,

    // Consistency metrics
    pub consistency_ratio: f64,
    pub monthly_win_rate: f64,
    pub quarterly_win_rate: f64,

    // Execution metrics
    pub average_slippage: f64,
    pub commission_impact_percentage: f64,
}
