use crate::models::analytics::CoreMetrics;
use anyhow::Result;

/// Helper structures for internal calculations

#[derive(Debug)]
pub struct DrawdownMetrics {
    pub maximum_drawdown: f64,
    pub maximum_drawdown_percentage: f64,
    pub maximum_drawdown_duration_days: u32,
    pub current_drawdown: f64,
    pub ulcer_index: f64,
}

#[derive(Debug)]
pub struct PositionSizing {
    pub avg_size: f64,
    pub std_dev: f64,
    pub variability: f64,
}

/// Helper function to safely extract f64 from libsql::Value
pub fn get_f64_value(row: &libsql::Row, index: usize) -> f64 {
    match row.get::<libsql::Value>(index as i32) {
        Ok(libsql::Value::Integer(i)) => i as f64,
        Ok(libsql::Value::Real(f)) => f,
        Ok(libsql::Value::Null) => 0.0,
        _ => 0.0,
    }
}

/// Helper function to safely extract i64 from libsql::Value
pub fn get_i64_value(row: &libsql::Row, index: usize) -> i64 {
    match row.get::<libsql::Value>(index as i32) {
        Ok(libsql::Value::Integer(i)) => i,
        Ok(libsql::Value::Null) => 0,
        _ => 0,
    }
}

/// Helper function to build CoreMetrics from a row
pub fn build_core_metrics_from_row(row: &libsql::Row) -> Result<CoreMetrics> {
    let total_trades = get_i64_value(row, 0) as u32;
    let winning_trades = get_i64_value(row, 1) as u32;
    let losing_trades = get_i64_value(row, 2) as u32;
    let break_even_trades = get_i64_value(row, 3) as u32;
    let total_pnl = get_f64_value(row, 4);
    let gross_profit = get_f64_value(row, 5);
    let gross_loss = get_f64_value(row, 6);
    let average_win = get_f64_value(row, 7);
    let average_loss = get_f64_value(row, 8);
    let biggest_winner = get_f64_value(row, 9);
    let biggest_loser = get_f64_value(row, 10);
    let total_commissions = get_f64_value(row, 11);
    let average_commission_per_trade = get_f64_value(row, 12);
    let average_position_size = get_f64_value(row, 13);

    let win_rate = if total_trades > 0 {
        (winning_trades as f64 / total_trades as f64) * 100.0
    } else {
        0.0
    };

    let loss_rate = if total_trades > 0 {
        (losing_trades as f64 / total_trades as f64) * 100.0
    } else {
        0.0
    };

    let profit_factor = if gross_loss != 0.0 {
        gross_profit.abs() / gross_loss.abs()
    } else if gross_profit > 0.0 {
        f64::INFINITY
    } else {
        0.0
    };

    let win_loss_ratio = if average_loss != 0.0 {
        average_win.abs() / average_loss.abs()
    } else if average_win > 0.0 {
        f64::INFINITY
    } else {
        0.0
    };

    Ok(CoreMetrics {
        total_trades,
        winning_trades,
        losing_trades,
        break_even_trades,
        win_rate,
        loss_rate,
        total_pnl,
        net_profit_loss: total_pnl,
        gross_profit,
        gross_loss,
        average_win,
        average_loss,
        average_position_size,
        biggest_winner,
        biggest_loser,
        profit_factor,
        win_loss_ratio,
        max_consecutive_wins: 0,
        max_consecutive_losses: 0,
        total_commissions,
        average_commission_per_trade,
    })
}

/// Calculate consecutive streaks from a sequence of P&L values
pub fn calculate_streaks(trades: &[f64]) -> (u32, u32) {
    let mut current_wins = 0;
    let mut current_losses = 0;
    let mut max_wins = 0;
    let mut max_losses = 0;

    for &pnl in trades {
        if pnl > 0.0 {
            // Win
            current_wins += 1;
            current_losses = 0;
            max_wins = max_wins.max(current_wins);
        } else if pnl < 0.0 {
            // Loss
            current_losses += 1;
            current_wins = 0;
            max_losses = max_losses.max(current_losses);
        } else {
            // Break-even - resets streak
            current_wins = 0;
            current_losses = 0;
        }
    }

    (max_wins, max_losses)
}

/// Calculate drawdown metrics from daily returns
pub async fn calculate_drawdown_metrics(daily_returns: &[f64]) -> Result<DrawdownMetrics> {
    if daily_returns.is_empty() {
        return Ok(DrawdownMetrics {
            maximum_drawdown: 0.0,
            maximum_drawdown_percentage: 0.0,
            maximum_drawdown_duration_days: 0,
            current_drawdown: 0.0,
            ulcer_index: 0.0,
        });
    }

    let mut cumulative_pnl: f64 = 0.0;
    let mut peak: f64 = 0.0;
    let mut max_drawdown: f64 = 0.0;
    let mut max_drawdown_percentage: f64 = 0.0;
    let mut current_drawdown_duration = 0;
    let mut max_drawdown_duration = 0;
    let mut ulcer_sum: f64 = 0.0;

    for &pnl in daily_returns {
        cumulative_pnl += pnl;

        if cumulative_pnl > peak {
            peak = cumulative_pnl;
            current_drawdown_duration = 0;
        } else {
            current_drawdown_duration += 1;
            max_drawdown_duration = max_drawdown_duration.max(current_drawdown_duration);
        }

        let drawdown: f64 = peak - cumulative_pnl;
        max_drawdown = max_drawdown.max(drawdown);

        if peak > 0.0 {
            let drawdown_percentage: f64 = (drawdown / peak) * 100.0;
            max_drawdown_percentage = max_drawdown_percentage.max(drawdown_percentage);
            ulcer_sum += ((drawdown_percentage / 100.0) * 100.0).powi(2);
        }
    }

    let current_drawdown = peak - cumulative_pnl;
    let ulcer_index = (ulcer_sum / daily_returns.len() as f64).sqrt();

    Ok(DrawdownMetrics {
        maximum_drawdown: max_drawdown,
        maximum_drawdown_percentage: max_drawdown_percentage,
        maximum_drawdown_duration_days: max_drawdown_duration,
        current_drawdown,
        ulcer_index,
    })
}
