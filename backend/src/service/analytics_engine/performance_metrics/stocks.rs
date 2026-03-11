use super::utils::get_f64_value;
use crate::models::analytics::PerformanceMetrics;
use anyhow::Result;
use libsql::Connection;

/// Calculate performance metrics for stocks table
pub async fn calculate_stocks_performance_metrics(
    conn: &Connection,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<PerformanceMetrics> {
    // Main performance metrics query
    let sql = format!(
        r#"
        SELECT
            AVG(JULIANDAY(exit_date) - JULIANDAY(entry_date)) as avg_hold_time_days,
            AVG(number_shares * entry_price) as avg_position_size,
            STDDEV(number_shares * entry_price) as position_size_std_dev,
            AVG(commissions) as avg_commission_per_trade,
            SUM(commissions) / NULLIF(SUM(ABS(calculated_pnl)), 0) * 100 as commission_impact_percentage
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
        .query(libsql::params_from_iter(query_params.clone()))
        .await?;

    let mut avg_hold_time_days = 0.0;
    let mut avg_position_size = 0.0;
    let mut position_size_std_dev = 0.0;
    let mut _avg_commission_per_trade = 0.0;
    let mut commission_impact_percentage = 0.0;

    if let Some(row) = rows.next().await? {
        avg_hold_time_days = get_f64_value(&row, 0);
        avg_position_size = get_f64_value(&row, 1);
        position_size_std_dev = get_f64_value(&row, 2);
        _avg_commission_per_trade = get_f64_value(&row, 3);
        commission_impact_percentage = get_f64_value(&row, 4);
    }

    // Calculate hold times for winners and losers separately
    let winners_hold_time = calculate_winners_hold_time(conn, time_condition, time_params).await?;
    let losers_hold_time = calculate_losers_hold_time(conn, time_condition, time_params).await?;

    // Calculate advanced metrics
    let (trade_expectancy, edge, payoff_ratio) =
        calculate_expectancy_and_edge_stocks(conn, time_condition, time_params).await?;
    let kelly_criterion =
        calculate_kelly_criterion_stocks(conn, time_condition, time_params).await?;
    let (avg_r_multiple, r_multiple_std_dev, positive_r_count, negative_r_count) =
        calculate_r_multiples_stocks(conn, time_condition, time_params).await?;
    let consistency_ratio =
        calculate_consistency_ratio_stocks(conn, time_condition, time_params).await?;
    let (monthly_win_rate, quarterly_win_rate) =
        calculate_periodic_win_rates_stocks(conn, time_condition, time_params).await?;
    let system_quality_number =
        calculate_system_quality_number_stocks(conn, time_condition, time_params).await?;

    Ok(PerformanceMetrics {
        trade_expectancy,
        edge,
        average_hold_time_days: avg_hold_time_days,
        average_hold_time_winners_days: winners_hold_time,
        average_hold_time_losers_days: losers_hold_time,
        average_position_size: avg_position_size,
        position_size_standard_deviation: position_size_std_dev,
        position_size_variability: if avg_position_size > 0.0 {
            position_size_std_dev / avg_position_size
        } else {
            0.0
        },
        kelly_criterion,
        system_quality_number,
        payoff_ratio,
        average_r_multiple: avg_r_multiple,
        r_multiple_standard_deviation: r_multiple_std_dev,
        positive_r_multiple_count: positive_r_count,
        negative_r_multiple_count: negative_r_count,
        consistency_ratio,
        monthly_win_rate,
        quarterly_win_rate,
        average_slippage: 0.0, // Not available in current schema
        commission_impact_percentage,
    })
}

/// Calculate average hold time for winning trades
async fn calculate_winners_hold_time(
    conn: &Connection,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<f64> {
    let sql = format!(
        r#"
        SELECT AVG(JULIANDAY(exit_date) - JULIANDAY(entry_date)) as avg_hold_time_winners
        FROM stocks
        WHERE exit_price IS NOT NULL AND exit_date IS NOT NULL AND ({})
          AND ((trade_type = 'BUY' AND exit_price > entry_price)
               OR (trade_type = 'SELL' AND exit_price < entry_price))
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
        Ok(get_f64_value(&row, 0))
    } else {
        Ok(0.0)
    }
}

/// Calculate average hold time for losing trades
async fn calculate_losers_hold_time(
    conn: &Connection,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<f64> {
    let sql = format!(
        r#"
        SELECT AVG(JULIANDAY(exit_date) - JULIANDAY(entry_date)) as avg_hold_time_losers
        FROM stocks
        WHERE exit_price IS NOT NULL AND exit_date IS NOT NULL AND ({})
          AND ((trade_type = 'BUY' AND exit_price < entry_price)
               OR (trade_type = 'SELL' AND exit_price > entry_price))
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
        Ok(get_f64_value(&row, 0))
    } else {
        Ok(0.0)
    }
}

/// Calculate average risk per trade (entry - stop_loss) * position_size
#[allow(dead_code)]
pub async fn calculate_average_risk_per_trade(
    conn: &Connection,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<f64> {
    let sql = format!(
        r#"
        SELECT AVG(ABS(entry_price - stop_loss) * number_shares) as avg_risk_per_trade
        FROM stocks
        WHERE stop_loss IS NOT NULL AND ({})
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
        Ok(get_f64_value(&row, 0))
    } else {
        Ok(0.0)
    }
}

/// Calculate expectancy, edge, and payoff ratio for stocks
async fn calculate_expectancy_and_edge_stocks(
    conn: &Connection,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<(f64, f64, f64)> {
    let sql = format!(
        r#"
        SELECT
            COUNT(*) as total_trades,
            SUM(CASE WHEN calculated_pnl > 0 THEN 1 ELSE 0 END) as winning_trades,
            SUM(CASE WHEN calculated_pnl < 0 THEN 1 ELSE 0 END) as losing_trades,
            AVG(CASE WHEN calculated_pnl > 0 THEN calculated_pnl ELSE NULL END) as avg_winner,
            AVG(CASE WHEN calculated_pnl < 0 THEN calculated_pnl ELSE NULL END) as avg_loser,
            SUM(calculated_pnl) as total_pnl
        FROM (
            SELECT
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

    let mut total_trades = 0.0;
    let mut winning_trades = 0.0;
    let mut losing_trades = 0.0;
    let mut avg_winner = 0.0;
    let mut avg_loser = 0.0;

    if let Some(row) = rows.next().await? {
        total_trades = get_f64_value(&row, 0);
        winning_trades = get_f64_value(&row, 1);
        losing_trades = get_f64_value(&row, 2);
        avg_winner = get_f64_value(&row, 3);
        avg_loser = get_f64_value(&row, 4);
    }

    // Calculate expectancy = (win rate * avg winner) + (loss rate * avg loser)
    let win_rate = if total_trades > 0.0 {
        winning_trades / total_trades
    } else {
        0.0
    };
    let loss_rate = if total_trades > 0.0 {
        losing_trades / total_trades
    } else {
        0.0
    };
    let expectancy = (win_rate * avg_winner) + (loss_rate * avg_loser);

    // Calculate edge (similar to expectancy, percentage form)
    let edge = if avg_loser != 0.0 {
        ((win_rate * avg_winner) + (loss_rate * avg_loser)) / avg_loser.abs()
    } else {
        0.0
    };

    // Calculate payoff ratio (avg winner / avg loser)
    let payoff_ratio = if avg_loser != 0.0 {
        avg_winner / avg_loser.abs()
    } else {
        0.0
    };

    Ok((expectancy, edge, payoff_ratio))
}

/// Calculate Kelly Criterion for stocks
/// Kelly % = W - [(1-W) / R]
/// Where W = win rate, R = avg winner / avg loser
async fn calculate_kelly_criterion_stocks(
    conn: &Connection,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<f64> {
    let sql = format!(
        r#"
        SELECT
            AVG(CASE WHEN calculated_pnl > 0 THEN calculated_pnl ELSE NULL END) as avg_winner,
            AVG(CASE WHEN calculated_pnl < 0 THEN ABS(calculated_pnl) ELSE NULL END) as avg_loser,
            CAST(SUM(CASE WHEN calculated_pnl > 0 THEN 1 ELSE 0 END) AS REAL) / COUNT(*) as win_rate
        FROM (
            SELECT
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

    let mut avg_winner = 0.0;
    let mut avg_loser = 0.0;
    let mut win_rate = 0.0;

    if let Some(row) = rows.next().await? {
        avg_winner = get_f64_value(&row, 0);
        avg_loser = get_f64_value(&row, 1);
        win_rate = get_f64_value(&row, 2);
    }

    // Kelly = W - [(1-W) / R] where R = avg_winner / avg_loser
    let kelly = if avg_loser > 0.0 {
        let r = avg_winner / avg_loser;
        win_rate - ((1.0 - win_rate) / r)
    } else {
        0.0
    };

    Ok(kelly)
}

/// Calculate R-Multiples for stocks
async fn calculate_r_multiples_stocks(
    conn: &Connection,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<(f64, f64, u32, u32)> {
    let sql = format!(
        r#"
        SELECT
            CASE
                WHEN trade_type = 'BUY' THEN (exit_price - entry_price) * number_shares - commissions
                WHEN trade_type = 'SELL' THEN (entry_price - exit_price) * number_shares - commissions
                ELSE 0
            END as pnl,
            ABS(entry_price - stop_loss) * number_shares as risk
        FROM stocks
        WHERE exit_price IS NOT NULL AND exit_date IS NOT NULL
          AND stop_loss IS NOT NULL AND ({})
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

    let mut r_multiples = Vec::new();
    let mut positive_count = 0;
    let mut negative_count = 0;

    while let Some(row) = rows.next().await? {
        let pnl = get_f64_value(&row, 0);
        let risk = get_f64_value(&row, 1);

        if risk > 0.0 {
            let r_multiple = pnl / risk;
            r_multiples.push(r_multiple);
            if r_multiple > 0.0 {
                positive_count += 1;
            } else if r_multiple < 0.0 {
                negative_count += 1;
            }
        }
    }

    let avg_r_multiple = if !r_multiples.is_empty() {
        r_multiples.iter().sum::<f64>() / r_multiples.len() as f64
    } else {
        0.0
    };

    let variance = if !r_multiples.is_empty() {
        let mean = avg_r_multiple;
        r_multiples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / r_multiples.len() as f64
    } else {
        0.0
    };

    let std_dev = variance.sqrt();

    Ok((
        avg_r_multiple,
        std_dev,
        positive_count as u32,
        negative_count as u32,
    ))
}

/// Calculate consistency ratio for stocks
/// Ratio = Winning trades / Total trades - (Absolute sum of losses / Absolute sum of wins)
async fn calculate_consistency_ratio_stocks(
    conn: &Connection,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<f64> {
    let sql = format!(
        r#"
        SELECT
            CAST(SUM(CASE WHEN calculated_pnl > 0 THEN 1 ELSE 0 END) AS REAL) / COUNT(*) as win_rate,
            SUM(CASE WHEN calculated_pnl > 0 THEN calculated_pnl ELSE 0 END) as total_wins,
            SUM(CASE WHEN calculated_pnl < 0 THEN ABS(calculated_pnl) ELSE 0 END) as total_losses
        FROM (
            SELECT
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

    let mut win_rate = 0.0;
    let mut total_wins = 0.0;
    let mut total_losses = 0.0;

    if let Some(row) = rows.next().await? {
        win_rate = get_f64_value(&row, 0);
        total_wins = get_f64_value(&row, 1);
        total_losses = get_f64_value(&row, 2);
    }

    let consistency_ratio = if total_wins > 0.0 && total_losses > 0.0 {
        win_rate * (1.0 - (total_losses / total_wins))
    } else {
        0.0
    };

    Ok(consistency_ratio)
}

/// Calculate monthly and quarterly win rates for stocks
async fn calculate_periodic_win_rates_stocks(
    conn: &Connection,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<(f64, f64)> {
    // Monthly win rate
    let sql = format!(
        r#"
        SELECT
            CAST(SUM(CASE WHEN calculated_pnl > 0 THEN 1 ELSE 0 END) AS REAL) / COUNT(*) as win_rate
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
            AND JULIANDAY('now') - JULIANDAY(exit_date) <= 30
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
        .query(libsql::params_from_iter(query_params.clone()))
        .await?;

    let mut monthly_win_rate = 0.0;
    if let Some(row) = rows.next().await? {
        monthly_win_rate = get_f64_value(&row, 0);
    }

    // Quarterly win rate
    let sql = format!(
        r#"
        SELECT
            CAST(SUM(CASE WHEN calculated_pnl > 0 THEN 1 ELSE 0 END) AS REAL) / COUNT(*) as win_rate
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
            AND JULIANDAY('now') - JULIANDAY(exit_date) <= 90
        )
        "#,
        time_condition
    );

    let mut rows = conn
        .prepare(&sql)
        .await?
        .query(libsql::params_from_iter(query_params))
        .await?;

    let mut quarterly_win_rate = 0.0;
    if let Some(row) = rows.next().await? {
        quarterly_win_rate = get_f64_value(&row, 0);
    }

    Ok((monthly_win_rate, quarterly_win_rate))
}

/// Calculate System Quality Number (SQN) for stocks
/// SQN = (Expectancy / StdDev of R-Multiples) * sqrt(Number of Trades)
async fn calculate_system_quality_number_stocks(
    conn: &Connection,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<f64> {
    let (expectancy, _, _, _) =
        calculate_r_multiples_stocks(conn, time_condition, time_params).await?;
    let (total_trades, _, _) = get_basic_stats_stocks(conn, time_condition, time_params).await?;

    if total_trades > 0.0 && expectancy > 0.0 {
        Ok(expectancy * total_trades.sqrt())
    } else {
        Ok(0.0)
    }
}

/// Helper function to get basic stats
async fn get_basic_stats_stocks(
    conn: &Connection,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<(f64, f64, f64)> {
    let sql = format!(
        r#"
        SELECT
            COUNT(*) as total_trades,
            SUM(calculated_pnl) as total_pnl,
            AVG(calculated_pnl) as avg_pnl
        FROM (
            SELECT
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

    let mut total_trades = 0.0;
    let mut total_pnl = 0.0;
    let mut avg_pnl = 0.0;

    if let Some(row) = rows.next().await? {
        total_trades = get_f64_value(&row, 0);
        total_pnl = get_f64_value(&row, 1);
        avg_pnl = get_f64_value(&row, 2);
    }

    Ok((total_trades, total_pnl, avg_pnl))
}
