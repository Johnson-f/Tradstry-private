use super::helpers::{get_f64_value, get_i64_value};
use super::streaks::calculate_streaks;
use crate::models::analytics::CoreMetrics;
use anyhow::Result;
use libsql::Connection;

/// Calculate core metrics for stocks table
pub(crate) async fn calculate_stocks_core_metrics(
    conn: &Connection,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<CoreMetrics> {
    let sql = format!(
        r#"
        SELECT 
            COUNT(*) as total_trades,
            SUM(CASE WHEN calculated_pnl > 0 THEN 1 ELSE 0 END) as winning_trades,
            SUM(CASE WHEN calculated_pnl < 0 THEN 1 ELSE 0 END) as losing_trades,
            SUM(CASE WHEN calculated_pnl = 0 THEN 1 ELSE 0 END) as break_even_trades,
            SUM(calculated_pnl) as total_pnl,
            SUM(CASE WHEN calculated_pnl > 0 THEN calculated_pnl ELSE 0 END) as gross_profit,
            SUM(CASE WHEN calculated_pnl < 0 THEN calculated_pnl ELSE 0 END) as gross_loss,
            AVG(CASE WHEN calculated_pnl > 0 THEN calculated_pnl END) as average_win,
            AVG(CASE WHEN calculated_pnl < 0 THEN calculated_pnl END) as average_loss,
            MAX(calculated_pnl) as biggest_winner,
            MIN(calculated_pnl) as biggest_loser,
            SUM(commissions) as total_commissions,
            AVG(commissions) as average_commission_per_trade,
            AVG(number_shares * entry_price) as average_position_size
        FROM (
            SELECT 
                *,
                CASE 
                    WHEN trade_type = 'BUY' THEN (exit_price - entry_price) * number_shares - commissions
                    WHEN trade_type = 'SELL' THEN (entry_price - exit_price) * number_shares - commissions
                    ELSE 0
                END as calculated_pnl
            FROM stocks
            WHERE exit_price IS NOT NULL AND exit_date IS NOT NULL AND ({})
        )
        "#,
        time_condition
    );

    let mut query_params = Vec::new();
    for param in time_params {
        query_params.push(libsql::Value::Text(param.to_rfc3339()));
    }

    let mut rows = conn
        .prepare(&sql)
        .await?
        .query(libsql::params_from_iter(query_params))
        .await?;

    if let Some(row) = rows.next().await? {
        let total_trades = get_i64_value(&row, 0) as u32;
        let winning_trades = get_i64_value(&row, 1) as u32;
        let losing_trades = get_i64_value(&row, 2) as u32;
        let break_even_trades = get_i64_value(&row, 3) as u32;
        let total_pnl = get_f64_value(&row, 4);
        let gross_profit = get_f64_value(&row, 5);
        let gross_loss = get_f64_value(&row, 6);
        let average_win = get_f64_value(&row, 7);
        let average_loss = get_f64_value(&row, 8);
        let biggest_winner = get_f64_value(&row, 9);
        let biggest_loser = get_f64_value(&row, 10);
        let total_commissions = get_f64_value(&row, 11);
        let average_commission_per_trade = get_f64_value(&row, 12);
        let average_position_size = get_f64_value(&row, 13);

        // Calculate derived metrics
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

        // Calculate consecutive streaks for stocks
        let (max_consecutive_wins, max_consecutive_losses) =
            calculate_stocks_consecutive_streaks(conn, time_condition, time_params).await?;

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
            max_consecutive_wins,
            max_consecutive_losses,
            total_commissions,
            average_commission_per_trade,
        })
    } else {
        Ok(CoreMetrics::default())
    }
}

/// Calculate consecutive streaks for stocks
pub(crate) async fn calculate_stocks_consecutive_streaks(
    conn: &Connection,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<(u32, u32)> {
    // Get all trades ordered by exit_date to track consecutive streaks
    let sql = format!(
        r#"
        SELECT 
            CASE 
                WHEN trade_type = 'BUY' THEN (exit_price - entry_price) * number_shares - commissions
                WHEN trade_type = 'SELL' THEN (entry_price - exit_price) * number_shares - commissions
                ELSE 0
            END as calculated_pnl
        FROM stocks
        WHERE exit_price IS NOT NULL AND exit_date IS NOT NULL AND ({})
        ORDER BY exit_date ASC
        "#,
        time_condition
    );

    let mut query_params = Vec::new();
    for param in time_params {
        query_params.push(libsql::Value::Text(param.to_rfc3339()));
    }

    let mut rows = conn
        .prepare(&sql)
        .await?
        .query(libsql::params_from_iter(query_params))
        .await?;

    let mut trades = Vec::new();
    while let Some(row) = rows.next().await? {
        let pnl = get_f64_value(&row, 0);
        trades.push(pnl);
    }

    let (max_wins, max_losses) = calculate_streaks(&trades);
    Ok((max_wins, max_losses))
}
