use super::utils::*;
use crate::models::analytics::{
    CoreMetrics, GroupType, GroupedMetrics, PerformanceMetrics, RiskMetrics,
};
use crate::models::stock::stocks::TimeRange;
use anyhow::Result;
use libsql::Connection;
use std::collections::HashMap;

/// Calculate analytics grouped by strategy (options only)
pub async fn calculate_strategy_grouped_analytics(
    conn: &Connection,
    time_range: &TimeRange,
) -> Result<HashMap<String, GroupedMetrics>> {
    let (time_condition, time_params) = time_range.to_sql_condition();
    let mut grouped_analytics = HashMap::new();

    // Get all strategies from options table
    let sql = format!(
        r#"
        SELECT DISTINCT strategy_type
        FROM options
        WHERE status = 'closed' AND exit_price IS NOT NULL AND ({})
        "#,
        time_condition
    );

    let mut query_params = Vec::new();
    for param in &time_params {
        query_params.push(libsql::Value::Text(param.to_rfc3339()));
    }

    let mut rows = conn
        .prepare(&sql)
        .await?
        .query(libsql::params_from_iter(query_params.clone()))
        .await?;

    let mut strategies = Vec::new();
    while let Some(row) = rows.next().await? {
        if let Ok(strategy) = row.get::<String>(0) {
            strategies.push(strategy);
        }
    }

    // Calculate analytics for each strategy
    for strategy in strategies {
        let core_metrics =
            calculate_strategy_core_metrics(conn, &strategy, &time_condition, &time_params).await?;
        let risk_metrics =
            calculate_strategy_risk_metrics(conn, &strategy, &time_condition, &time_params).await?;
        let performance_metrics =
            calculate_strategy_performance_metrics(conn, &strategy, &time_condition, &time_params)
                .await?;

        grouped_analytics.insert(
            strategy.clone(),
            GroupedMetrics {
                group_name: strategy.clone(),
                group_type: GroupType::Strategy,
                core_metrics,
                risk_metrics,
                performance_metrics,
            },
        );
    }

    Ok(grouped_analytics)
}

/// Calculate core metrics for a specific strategy
async fn calculate_strategy_core_metrics(
    conn: &Connection,
    strategy: &str,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<CoreMetrics> {
    // Strategies are only in the options table
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
            WHERE strategy_type = ? AND status = 'closed' AND ({})
        )
        "#,
        time_condition
    );

    let mut query_params = vec![libsql::Value::Text(strategy.to_string())];
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

        // Calculate consecutive streaks for this strategy
        let (max_consecutive_wins, max_consecutive_losses) =
            calculate_strategy_consecutive_streaks(conn, strategy, time_condition, time_params)
                .await?;

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

/// Calculate risk metrics for a specific strategy
async fn calculate_strategy_risk_metrics(
    conn: &Connection,
    strategy: &str,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<RiskMetrics> {
    // Calculate average risk per trade for this strategy
    let avg_risk_per_trade =
        calculate_strategy_avg_risk(conn, strategy, time_condition, time_params).await?;

    // Calculate daily returns for this strategy
    let daily_returns =
        calculate_strategy_daily_returns(conn, strategy, time_condition, time_params).await?;

    // Calculate drawdown metrics (reuse the symbol version)
    let drawdown_metrics = calculate_drawdown_metrics(&daily_returns).await?;

    // Calculate volatility and other metrics (reuse the symbol version logic)
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

    let sharpe_ratio = if volatility > 0.0 {
        let mean_return = daily_returns.iter().sum::<f64>() / daily_returns.len() as f64;
        mean_return / volatility * (252.0_f64).sqrt()
    } else {
        0.0
    };

    let sortino_ratio = if downside_deviation > 0.0 {
        let mean_return = daily_returns.iter().sum::<f64>() / daily_returns.len() as f64;
        mean_return / downside_deviation * (252.0_f64).sqrt()
    } else {
        0.0
    };

    let recovery_factor = if drawdown_metrics.maximum_drawdown > 0.0 {
        let total_return = daily_returns.iter().sum::<f64>();
        total_return / drawdown_metrics.maximum_drawdown
    } else {
        0.0
    };

    let calmar_ratio = if drawdown_metrics.maximum_drawdown_percentage > 0.0 {
        let mean_return = daily_returns.iter().sum::<f64>() / daily_returns.len() as f64;
        mean_return * 252.0 / drawdown_metrics.maximum_drawdown_percentage
    } else {
        0.0
    };

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
        risk_reward_ratio: 0.0,
        volatility,
        downside_deviation,
        ulcer_index: drawdown_metrics.ulcer_index,
        recovery_factor,
        sterling_ratio: 0.0,
    })
}

/// Calculate performance metrics for a specific strategy
async fn calculate_strategy_performance_metrics(
    conn: &Connection,
    strategy: &str,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<PerformanceMetrics> {
    // Get core metrics
    let core = calculate_strategy_core_metrics(conn, strategy, time_condition, time_params).await?;

    let trade_expectancy = if core.total_trades > 0 {
        (core.average_win * core.win_rate / 100.0)
            - (core.average_loss.abs() * core.loss_rate / 100.0)
    } else {
        0.0
    };

    let edge = if core.average_position_size > 0.0 {
        trade_expectancy / core.average_position_size
    } else {
        0.0
    };

    // Calculate hold times for options
    let avg_hold_time =
        calculate_strategy_avg_hold_time(conn, strategy, time_condition, time_params).await?;
    let winners_hold_time =
        calculate_strategy_winners_hold_time(conn, strategy, time_condition, time_params).await?;
    let losers_hold_time =
        calculate_strategy_losers_hold_time(conn, strategy, time_condition, time_params).await?;

    // Calculate position sizing
    let position_size =
        calculate_strategy_position_sizing(conn, strategy, time_condition, time_params).await?;

    let payoff_ratio = if core.average_loss != 0.0 {
        core.average_win / core.average_loss.abs()
    } else {
        0.0
    };

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
        kelly_criterion: 0.0,
        system_quality_number: 0.0,
        payoff_ratio,
        average_r_multiple: 0.0,
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

// Helper functions for strategy-specific calculations
async fn calculate_strategy_avg_risk(
    conn: &Connection,
    strategy: &str,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<f64> {
    let sql = format!(
        r#"
        SELECT AVG(premium) as avg_risk
        FROM options
        WHERE strategy_type = ? AND status = 'closed' AND ({})
        "#,
        time_condition
    );

    let mut query_params = vec![libsql::Value::Text(strategy.to_string())];
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

async fn calculate_strategy_daily_returns(
    conn: &Connection,
    strategy: &str,
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
                    WHEN exit_price IS NOT NULL THEN 
                        (exit_price - entry_price) * total_quantity * 100 - commissions
                    ELSE 0
                END as calculated_pnl
            FROM options
            WHERE strategy_type = ? AND status = 'closed' AND exit_price IS NOT NULL AND ({})
        )
        GROUP BY DATE(exit_date)
        ORDER BY trade_date
        "#,
        time_condition
    );

    let mut query_params = vec![libsql::Value::Text(strategy.to_string())];
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

async fn calculate_strategy_avg_hold_time(
    conn: &Connection,
    strategy: &str,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<f64> {
    let sql = format!(
        r#"
        SELECT AVG(JULIANDAY(exit_date) - JULIANDAY(entry_date)) as avg_hold_time
        FROM options
        WHERE strategy_type = ? AND status = 'closed' AND exit_date IS NOT NULL AND ({})
        "#,
        time_condition
    );

    let mut query_params = vec![libsql::Value::Text(strategy.to_string())];
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

async fn calculate_strategy_winners_hold_time(
    conn: &Connection,
    strategy: &str,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<f64> {
    let sql = format!(
        r#"
        SELECT AVG(JULIANDAY(exit_date) - JULIANDAY(entry_date)) as avg_hold_time_winners
        FROM options
        WHERE strategy_type = ? AND status = 'closed' AND exit_date IS NOT NULL AND ({})
          AND exit_price > entry_price
        "#,
        time_condition
    );

    let mut query_params = vec![libsql::Value::Text(strategy.to_string())];
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

async fn calculate_strategy_losers_hold_time(
    conn: &Connection,
    strategy: &str,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<f64> {
    let sql = format!(
        r#"
        SELECT AVG(JULIANDAY(exit_date) - JULIANDAY(entry_date)) as avg_hold_time_losers
        FROM options
        WHERE strategy_type = ? AND status = 'closed' AND exit_date IS NOT NULL AND ({})
          AND exit_price < entry_price
        "#,
        time_condition
    );

    let mut query_params = vec![libsql::Value::Text(strategy.to_string())];
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

async fn calculate_strategy_position_sizing(
    conn: &Connection,
    strategy: &str,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<PositionSizing> {
    let sql = format!(
        r#"
        SELECT 
            AVG(premium) as avg_position_size,
            STDDEV(premium) as position_size_std_dev
        FROM options
        WHERE strategy_type = ? AND status = 'closed' AND ({})
        "#,
        time_condition
    );

    let mut query_params = vec![libsql::Value::Text(strategy.to_string())];
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

/// Calculate consecutive streaks for a specific strategy
async fn calculate_strategy_consecutive_streaks(
    conn: &Connection,
    strategy: &str,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<(u32, u32)> {
    let sql = format!(
        r#"
        SELECT 
            CASE 
                WHEN exit_price IS NOT NULL THEN 
                    (exit_price - entry_price) * total_quantity * 100 - commissions
                ELSE 0
            END as calculated_pnl
        FROM options
        WHERE strategy_type = ? AND status = 'closed' AND ({})
        ORDER BY exit_date ASC
        "#,
        time_condition
    );

    let mut query_params = vec![libsql::Value::Text(strategy.to_string())];
    for param in time_params {
        query_params.push(libsql::Value::Text(param.to_rfc3339()));
    }

    let mut trades = Vec::new();
    if let Ok(mut rows) = conn
        .prepare(&sql)
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
