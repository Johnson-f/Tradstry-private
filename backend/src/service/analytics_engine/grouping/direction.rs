use super::utils::*;
use crate::models::analytics::{
    CoreMetrics, GroupType, GroupedMetrics, PerformanceMetrics, RiskMetrics,
};
use crate::models::stock::stocks::TimeRange;
use anyhow::Result;
use libsql::Connection;
use std::collections::HashMap;

/// Calculate analytics grouped by trade direction
pub async fn calculate_direction_grouped_analytics(
    conn: &Connection,
    time_range: &TimeRange,
) -> Result<HashMap<String, GroupedMetrics>> {
    let mut grouped_analytics = HashMap::new();

    // Bullish trades (BUY stocks, CALL options)
    let bullish_core = calculate_direction_core_metrics(conn, "bullish", time_range).await?;
    let bullish_risk = calculate_direction_risk_metrics(conn, "bullish", time_range).await?;
    let bullish_performance =
        calculate_direction_performance_metrics(conn, "bullish", time_range).await?;

    grouped_analytics.insert(
        "Bullish".to_string(),
        GroupedMetrics {
            group_name: "Bullish".to_string(),
            group_type: GroupType::TradeDirection,
            core_metrics: bullish_core,
            risk_metrics: bullish_risk,
            performance_metrics: bullish_performance,
        },
    );

    // Bearish trades (SELL stocks, PUT options)
    let bearish_core = calculate_direction_core_metrics(conn, "bearish", time_range).await?;
    let bearish_risk = calculate_direction_risk_metrics(conn, "bearish", time_range).await?;
    let bearish_performance =
        calculate_direction_performance_metrics(conn, "bearish", time_range).await?;

    grouped_analytics.insert(
        "Bearish".to_string(),
        GroupedMetrics {
            group_name: "Bearish".to_string(),
            group_type: GroupType::TradeDirection,
            core_metrics: bearish_core,
            risk_metrics: bearish_risk,
            performance_metrics: bearish_performance,
        },
    );

    Ok(grouped_analytics)
}

/// Calculate core metrics for a specific trade direction (bullish/bearish)
async fn calculate_direction_core_metrics(
    conn: &Connection,
    direction: &str,
    time_range: &TimeRange,
) -> Result<CoreMetrics> {
    let (time_condition, time_params) = time_range.to_sql_condition();

    // Determine direction filters
    let (stocks_filter, options_filter) = match direction {
        "bullish" => ("trade_type = 'BUY'", "option_type = 'CALL'"),
        "bearish" => ("trade_type = 'SELL'", "option_type = 'PUT'"),
        _ => return Ok(CoreMetrics::default()),
    };

    // Calculate stocks metrics
    let stocks_sql = format!(
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
            WHERE {} AND exit_price IS NOT NULL AND exit_date IS NOT NULL AND ({})
        )
        "#,
        stocks_filter, time_condition
    );

    let mut query_params_stocks = Vec::new();
    for param in &time_params {
        query_params_stocks.push(libsql::Value::Text(param.to_rfc3339()));
    }

    let mut stocks_metrics = CoreMetrics::default();
    if let Some(row) = conn
        .prepare(&stocks_sql)
        .await?
        .query(libsql::params_from_iter(query_params_stocks.clone()))
        .await?
        .next()
        .await?
    {
        stocks_metrics = build_core_metrics_from_row(&row)?;
    }

    // Calculate options metrics
    let options_sql = format!(
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
            WHERE {} AND status = 'closed' AND ({})
        )
        "#,
        options_filter, time_condition
    );

    let mut options_metrics = CoreMetrics::default();
    if let Some(row) = conn
        .prepare(&options_sql)
        .await?
        .query(libsql::params_from_iter(query_params_stocks))
        .await?
        .next()
        .await?
    {
        options_metrics = build_core_metrics_from_row(&row)?;
    }

    // Combine metrics
    let total_trades = stocks_metrics.total_trades + options_metrics.total_trades;
    let total_winning_trades = stocks_metrics.winning_trades + options_metrics.winning_trades;
    let total_losing_trades = stocks_metrics.losing_trades + options_metrics.losing_trades;
    let total_break_even_trades =
        stocks_metrics.break_even_trades + options_metrics.break_even_trades;

    let total_pnl = stocks_metrics.total_pnl + options_metrics.total_pnl;
    let total_gross_profit = stocks_metrics.gross_profit + options_metrics.gross_profit;
    let total_gross_loss = stocks_metrics.gross_loss + options_metrics.gross_loss;
    let total_commissions = stocks_metrics.total_commissions + options_metrics.total_commissions;

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

    // Calculate consecutive streaks for this direction
    let (max_consecutive_wins, max_consecutive_losses) =
        calculate_direction_consecutive_streaks(conn, direction, &time_condition, &time_params)
            .await?;

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

/// Calculate risk metrics for a specific trade direction
async fn calculate_direction_risk_metrics(
    conn: &Connection,
    direction: &str,
    time_range: &TimeRange,
) -> Result<RiskMetrics> {
    let (time_condition, time_params) = time_range.to_sql_condition();

    // Calculate daily returns for this direction
    let daily_returns =
        calculate_direction_daily_returns(conn, direction, &time_condition, &time_params).await?;

    // Calculate drawdown metrics
    let drawdown_metrics = calculate_drawdown_metrics(&daily_returns).await?;

    // Calculate volatility metrics
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
        average_risk_per_trade: 0.0, // Could be calculated
        risk_reward_ratio: 0.0,
        volatility,
        downside_deviation,
        ulcer_index: drawdown_metrics.ulcer_index,
        recovery_factor,
        sterling_ratio: 0.0,
    })
}

/// Calculate performance metrics for a specific trade direction
async fn calculate_direction_performance_metrics(
    conn: &Connection,
    direction: &str,
    time_range: &TimeRange,
) -> Result<PerformanceMetrics> {
    // Get core metrics
    let core = calculate_direction_core_metrics(conn, direction, time_range).await?;

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

    // Calculate hold times
    let avg_hold_time = calculate_direction_avg_hold_time(conn, direction, time_range).await?;
    let winners_hold_time =
        calculate_direction_winners_hold_time(conn, direction, time_range).await?;
    let losers_hold_time =
        calculate_direction_losers_hold_time(conn, direction, time_range).await?;

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
        average_position_size: core.average_position_size,
        position_size_standard_deviation: 0.0, // Could be calculated
        position_size_variability: 0.0,
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

// Helper functions for direction-specific calculations
async fn calculate_direction_daily_returns(
    conn: &Connection,
    direction: &str,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<Vec<f64>> {
    let (stocks_filter, options_filter) = match direction {
        "bullish" => ("trade_type = 'BUY'", "option_type = 'CALL'"),
        "bearish" => ("trade_type = 'SELL'", "option_type = 'PUT'"),
        _ => return Ok(Vec::new()),
    };

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
            WHERE {} AND exit_price IS NOT NULL AND exit_date IS NOT NULL AND ({})
            
            UNION ALL
            
            SELECT 
                *,
                CASE 
                    WHEN exit_price IS NOT NULL THEN 
                        (exit_price - entry_price) * total_quantity * 100 - commissions
                    ELSE 0
                END as calculated_pnl
            FROM options
            WHERE {} AND status = 'closed' AND exit_price IS NOT NULL AND ({})
        )
        GROUP BY DATE(exit_date)
        ORDER BY trade_date
        "#,
        stocks_filter, time_condition, options_filter, time_condition
    );

    let mut query_params = Vec::new();
    for param in time_params {
        query_params.push(libsql::Value::Text(param.to_rfc3339()));
    }
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

async fn calculate_direction_avg_hold_time(
    conn: &Connection,
    direction: &str,
    time_range: &TimeRange,
) -> Result<f64> {
    let (time_condition, time_params) = time_range.to_sql_condition();
    let (stocks_filter, options_filter) = match direction {
        "bullish" => ("trade_type = 'BUY'", "option_type = 'CALL'"),
        "bearish" => ("trade_type = 'SELL'", "option_type = 'PUT'"),
        _ => return Ok(0.0),
    };

    let sql = format!(
        r#"
        SELECT 
            (SUM(CASE WHEN source = 'stocks' THEN hold_days ELSE 0 END) + 
             SUM(CASE WHEN source = 'options' THEN hold_days ELSE 0 END)) / 
            COUNT(*) as avg_hold_time
        FROM (
            SELECT 
                JULIANDAY(exit_date) - JULIANDAY(entry_date) as hold_days,
                'stocks' as source
            FROM stocks
            WHERE {} AND exit_date IS NOT NULL AND ({})
            
            UNION ALL
            
            SELECT 
                JULIANDAY(exit_date) - JULIANDAY(entry_date) as hold_days,
                'options' as source
            FROM options
            WHERE {} AND status = 'closed' AND exit_date IS NOT NULL AND ({})
        )
        "#,
        stocks_filter, time_condition, options_filter, time_condition
    );

    let mut query_params = Vec::new();
    for param in &time_params {
        query_params.push(libsql::Value::Text(param.to_rfc3339()));
    }
    for param in &time_params {
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

async fn calculate_direction_winners_hold_time(
    conn: &Connection,
    direction: &str,
    time_range: &TimeRange,
) -> Result<f64> {
    let (time_condition, time_params) = time_range.to_sql_condition();
    let stocks_filter = match direction {
        "bullish" => "trade_type = 'BUY' AND exit_price > entry_price",
        "bearish" => "trade_type = 'SELL' AND exit_price < entry_price",
        _ => return Ok(0.0),
    };

    let sql = format!(
        r#"
        SELECT AVG(JULIANDAY(exit_date) - JULIANDAY(entry_date)) as avg_hold_time
        FROM stocks
        WHERE {} AND exit_date IS NOT NULL AND ({})
        "#,
        stocks_filter, time_condition
    );

    let mut query_params = Vec::new();
    for param in &time_params {
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

async fn calculate_direction_losers_hold_time(
    conn: &Connection,
    direction: &str,
    time_range: &TimeRange,
) -> Result<f64> {
    let (time_condition, time_params) = time_range.to_sql_condition();
    let stocks_filter = match direction {
        "bullish" => "trade_type = 'BUY' AND exit_price < entry_price",
        "bearish" => "trade_type = 'SELL' AND exit_price > entry_price",
        _ => return Ok(0.0),
    };

    let sql = format!(
        r#"
        SELECT AVG(JULIANDAY(exit_date) - JULIANDAY(entry_date)) as avg_hold_time
        FROM stocks
        WHERE {} AND exit_date IS NOT NULL AND ({})
        "#,
        stocks_filter, time_condition
    );

    let mut query_params = Vec::new();
    for param in &time_params {
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

/// Calculate consecutive streaks for a specific trade direction
async fn calculate_direction_consecutive_streaks(
    conn: &Connection,
    direction: &str,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<(u32, u32)> {
    let (stocks_filter, options_filter) = match direction {
        "bullish" => ("trade_type = 'BUY'", "option_type = 'CALL'"),
        "bearish" => ("trade_type = 'SELL'", "option_type = 'PUT'"),
        _ => return Ok((0, 0)),
    };

    let mut trades = Vec::new();

    // Get stocks trades
    let stocks_sql = format!(
        r#"
        SELECT 
            CASE 
                WHEN trade_type = 'BUY' THEN (exit_price - entry_price) * number_shares - commissions
                WHEN trade_type = 'SELL' THEN (entry_price - exit_price) * number_shares - commissions
                ELSE 0
            END as calculated_pnl
        FROM stocks
        WHERE {} AND exit_price IS NOT NULL AND exit_date IS NOT NULL AND ({})
        ORDER BY exit_date ASC
        "#,
        stocks_filter, time_condition
    );

    let mut query_params = Vec::new();
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

    // Get options trades
    let options_sql = format!(
        r#"
        SELECT 
            CASE 
                WHEN exit_price IS NOT NULL THEN 
                    (exit_price - entry_price) * total_quantity * 100 - commissions
                ELSE 0
            END as calculated_pnl
        FROM options
        WHERE {} AND status = 'closed' AND ({})
        ORDER BY exit_date ASC
        "#,
        options_filter, time_condition
    );

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
