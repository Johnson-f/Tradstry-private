//! Core metrics calculation module for trading analytics.
//! This module calculates statistics for both options and stocks tables combined.

mod combine;
mod helpers;
mod individual;
mod main;
mod options;
mod stocks;
mod streaks;

// Re-export public API
#[allow(unused_imports)] // These are re-exported for external use
pub use individual::{
    IndividualTradeAnalytics, SymbolAnalytics, calculate_individual_option_trade_analytics,
    calculate_individual_stock_trade_analytics, calculate_symbol_analytics,
};
pub use main::calculate_core_metrics;

use crate::models::analytics::CoreMetrics;

impl Default for CoreMetrics {
    fn default() -> Self {
        Self {
            total_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            break_even_trades: 0,
            win_rate: 0.0,
            loss_rate: 0.0,
            total_pnl: 0.0,
            net_profit_loss: 0.0,
            gross_profit: 0.0,
            gross_loss: 0.0,
            average_win: 0.0,
            average_loss: 0.0,
            average_position_size: 0.0,
            biggest_winner: 0.0,
            biggest_loser: 0.0,
            profit_factor: 0.0,
            win_loss_ratio: 0.0,
            max_consecutive_wins: 0,
            max_consecutive_losses: 0,
            total_commissions: 0.0,
            average_commission_per_trade: 0.0,
        }
    }
}
