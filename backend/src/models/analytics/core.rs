use serde::{Deserialize, Serialize};

/// Core trading metrics - basic performance indicators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreMetrics {
    // Trade counts
    pub total_trades: u32,
    pub winning_trades: u32,
    pub losing_trades: u32,
    pub break_even_trades: u32,

    // Rates
    pub win_rate: f64,
    pub loss_rate: f64,

    // PnL metrics
    pub total_pnl: f64,
    pub net_profit_loss: f64,
    pub gross_profit: f64,
    pub gross_loss: f64,

    // Average amounts
    pub average_win: f64,
    pub average_loss: f64,
    pub average_position_size: f64,

    // Extremes
    pub biggest_winner: f64,
    pub biggest_loser: f64,

    // Ratios
    pub profit_factor: f64,
    pub win_loss_ratio: f64,

    // Consecutive streaks
    pub max_consecutive_wins: u32,
    pub max_consecutive_losses: u32,

    // Commission impact
    pub total_commissions: f64,
    pub average_commission_per_trade: f64,
}
