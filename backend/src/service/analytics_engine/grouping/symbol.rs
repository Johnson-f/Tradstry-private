use super::utils::*;
use crate::models::analytics::{
    CoreMetrics, GroupType, GroupedMetrics, PerformanceMetrics, RiskMetrics,
};
use crate::models::stock::stocks::TimeRange;
use anyhow::Result;
use libsql::Connection;
use std::collections::HashMap;

/// Calculate analytics grouped by symbol
pub async fn calculate_symbol_grouped_analytics(
    conn: &Connection,
    time_range: &TimeRange,
) -> Result<HashMap<String, GroupedMetrics>> {
    let (time_condition, time_params) = time_range.to_sql_condition();
    let mut grouped_analytics = HashMap::new();

    // Get all symbols from stocks table
    let stocks_sql = format!(
        r#"
        SELECT DISTINCT symbol
        FROM stocks
        WHERE exit_price IS NOT NULL AND exit_date IS NOT NULL AND ({})
        "#,
        time_condition
    );

    let mut query_params = Vec::new();
    for param in &time_params {
        query_params.push(libsql::Value::Text(param.to_rfc3339()));
    }

    let mut rows = conn
        .prepare(&stocks_sql)
        .await?
        .query(libsql::params_from_iter(query_params.clone()))
        .await?;

    let mut symbols = Vec::new();
    while let Some(row) = rows.next().await? {
        if let Ok(symbol) = row.get::<String>(0) {
            symbols.push(symbol);
        }
    }

    // Get all symbols from options table
    let options_sql = format!(
        r#"
        SELECT DISTINCT symbol
        FROM options
        WHERE status = 'closed' AND exit_price IS NOT NULL AND ({})
        "#,
        time_condition
    );

    let mut rows = conn
        .prepare(&options_sql)
        .await?
        .query(libsql::params_from_iter(query_params))
        .await?;

    while let Some(row) = rows.next().await? {
        if let Ok(symbol) = row.get::<String>(0)
            && !symbols.contains(&symbol)
        {
            symbols.push(symbol);
        }
    }

    // Calculate analytics for each symbol
    for symbol in symbols {
        let core_metrics =
            calculate_symbol_core_metrics(conn, &symbol, &time_condition, &time_params).await?;
        let risk_metrics =
            calculate_symbol_risk_metrics(conn, &symbol, &time_condition, &time_params).await?;
        let performance_metrics =
            calculate_symbol_performance_metrics(conn, &symbol, &time_condition, &time_params)
                .await?;

        grouped_analytics.insert(
            symbol.clone(),
            GroupedMetrics {
                group_name: symbol.clone(),
                group_type: GroupType::Symbol,
                core_metrics,
                risk_metrics,
                performance_metrics,
            },
        );
    }

    Ok(grouped_analytics)
}

/// Calculate core metrics for a specific symbol
async fn calculate_symbol_core_metrics(
    conn: &Connection,
    symbol: &str,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<CoreMetrics> {
    // Calculate stocks metrics for this symbol
    let stocks_metrics =
        calculate_symbol_stocks_metrics(conn, symbol, time_condition, time_params).await?;

    // Calculate options metrics for this symbol
    let options_metrics =
        calculate_symbol_options_metrics(conn, symbol, time_condition, time_params).await?;

    // Combine metrics using similar logic to core_metrics.rs
    let total_trades = stocks_metrics.total_trades + options_metrics.total_trades;
    let total_winning_trades = stocks_metrics.winning_trades + options_metrics.winning_trades;
    let total_losing_trades = stocks_metrics.losing_trades + options_metrics.losing_trades;
    let total_break_even_trades =
        stocks_metrics.break_even_trades + options_metrics.break_even_trades;

    let total_pnl = stocks_metrics.total_pnl + options_metrics.total_pnl;
    let total_gross_profit = stocks_metrics.gross_profit + options_metrics.gross_profit;
    let total_gross_loss = stocks_metrics.gross_loss + options_metrics.gross_loss;
    let total_commissions = stocks_metrics.total_commissions + options_metrics.total_commissions;

    // Weighted averages
    let average_win = if total_winning_trades > 0 {
        (stocks_metrics.average_win * stocks_metrics.winning_trades as f64
            + options_metrics.average_win * options_metrics.winning_trades as f64)
            / total_winning_trades as f64
    } else {
        0.0
    };

    let average_loss = if total_losing_trades > 0 {
        (stocks_metrics.average_loss * stocks_metrics.losing_trades as f64
            + options_metrics.average_loss * options_metrics.losing_trades as f64)
            / total_losing_trades as f64
    } else {
        0.0
    };

    let stocks_weight = if total_trades > 0 {
        stocks_metrics.total_trades as f64 / total_trades as f64
    } else {
        0.0
    };
    let average_position_size = stocks_metrics.average_position_size * stocks_weight
        + options_metrics.average_position_size * (1.0 - stocks_weight);
    let average_commission_per_trade = if total_trades > 0 {
        total_commissions / total_trades as f64
    } else {
        0.0
    };

    let biggest_winner = stocks_metrics
        .biggest_winner
        .max(options_metrics.biggest_winner);
    let biggest_loser = stocks_metrics
        .biggest_loser
        .min(options_metrics.biggest_loser);

    let win_rate = if total_trades > 0 {
        (total_winning_trades as f64 / total_trades as f64) * 100.0
    } else {
        0.0
    };
    let loss_rate = if total_trades > 0 {
        (total_losing_trades as f64 / total_trades as f64) * 100.0
    } else {
        0.0
    };

    let profit_factor = if total_gross_loss != 0.0 {
        total_gross_profit.abs() / total_gross_loss.abs()
    } else if total_gross_profit > 0.0 {
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

    // Calculate consecutive streaks for this symbol
    let (max_consecutive_wins, max_consecutive_losses) =
        calculate_symbol_consecutive_streaks(conn, symbol, time_condition, time_params).await?;

    Ok(CoreMetrics {
        total_trades,
        winning_trades: total_winning_trades,
        losing_trades: total_losing_trades,
        break_even_trades: total_break_even_trades,
        win_rate,
        loss_rate,
        total_pnl,
        net_profit_loss: total_pnl,
        gross_profit: total_gross_profit,
        gross_loss: total_gross_loss,
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
}

/// Calculate stocks metrics for a specific symbol
async fn calculate_symbol_stocks_metrics(
    conn: &Connection,
    symbol: &str,
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
            WHERE symbol = ? AND exit_price IS NOT NULL AND exit_date IS NOT NULL AND ({})
        )
        "#,
        time_condition
    );

    let mut query_params = vec![libsql::Value::Text(symbol.to_string())];
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
    } else {
        Ok(CoreMetrics::default())
    }
}

/// Calculate options metrics for a specific symbol
async fn calculate_symbol_options_metrics(
    conn: &Connection,
    symbol: &str,
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
            AVG(premium) as average_position_size
        FROM (
            SELECT 
                *,
                CASE 
                    WHEN exit_price IS NOT NULL THEN 
                        (exit_price - entry_price) * total_quantity * 100 - commissions
                    ELSE 0
                END as calculated_pnl
            FROM options
            WHERE symbol = ? AND status = 'closed' AND ({})
        )
        "#,
        time_condition
    );

    let mut query_params = vec![libsql::Value::Text(symbol.to_string())];
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
    } else {
        Ok(CoreMetrics::default())
    }
}

/// Calculate risk metrics for a specific symbol
async fn calculate_symbol_risk_metrics(
    conn: &Connection,
    symbol: &str,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<RiskMetrics> {
    // Calculate average risk per trade for this symbol
    let avg_risk_per_trade =
        calculate_symbol_avg_risk(conn, symbol, time_condition, time_params).await?;

    // Calculate daily returns for this symbol
    let daily_returns =
        calculate_symbol_daily_returns(conn, symbol, time_condition, time_params).await?;

    // Calculate drawdown metrics
    let drawdown_metrics = calculate_drawdown_metrics(&daily_returns).await?;

    // Calculate simple metrics (simplified version without full VaR)
    let volatility = if daily_returns.len() > 1 {
        let mean = daily_returns.iter().sum::<f64>() / daily_returns.len() as f64;
        let variance = daily_returns
            .iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>()
            / (daily_returns.len() - 1) as f64;
        variance.sqrt()
    } else {
        0.0
    };

    let downside_deviation = if daily_returns.len() > 1 {
        let downside_returns: Vec<f64> = daily_returns
            .iter()
            .filter(|&&x| x < 0.0)
            .cloned()
            .collect();
        let mean = downside_returns.iter().sum::<f64>() / downside_returns.len() as f64;
        let variance = downside_returns
            .iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>()
            / downside_returns.len() as f64;
        variance.sqrt()
    } else {
        0.0
    };

    // Simplified Sharpe ratio (assuming 0 risk-free rate)
    let sharpe_ratio = if volatility > 0.0 {
        let mean_return = daily_returns.iter().sum::<f64>() / daily_returns.len() as f64;
        mean_return / volatility * (252.0_f64).sqrt() // Annualized
    } else {
        0.0
    };

    // Sortino ratio
    let sortino_ratio = if downside_deviation > 0.0 {
        let mean_return = daily_returns.iter().sum::<f64>() / daily_returns.len() as f64;
        mean_return / downside_deviation * (252.0_f64).sqrt() // Annualized
    } else {
        0.0
    };

    // Recovery factor
    let recovery_factor = if drawdown_metrics.maximum_drawdown > 0.0 {
        let total_return = daily_returns.iter().sum::<f64>();
        total_return / drawdown_metrics.maximum_drawdown
    } else {
        0.0
    };

    // Calmar ratio
    let calmar_ratio = if drawdown_metrics.maximum_drawdown_percentage > 0.0 {
        let mean_return = daily_returns.iter().sum::<f64>() / daily_returns.len() as f64;
        mean_return * 252.0 / drawdown_metrics.maximum_drawdown_percentage
    } else {
        0.0
    };

    // Calculate VaR at 95% and 99% (simplified - using sorted returns)
    let mut sorted_returns = daily_returns.clone();
    sorted_returns.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let var_95 = if !sorted_returns.is_empty() {
        let index = ((1.0 - 0.95) * sorted_returns.len() as f64) as usize;
        sorted_returns.get(index).copied().unwrap_or(0.0)
    } else {
        0.0
    };

    let var_99 = if !sorted_returns.is_empty() {
        let index = ((1.0 - 0.99) * sorted_returns.len() as f64) as usize;
        sorted_returns.get(index).copied().unwrap_or(0.0)
    } else {
        0.0
    };

    // Expected shortfall (CVaR) - average of returns below VaR
    let expected_shortfall_95 = if !sorted_returns.is_empty() {
        sorted_returns.iter().filter(|&&x| x <= var_95).sum::<f64>()
            / sorted_returns.iter().filter(|&&x| x <= var_95).count() as f64
    } else {
        0.0
    };

    let expected_shortfall_99 = if !sorted_returns.is_empty() {
        sorted_returns.iter().filter(|&&x| x <= var_99).sum::<f64>()
            / sorted_returns.iter().filter(|&&x| x <= var_99).count() as f64
    } else {
        0.0
    };

    Ok(RiskMetrics {
        sharpe_ratio,
        sortino_ratio,
        calmar_ratio,
        maximum_drawdown: drawdown_metrics.maximum_drawdown,
        maximum_drawdown_percentage: drawdown_metrics.maximum_drawdown_percentage,
        maximum_drawdown_duration_days: drawdown_metrics.maximum_drawdown_duration_days,
        current_drawdown: drawdown_metrics.current_drawdown,
        var_95,
        var_99,
        expected_shortfall_95,
        expected_shortfall_99,
        average_risk_per_trade: avg_risk_per_trade,
        risk_reward_ratio: 0.0, // Could be calculated from core metrics
        volatility,
        downside_deviation,
        ulcer_index: drawdown_metrics.ulcer_index,
        recovery_factor,
        sterling_ratio: 0.0, // Simplified - not calculating full Sterling ratio
    })
}

/// Calculate performance metrics for a specific symbol
async fn calculate_symbol_performance_metrics(
    conn: &Connection,
    symbol: &str,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<PerformanceMetrics> {
    // Get core metrics to calculate expectancy and edge
    let core = calculate_symbol_core_metrics(conn, symbol, time_condition, time_params).await?;

    // Calculate trade expectancy
    let trade_expectancy = if core.total_trades > 0 {
        (core.average_win * core.win_rate / 100.0)
            - (core.average_loss.abs() * core.loss_rate / 100.0)
    } else {
        0.0
    };

    // Calculate edge
    let edge = if core.average_position_size > 0.0 {
        trade_expectancy / core.average_position_size
    } else {
        0.0
    };

    // Calculate hold times
    let avg_hold_time =
        calculate_symbol_avg_hold_time(conn, symbol, time_condition, time_params).await?;
    let winners_hold_time =
        calculate_symbol_winners_hold_time(conn, symbol, time_condition, time_params).await?;
    let losers_hold_time =
        calculate_symbol_losers_hold_time(conn, symbol, time_condition, time_params).await?;

    // Calculate position sizing
    let position_size =
        calculate_symbol_position_sizing(conn, symbol, time_condition, time_params).await?;

    // Calculate payoff ratio
    let payoff_ratio = if core.average_loss != 0.0 {
        core.average_win / core.average_loss.abs()
    } else {
        0.0
    };

    // Commission impact
    let commission_impact_percentage = if core.total_pnl != 0.0 {
        (core.total_commissions / core.total_pnl.abs()) * 100.0
    } else {
        0.0
    };

    Ok(PerformanceMetrics {
        trade_expectancy,
        edge,
        average_hold_time_days: avg_hold_time,
        average_hold_time_winners_days: winners_hold_time,
        average_hold_time_losers_days: losers_hold_time,
        average_position_size: position_size.avg_size,
        position_size_standard_deviation: position_size.std_dev,
        position_size_variability: position_size.variability,
        kelly_criterion: 0.0,       // Advanced calculation
        system_quality_number: 0.0, // Advanced calculation
        payoff_ratio,
        average_r_multiple: 0.0, // Could be calculated
        r_multiple_standard_deviation: 0.0,
        positive_r_multiple_count: 0,
        negative_r_multiple_count: 0,
        consistency_ratio: 0.0,
        monthly_win_rate: 0.0,
        quarterly_win_rate: 0.0,
        average_slippage: 0.0,
        commission_impact_percentage,
    })
}

// Helper functions for symbol-specific calculations
async fn calculate_symbol_avg_risk(
    conn: &Connection,
    symbol: &str,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<f64> {
    // Stocks risk
    let stocks_sql = format!(
        r#"
        SELECT AVG(ABS(entry_price - stop_loss) * number_shares) as avg_risk_stocks
        FROM stocks
        WHERE symbol = ? AND stop_loss IS NOT NULL AND ({})
        "#,
        time_condition
    );

    let mut query_params = vec![libsql::Value::Text(symbol.to_string())];
    for param in time_params {
        query_params.push(libsql::Value::Text(param.to_rfc3339()));
    }

    let mut stocks_risk = 0.0;
    if let Some(row) = conn
        .prepare(&stocks_sql)
        .await?
        .query(libsql::params_from_iter(query_params.clone()))
        .await?
        .next()
        .await?
    {
        stocks_risk = get_f64_value(&row, 0);
    }

    // Options risk (premium paid)
    let options_sql = format!(
        r#"
        SELECT AVG(premium) as avg_risk_options
        FROM options
        WHERE symbol = ? AND status = 'closed' AND ({})
        "#,
        time_condition
    );

    let mut options_risk = 0.0;
    if let Some(row) = conn
        .prepare(&options_sql)
        .await?
        .query(libsql::params_from_iter(query_params))
        .await?
        .next()
        .await?
    {
        options_risk = get_f64_value(&row, 0);
    }

    Ok((stocks_risk + options_risk) / 2.0)
}

async fn calculate_symbol_daily_returns(
    conn: &Connection,
    symbol: &str,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<Vec<f64>> {
    let sql = format!(
        r#"
        SELECT 
            DATE(exit_date) as trade_date,
            SUM(calculated_pnl) as daily_pnl
        FROM (
            SELECT 
                *,
                CASE 
                    WHEN trade_type = 'BUY' THEN (exit_price - entry_price) * number_shares - commissions
                    WHEN trade_type = 'SELL' THEN (entry_price - exit_price) * number_shares - commissions
                    ELSE 0
                END as calculated_pnl
            FROM stocks
            WHERE symbol = ? AND exit_price IS NOT NULL AND exit_date IS NOT NULL AND ({})
            
            UNION ALL
            
            SELECT 
                *,
                CASE 
                    WHEN exit_price IS NOT NULL THEN 
                        (exit_price - entry_price) * total_quantity * 100 - commissions
                    ELSE 0
                END as calculated_pnl
            FROM options
            WHERE symbol = ? AND status = 'closed' AND exit_price IS NOT NULL AND ({})
        )
        GROUP BY DATE(exit_date)
        ORDER BY trade_date
        "#,
        time_condition, time_condition
    );

    let mut query_params = vec![libsql::Value::Text(symbol.to_string())];
    for param in time_params {
        query_params.push(libsql::Value::Text(param.to_rfc3339()));
    }
    query_params.push(libsql::Value::Text(symbol.to_string()));
    for param in time_params {
        query_params.push(libsql::Value::Text(param.to_rfc3339()));
    }

    let mut rows = conn
        .prepare(&sql)
        .await?
        .query(libsql::params_from_iter(query_params))
        .await?;

    let mut daily_returns = Vec::new();
    while let Some(row) = rows.next().await? {
        daily_returns.push(get_f64_value(&row, 1));
    }

    Ok(daily_returns)
}

async fn calculate_symbol_avg_hold_time(
    conn: &Connection,
    symbol: &str,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<f64> {
    let sql = format!(
        r#"
        SELECT AVG(JULIANDAY(exit_date) - JULIANDAY(entry_date)) as avg_hold_time
        FROM stocks
        WHERE symbol = ? AND exit_price IS NOT NULL AND exit_date IS NOT NULL AND ({})
        "#,
        time_condition
    );

    let mut query_params = vec![libsql::Value::Text(symbol.to_string())];
    for param in time_params {
        query_params.push(libsql::Value::Text(param.to_rfc3339()));
    }

    if let Some(row) = conn
        .prepare(&sql)
        .await?
        .query(libsql::params_from_iter(query_params))
        .await?
        .next()
        .await?
    {
        Ok(get_f64_value(&row, 0))
    } else {
        Ok(0.0)
    }
}

async fn calculate_symbol_winners_hold_time(
    conn: &Connection,
    symbol: &str,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<f64> {
    let sql = format!(
        r#"
        SELECT AVG(JULIANDAY(exit_date) - JULIANDAY(entry_date)) as avg_hold_time_winners
        FROM stocks
        WHERE symbol = ? AND exit_price IS NOT NULL AND exit_date IS NOT NULL AND ({})
          AND ((trade_type = 'BUY' AND exit_price > entry_price) 
               OR (trade_type = 'SELL' AND exit_price < entry_price))
        "#,
        time_condition
    );

    let mut query_params = vec![libsql::Value::Text(symbol.to_string())];
    for param in time_params {
        query_params.push(libsql::Value::Text(param.to_rfc3339()));
    }

    if let Some(row) = conn
        .prepare(&sql)
        .await?
        .query(libsql::params_from_iter(query_params))
        .await?
        .next()
        .await?
    {
        Ok(get_f64_value(&row, 0))
    } else {
        Ok(0.0)
    }
}

async fn calculate_symbol_losers_hold_time(
    conn: &Connection,
    symbol: &str,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<f64> {
    let sql = format!(
        r#"
        SELECT AVG(JULIANDAY(exit_date) - JULIANDAY(entry_date)) as avg_hold_time_losers
        FROM stocks
        WHERE symbol = ? AND exit_price IS NOT NULL AND exit_date IS NOT NULL AND ({})
          AND ((trade_type = 'BUY' AND exit_price < entry_price)
               OR (trade_type = 'SELL' AND exit_price > entry_price))
        "#,
        time_condition
    );

    let mut query_params = vec![libsql::Value::Text(symbol.to_string())];
    for param in time_params {
        query_params.push(libsql::Value::Text(param.to_rfc3339()));
    }

    if let Some(row) = conn
        .prepare(&sql)
        .await?
        .query(libsql::params_from_iter(query_params))
        .await?
        .next()
        .await?
    {
        Ok(get_f64_value(&row, 0))
    } else {
        Ok(0.0)
    }
}

async fn calculate_symbol_position_sizing(
    conn: &Connection,
    symbol: &str,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<PositionSizing> {
    let sql = format!(
        r#"
        SELECT 
            AVG(number_shares * entry_price) as avg_position_size,
            STDDEV(number_shares * entry_price) as position_size_std_dev
        FROM stocks
        WHERE symbol = ? AND exit_price IS NOT NULL AND exit_date IS NOT NULL AND ({})
        "#,
        time_condition
    );

    let mut query_params = vec![libsql::Value::Text(symbol.to_string())];
    for param in time_params {
        query_params.push(libsql::Value::Text(param.to_rfc3339()));
    }

    let mut rows = conn
        .prepare(&sql)
        .await?
        .query(libsql::params_from_iter(query_params))
        .await?;

    if let Some(row) = rows.next().await? {
        let avg_size = get_f64_value(&row, 0);
        let std_dev = get_f64_value(&row, 1);
        let variability = if avg_size > 0.0 {
            std_dev / avg_size
        } else {
            0.0
        };

        Ok(PositionSizing {
            avg_size,
            std_dev,
            variability,
        })
    } else {
        Ok(PositionSizing {
            avg_size: 0.0,
            std_dev: 0.0,
            variability: 0.0,
        })
    }
}

/// Calculate consecutive streaks for a specific symbol
async fn calculate_symbol_consecutive_streaks(
    conn: &Connection,
    symbol: &str,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<(u32, u32)> {
    let mut trades = Vec::new();

    // Get all stocks trades for this symbol ordered by exit_date
    let stocks_sql = format!(
        r#"
        SELECT 
            CASE 
                WHEN trade_type = 'BUY' THEN (exit_price - entry_price) * number_shares - commissions
                WHEN trade_type = 'SELL' THEN (entry_price - exit_price) * number_shares - commissions
                ELSE 0
            END as calculated_pnl
        FROM stocks
        WHERE symbol = ? AND exit_price IS NOT NULL AND exit_date IS NOT NULL AND ({})
        ORDER BY exit_date ASC
        "#,
        time_condition
    );

    let mut query_params = vec![libsql::Value::Text(symbol.to_string())];
    for param in time_params {
        query_params.push(libsql::Value::Text(param.to_rfc3339()));
    }

    if let Ok(mut rows) = conn
        .prepare(&stocks_sql)
        .await?
        .query(libsql::params_from_iter(query_params.clone()))
        .await
    {
        while let Ok(Some(row)) = rows.next().await {
            trades.push(get_f64_value(&row, 0));
        }
    }

    // Get all options trades for this symbol ordered by exit_date
    let options_sql = format!(
        r#"
        SELECT 
            CASE 
                WHEN exit_price IS NOT NULL THEN 
                    (exit_price - entry_price) * total_quantity * 100 - commissions
                ELSE 0
            END as calculated_pnl
        FROM options
        WHERE symbol = ? AND status = 'closed' AND ({})
        ORDER BY exit_date ASC
        "#,
        time_condition
    );

    let mut query_params = vec![libsql::Value::Text(symbol.to_string())];
    for param in time_params {
        query_params.push(libsql::Value::Text(param.to_rfc3339()));
    }

    if let Ok(mut rows) = conn
        .prepare(&options_sql)
        .await?
        .query(libsql::params_from_iter(query_params))
        .await
    {
        while let Ok(Some(row)) = rows.next().await {
            trades.push(get_f64_value(&row, 0));
        }
    }

    let (max_wins, max_losses) = calculate_streaks(&trades);
    Ok((max_wins, max_losses))
}
