use serde::{Deserialize, Serialize};

/// Risk-adjusted metrics for portfolio analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskMetrics {
    // Risk-adjusted returns
    pub sharpe_ratio: f64,
    pub sortino_ratio: f64,
    pub calmar_ratio: f64,

    // Drawdown metrics
    pub maximum_drawdown: f64,
    pub maximum_drawdown_percentage: f64,
    pub maximum_drawdown_duration_days: u32,
    pub current_drawdown: f64,

    // Value at Risk
    pub var_95: f64,
    pub var_99: f64,
    pub expected_shortfall_95: f64,
    pub expected_shortfall_99: f64,

    // Risk per trade
    pub average_risk_per_trade: f64,
    pub risk_reward_ratio: f64,

    // Volatility metrics
    pub volatility: f64,
    pub downside_deviation: f64,

    // Advanced risk metrics
    pub ulcer_index: f64,
    pub recovery_factor: f64,
    pub sterling_ratio: f64,
}
