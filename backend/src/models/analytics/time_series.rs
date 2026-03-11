use crate::models::analytics::TimeSeriesPoint;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Time series data for equity curves and rolling metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesData {
    // Equity curve data
    pub daily_pnl: Vec<TimeSeriesPoint>,
    pub weekly_pnl: Vec<TimeSeriesPoint>,
    pub monthly_pnl: Vec<TimeSeriesPoint>,

    // Rolling metrics
    pub rolling_win_rate_20: Vec<TimeSeriesPoint>,
    pub rolling_win_rate_50: Vec<TimeSeriesPoint>,
    pub rolling_win_rate_100: Vec<TimeSeriesPoint>,

    pub rolling_sharpe_ratio_20: Vec<TimeSeriesPoint>,
    pub rolling_sharpe_ratio_50: Vec<TimeSeriesPoint>,

    // Calendar-based analytics
    pub profit_by_day_of_week: HashMap<String, f64>,
    pub profit_by_month: HashMap<String, f64>,

    // Drawdown curve
    pub drawdown_curve: Vec<TimeSeriesPoint>,

    // Cumulative metrics
    pub cumulative_return: f64,
    pub annualized_return: f64,
    pub total_return_percentage: f64,
    pub total_trades: u32,
}
