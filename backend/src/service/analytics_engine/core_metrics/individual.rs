use super::helpers::{get_f64_value, get_i64_value, get_optional_f64_value};
use crate::models::stock::stocks::TimeRange;
use anyhow::Result;
use libsql::Connection;
use serde::{Deserialize, Serialize};

/// Individual trade analytics for a single trade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndividualTradeAnalytics {
    pub trade_id: i64,
    pub trade_type: String, // "stock" or "option"
    pub symbol: String,
    pub net_pnl: f64,
    pub planned_risk_to_reward: Option<f64>,
    pub realized_risk_to_reward: Option<f64>,
    pub commission_impact: f64,
    pub hold_time_days: Option<f64>,
}

/// Symbol-level aggregate analytics (for repeated trades on same symbol)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolAnalytics {
    pub symbol: String,
    pub total_trades: u32,
    pub stock_trades: u32,
    pub option_trades: u32,
    pub winning_trades: u32,
    pub losing_trades: u32,
    pub net_pnl: f64,
    pub gross_profit: f64,
    pub gross_loss: f64,
    pub average_gain: f64, // Average of winning trades
    pub average_loss: f64, // Average of losing trades (absolute value)
    pub profit_factor: f64,
    pub win_rate: f64,
    pub expectancy: f64,
    pub biggest_winner: f64,
    pub biggest_loser: f64,
}

/// Calculate analytics for an individual trade (stock)
pub async fn calculate_individual_stock_trade_analytics(
    conn: &Connection,
    trade_id: i64,
) -> Result<IndividualTradeAnalytics> {
    let sql = r#"
        SELECT 
            id,
            symbol,
            trade_type,
            entry_price,
            exit_price,
            stop_loss,
            profit_target,
            number_shares,
            commissions,
            entry_date,
            exit_date
        FROM stocks
        WHERE id = ?
    "#;

    let mut rows = conn
        .prepare(sql)
        .await?
        .query(libsql::params![trade_id])
        .await?;

    if let Some(row) = rows.next().await? {
        let symbol: String = row.get(1)?;
        let trade_type_str: String = row.get(2)?;
        let entry_price: f64 = get_f64_value(&row, 3);
        let exit_price: Option<f64> = get_optional_f64_value(&row, 4);
        let stop_loss: f64 = get_f64_value(&row, 5);
        let profit_target: Option<f64> = get_optional_f64_value(&row, 6);
        let number_shares: f64 = get_f64_value(&row, 7);
        let commissions: f64 = get_f64_value(&row, 8);
        let entry_date: Option<String> = row.get(9).ok();
        let exit_date: Option<String> = row.get(10).ok();

        // Calculate net P&L
        let net_pnl = if let Some(exit) = exit_price {
            let pnl = match trade_type_str.as_str() {
                "BUY" => (exit - entry_price) * number_shares,
                "SELL" => (entry_price - exit) * number_shares,
                _ => 0.0,
            };
            pnl - commissions
        } else {
            0.0
        };

        // Calculate planned risk-to-reward ratio
        let planned_rr = if let Some(target) = profit_target {
            let risk = (entry_price - stop_loss).abs() * number_shares;
            if risk > 0.0 {
                let reward = match trade_type_str.as_str() {
                    "BUY" => (target - entry_price) * number_shares,
                    "SELL" => (entry_price - target) * number_shares,
                    _ => 0.0,
                };
                if reward > 0.0 && risk > 0.0 {
                    Some(reward / risk)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // Calculate realized risk-to-reward ratio
        let realized_rr = if let Some(_exit) = exit_price {
            let risk = (entry_price - stop_loss).abs() * number_shares;
            if risk > 0.0 {
                let realized_profit = net_pnl + commissions; // Add back commissions to get gross profit
                if realized_profit != 0.0 && risk > 0.0 {
                    Some(realized_profit / risk)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let commission_impact = if net_pnl != 0.0 {
            (commissions / net_pnl.abs()) * 100.0
        } else {
            0.0
        };

        // Calculate hold time in days
        let hold_time_days = match (entry_date, exit_date) {
            (Some(entry), Some(exit)) => {
                if let (Ok(start), Ok(end)) = (
                    chrono::DateTime::parse_from_rfc3339(&entry),
                    chrono::DateTime::parse_from_rfc3339(&exit),
                ) {
                    let secs = (end - start).num_seconds();
                    Some(secs as f64 / 86_400.0)
                } else {
                    None
                }
            }
            _ => None,
        };

        Ok(IndividualTradeAnalytics {
            trade_id,
            trade_type: "stock".to_string(),
            symbol,
            net_pnl,
            planned_risk_to_reward: planned_rr,
            realized_risk_to_reward: realized_rr,
            commission_impact,
            hold_time_days,
        })
    } else {
        anyhow::bail!("Trade not found: {}", trade_id)
    }
}

/// Calculate analytics for an individual option trade
pub async fn calculate_individual_option_trade_analytics(
    conn: &Connection,
    trade_id: i64,
) -> Result<IndividualTradeAnalytics> {
    let sql = r#"
        SELECT 
            id,
            symbol,
            option_type,
            entry_price,
            exit_price,
            initial_target,
            profit_target,
            total_quantity,
            premium,
            commissions,
            entry_date,
            exit_date
        FROM options
        WHERE id = ?
    "#;

    let mut rows = conn
        .prepare(sql)
        .await?
        .query(libsql::params![trade_id])
        .await?;

    if let Some(row) = rows.next().await? {
        let symbol: String = row.get(1)?;
        let entry_price: f64 = get_f64_value(&row, 3);
        let exit_price: Option<f64> = get_optional_f64_value(&row, 4);
        let _initial_target: Option<f64> = get_optional_f64_value(&row, 5);
        let profit_target: Option<f64> = get_optional_f64_value(&row, 6);
        let quantity: i32 = get_i64_value(&row, 7) as i32;
        let premium: f64 = get_f64_value(&row, 8);
        let commissions: f64 = get_f64_value(&row, 9);
        let entry_date: Option<String> = row.get(10).ok();
        let exit_date: Option<String> = row.get(11).ok();

        // Calculate net P&L for options
        let net_pnl = if let Some(exit) = exit_price {
            // Option P&L = (exit_price - entry_price) * contracts * 100 - commissions
            let pnl = (exit - entry_price) * quantity as f64 * 100.0;
            pnl - commissions
        } else {
            0.0
        };

        // For options, risk is the premium paid
        // Planned R:R uses profit_target
        let planned_rr = if let Some(target) = profit_target {
            let risk = premium;
            if risk > 0.0 {
                let reward = (target - entry_price) * quantity as f64 * 100.0;
                if reward > 0.0 {
                    Some(reward / risk)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // Realized R:R
        let realized_rr = if let Some(_exit) = exit_price {
            if premium > 0.0 {
                let realized_profit = net_pnl + commissions;
                Some(realized_profit / premium)
            } else {
                None
            }
        } else {
            None
        };

        let commission_impact = if net_pnl != 0.0 {
            (commissions / net_pnl.abs()) * 100.0
        } else {
            0.0
        };

        // Calculate hold time in days
        let hold_time_days = match (entry_date, exit_date) {
            (Some(entry), Some(exit)) => {
                if let (Ok(start), Ok(end)) = (
                    chrono::DateTime::parse_from_rfc3339(&entry),
                    chrono::DateTime::parse_from_rfc3339(&exit),
                ) {
                    let secs = (end - start).num_seconds();
                    Some(secs as f64 / 86_400.0)
                } else {
                    None
                }
            }
            _ => None,
        };

        Ok(IndividualTradeAnalytics {
            trade_id,
            trade_type: "option".to_string(),
            symbol,
            net_pnl,
            planned_risk_to_reward: planned_rr,
            realized_risk_to_reward: realized_rr,
            commission_impact,
            hold_time_days,
        })
    } else {
        anyhow::bail!("Trade not found: {}", trade_id)
    }
}

/// Calculate symbol-level analytics (for repeated trades on the same symbol)
pub async fn calculate_symbol_analytics(
    conn: &Connection,
    symbol: &str,
    time_range: &TimeRange,
) -> Result<SymbolAnalytics> {
    let (time_condition, time_params) = time_range.to_sql_condition();

    // Get stocks analytics for this symbol
    let stocks_sql = format!(
        r#"
        SELECT 
            COUNT(*) as total_trades,
            SUM(CASE WHEN calculated_pnl > 0 THEN 1 ELSE 0 END) as winning_trades,
            SUM(CASE WHEN calculated_pnl < 0 THEN 1 ELSE 0 END) as losing_trades,
            SUM(calculated_pnl) as net_pnl,
            SUM(CASE WHEN calculated_pnl > 0 THEN calculated_pnl ELSE 0 END) as gross_profit,
            SUM(CASE WHEN calculated_pnl < 0 THEN calculated_pnl ELSE 0 END) as gross_loss,
            AVG(CASE WHEN calculated_pnl > 0 THEN calculated_pnl END) as avg_gain,
            AVG(CASE WHEN calculated_pnl < 0 THEN calculated_pnl END) as avg_loss,
            MAX(calculated_pnl) as biggest_winner,
            MIN(calculated_pnl) as biggest_loser
        FROM (
            SELECT 
                CASE 
                    WHEN trade_type = 'BUY' THEN (exit_price - entry_price) * number_shares - commissions
                    WHEN trade_type = 'SELL' THEN (entry_price - exit_price) * number_shares - commissions
                    ELSE 0
                END as calculated_pnl
            FROM stocks
            WHERE symbol = ? AND exit_price IS NOT NULL AND exit_date IS NOT NULL AND ({})
        )
        "#,
        time_condition
    );

    let mut query_params = vec![libsql::Value::Text(symbol.to_string())];
    for param in &time_params {
        query_params.push(libsql::Value::Text(param.to_rfc3339()));
    }

    let mut rows = conn
        .prepare(&stocks_sql)
        .await?
        .query(libsql::params_from_iter(query_params.clone()))
        .await?;

    let mut stock_trades = 0u32;
    let mut winning_trades_stocks = 0u32;
    let mut losing_trades_stocks = 0u32;
    let mut net_pnl_stocks = 0.0;
    let mut gross_profit_stocks = 0.0;
    let mut gross_loss_stocks = 0.0;
    let mut avg_gain_stocks = 0.0;
    let mut avg_loss_stocks = 0.0;
    let mut biggest_winner_stocks = f64::MIN;
    let mut biggest_loser_stocks = f64::MAX;

    if let Some(row) = rows.next().await? {
        stock_trades = get_i64_value(&row, 0) as u32;
        winning_trades_stocks = get_i64_value(&row, 1) as u32;
        losing_trades_stocks = get_i64_value(&row, 2) as u32;
        net_pnl_stocks = get_f64_value(&row, 3);
        gross_profit_stocks = get_f64_value(&row, 4);
        gross_loss_stocks = get_f64_value(&row, 5);
        avg_gain_stocks = get_f64_value(&row, 6);
        avg_loss_stocks = get_f64_value(&row, 7);
        biggest_winner_stocks = get_f64_value(&row, 8);
        biggest_loser_stocks = get_f64_value(&row, 9);
    }

    // Get options analytics for this symbol
    let options_sql = format!(
        r#"
        SELECT 
            COUNT(*) as total_trades,
            SUM(CASE WHEN calculated_pnl > 0 THEN 1 ELSE 0 END) as winning_trades,
            SUM(CASE WHEN calculated_pnl < 0 THEN 1 ELSE 0 END) as losing_trades,
            SUM(calculated_pnl) as net_pnl,
            SUM(CASE WHEN calculated_pnl > 0 THEN calculated_pnl ELSE 0 END) as gross_profit,
            SUM(CASE WHEN calculated_pnl < 0 THEN calculated_pnl ELSE 0 END) as gross_loss,
            AVG(CASE WHEN calculated_pnl > 0 THEN calculated_pnl END) as avg_gain,
            AVG(CASE WHEN calculated_pnl < 0 THEN calculated_pnl END) as avg_loss,
            MAX(calculated_pnl) as biggest_winner,
            MIN(calculated_pnl) as biggest_loser
        FROM (
            SELECT 
                CASE 
                    WHEN exit_price IS NOT NULL THEN 
                        (exit_price - entry_price) * COALESCE(total_quantity, 1) * 100 - COALESCE(commissions, 0)
                    ELSE 0
                END as calculated_pnl
            FROM options
            WHERE symbol = ? AND status = 'closed' AND exit_price IS NOT NULL AND ({})
        )
        "#,
        time_condition
    );

    let mut query_params = vec![libsql::Value::Text(symbol.to_string())];
    for param in &time_params {
        query_params.push(libsql::Value::Text(param.to_rfc3339()));
    }

    let mut rows = conn
        .prepare(&options_sql)
        .await?
        .query(libsql::params_from_iter(query_params))
        .await?;

    let mut option_trades = 0u32;
    let mut winning_trades_options = 0u32;
    let mut losing_trades_options = 0u32;
    let mut net_pnl_options = 0.0;
    let mut gross_profit_options = 0.0;
    let mut gross_loss_options = 0.0;
    let mut avg_gain_options = 0.0;
    let mut avg_loss_options = 0.0;
    let mut biggest_winner_options = f64::MIN;
    let mut biggest_loser_options = f64::MAX;

    if let Some(row) = rows.next().await? {
        option_trades = get_i64_value(&row, 0) as u32;
        winning_trades_options = get_i64_value(&row, 1) as u32;
        losing_trades_options = get_i64_value(&row, 2) as u32;
        net_pnl_options = get_f64_value(&row, 3);
        gross_profit_options = get_f64_value(&row, 4);
        gross_loss_options = get_f64_value(&row, 5);
        avg_gain_options = get_f64_value(&row, 6);
        avg_loss_options = get_f64_value(&row, 7);
        biggest_winner_options = get_f64_value(&row, 8);
        biggest_loser_options = get_f64_value(&row, 9);
    }

    // Combine metrics
    let total_trades = stock_trades + option_trades;
    let total_winning = winning_trades_stocks + winning_trades_options;
    let total_losing = losing_trades_stocks + losing_trades_options;
    let net_pnl = net_pnl_stocks + net_pnl_options;
    let gross_profit = gross_profit_stocks + gross_profit_options;
    let gross_loss = gross_loss_stocks + gross_loss_options;

    // Weighted average gain
    let total_avg_gain = if total_winning > 0 {
        (avg_gain_stocks * winning_trades_stocks as f64
            + avg_gain_options * winning_trades_options as f64)
            / total_winning as f64
    } else {
        0.0
    };

    // Weighted average loss
    let total_avg_loss = if total_losing > 0 {
        (avg_loss_stocks * losing_trades_stocks as f64
            + avg_loss_options * losing_trades_options as f64)
            / total_losing as f64
    } else {
        0.0
    };

    let win_rate = if total_trades > 0 {
        (total_winning as f64 / total_trades as f64) * 100.0
    } else {
        0.0
    };

    let profit_factor = if gross_loss.abs() > 0.0 {
        gross_profit / gross_loss.abs()
    } else if gross_profit > 0.0 {
        f64::INFINITY
    } else {
        0.0
    };

    let expectancy = if total_trades > 0 {
        (total_avg_gain * (total_winning as f64 / total_trades as f64))
            + (total_avg_loss * (total_losing as f64 / total_trades as f64))
    } else {
        0.0
    };

    Ok(SymbolAnalytics {
        symbol: symbol.to_string(),
        total_trades,
        stock_trades,
        option_trades,
        winning_trades: total_winning,
        losing_trades: total_losing,
        net_pnl,
        gross_profit,
        gross_loss,
        average_gain: total_avg_gain,
        average_loss: total_avg_loss.abs(),
        profit_factor,
        win_rate,
        expectancy,
        biggest_winner: biggest_winner_stocks.max(biggest_winner_options),
        biggest_loser: biggest_loser_stocks.min(biggest_loser_options),
    })
}
