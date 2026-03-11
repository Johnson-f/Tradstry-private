// Shows portfolio and performance trends as they evolve, answering:
// Is profitability improving or declining?
// Which days/months perform best?
// Are you controlling risk over time?

use crate::models::analytics::{AnalyticsOptions, TimeSeriesData, TimeSeriesPoint};
use crate::models::stock::stocks::TimeRange;
use anyhow::Result;
use libsql::Connection;
use std::collections::HashMap;

/// Calculate time series data for equity curves and rolling metrics
pub async fn calculate_time_series_data(
    conn: &Connection,
    time_range: &TimeRange,
    options: &AnalyticsOptions,
) -> Result<TimeSeriesData> {
    let (time_condition, time_params) = time_range.to_sql_condition();

    // Calculate daily PnL time series
    let daily_pnl = calculate_daily_pnl_series(conn, &time_condition, &time_params).await?;

    // Calculate weekly PnL time series
    let weekly_pnl = calculate_weekly_pnl_series(conn, &time_condition, &time_params).await?;

    // Calculate monthly PnL time series
    let monthly_pnl = calculate_monthly_pnl_series(conn, &time_condition, &time_params).await?;

    // Calculate rolling win rates
    let rolling_win_rate_20 =
        calculate_rolling_win_rate(conn, &time_condition, &time_params, 20).await?;
    let rolling_win_rate_50 =
        calculate_rolling_win_rate(conn, &time_condition, &time_params, 50).await?;
    let rolling_win_rate_100 =
        calculate_rolling_win_rate(conn, &time_condition, &time_params, 100).await?;

    // Calculate rolling Sharpe ratios
    let rolling_sharpe_20 = calculate_rolling_sharpe_ratio(
        conn,
        &time_condition,
        &time_params,
        20,
        options.risk_free_rate,
    )
    .await?;
    let rolling_sharpe_50 = calculate_rolling_sharpe_ratio(
        conn,
        &time_condition,
        &time_params,
        50,
        options.risk_free_rate,
    )
    .await?;

    // Calculate calendar-based analytics
    let profit_by_day_of_week =
        calculate_profit_by_day_of_week(conn, &time_condition, &time_params).await?;
    let profit_by_month = calculate_profit_by_month(conn, &time_condition, &time_params).await?;

    // Calculate drawdown curve
    let drawdown_curve = calculate_drawdown_curve(&daily_pnl).await?;

    // Calculate cumulative metrics
    let cumulative_return = daily_pnl
        .iter()
        .map(|p| p.cumulative_value)
        .next_back()
        .unwrap_or(0.0);
    let total_return_percentage = if cumulative_return != 0.0 {
        (cumulative_return / daily_pnl.first().map(|p| p.cumulative_value).unwrap_or(1.0)) * 100.0
    } else {
        0.0
    };

    // Calculate annualized return (assuming 365 trading days)
    let annualized_return = if daily_pnl.len() > 1 {
        let days = daily_pnl.len() as f64;
        let total_return = cumulative_return;
        ((1.0 + total_return).powf(365.0 / days) - 1.0) * 100.0
    } else {
        0.0
    };

    // Calculate total trades
    let total_trades = daily_pnl.iter().map(|p| p.trade_count).sum();

    Ok(TimeSeriesData {
        daily_pnl,
        weekly_pnl,
        monthly_pnl,
        rolling_win_rate_20,
        rolling_win_rate_50,
        rolling_win_rate_100,
        rolling_sharpe_ratio_20: rolling_sharpe_20,
        rolling_sharpe_ratio_50: rolling_sharpe_50,
        profit_by_day_of_week,
        profit_by_month,
        drawdown_curve,
        cumulative_return,
        annualized_return,
        total_return_percentage,
        total_trades,
    })
}

/// Calculate daily PnL time series
/// Calculate daily PnL time series
async fn calculate_daily_pnl_series(
    conn: &Connection,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<Vec<TimeSeriesPoint>> {
    let sql = format!(
        r#"
        SELECT
            DATE(exit_date) as trade_date,
            SUM(calculated_pnl) as daily_pnl,
            COUNT(*) as trade_count
        FROM (
            SELECT
                exit_date,
                CASE
                    WHEN trade_type = 'BUY' THEN (exit_price - entry_price) * number_shares - commissions
                    WHEN trade_type = 'SELL' THEN (entry_price - exit_price) * number_shares - commissions
                    ELSE 0
                END as calculated_pnl
            FROM stocks
            WHERE exit_price IS NOT NULL AND exit_date IS NOT NULL AND ({})

            UNION ALL

            SELECT
                exit_date,
                CASE
                    WHEN exit_price IS NOT NULL THEN
                        (exit_price - entry_price) * total_quantity * 100 - commissions
                    ELSE 0
                END as calculated_pnl
            FROM options
            WHERE status = 'closed' AND exit_price IS NOT NULL AND ({})
        )
        GROUP BY DATE(exit_date)
        ORDER BY trade_date
        "#,
        time_condition, time_condition
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

    let mut time_series = Vec::new();
    let mut cumulative_value = 0.0;

    while let Some(row) = rows.next().await? {
        let date = row.get::<String>(0).unwrap_or_default();

        // Safely handle the daily_pnl conversion
        let daily_pnl = match row.get::<libsql::Value>(1) {
            Ok(libsql::Value::Null) => 0.0,
            Ok(libsql::Value::Real(val)) => val,
            Ok(libsql::Value::Integer(val)) => val as f64,
            Ok(libsql::Value::Text(_)) => 0.0, // Unexpected but handle gracefully
            Err(_) | Ok(_) => 0.0,
        };

        // Safely handle the trade_count conversion
        let trade_count = match row.get::<libsql::Value>(2) {
            Ok(libsql::Value::Null) => 0,
            Ok(libsql::Value::Integer(val)) => val as u32,
            Ok(libsql::Value::Real(val)) => val as u32,
            Ok(libsql::Value::Text(_)) => 0,
            Err(_) | Ok(_) => 0,
        };

        cumulative_value += daily_pnl;

        time_series.push(TimeSeriesPoint {
            date,
            value: daily_pnl,
            cumulative_value,
            trade_count,
        });
    }

    Ok(time_series)
}

/// Calculate weekly PnL time series
async fn calculate_weekly_pnl_series(
    conn: &Connection,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<Vec<TimeSeriesPoint>> {
    let sql = format!(
        r#"
        SELECT
            strftime('%Y-W%W', exit_date) as week,
            SUM(calculated_pnl) as weekly_pnl,
            COUNT(*) as trade_count
        FROM (
            SELECT
                exit_date,
                CASE
                    WHEN trade_type = 'BUY' THEN (exit_price - entry_price) * number_shares - commissions
                    WHEN trade_type = 'SELL' THEN (entry_price - exit_price) * number_shares - commissions
                    ELSE 0
                END as calculated_pnl
            FROM stocks
            WHERE exit_price IS NOT NULL AND exit_date IS NOT NULL AND ({})

            UNION ALL

            SELECT
                exit_date,
                CASE
                    WHEN exit_price IS NOT NULL THEN
                        (exit_price - entry_price) * total_quantity * 100 - commissions
                    ELSE 0
                END as calculated_pnl
            FROM options
            WHERE status = 'closed' AND exit_price IS NOT NULL AND ({})
        )
        GROUP BY strftime('%Y-W%W', exit_date)
        ORDER BY week
        "#,
        time_condition, time_condition
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

    let mut time_series = Vec::new();
    let mut cumulative_value = 0.0;

    while let Some(row) = rows.next().await? {
        let week = row.get::<String>(0).unwrap_or_default();

        // Safely handle the weekly_pnl conversion
        let weekly_pnl = match row.get::<libsql::Value>(1) {
            Ok(libsql::Value::Null) => 0.0,
            Ok(libsql::Value::Real(val)) => val,
            Ok(libsql::Value::Integer(val)) => val as f64,
            Ok(libsql::Value::Text(_)) => 0.0,
            Err(_) | Ok(_) => 0.0,
        };

        // Safely handle the trade_count conversion
        let trade_count = match row.get::<libsql::Value>(2) {
            Ok(libsql::Value::Null) => 0,
            Ok(libsql::Value::Integer(val)) => val as u32,
            Ok(libsql::Value::Real(val)) => val as u32,
            Ok(libsql::Value::Text(_)) => 0,
            Err(_) | Ok(_) => 0,
        };

        cumulative_value += weekly_pnl;

        time_series.push(TimeSeriesPoint {
            date: week,
            value: weekly_pnl,
            cumulative_value,
            trade_count,
        });
    }

    Ok(time_series)
}

/// Calculate monthly PnL time series
async fn calculate_monthly_pnl_series(
    conn: &Connection,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<Vec<TimeSeriesPoint>> {
    let sql = format!(
        r#"
        SELECT
            strftime('%Y-%m', exit_date) as month,
            SUM(calculated_pnl) as monthly_pnl,
            COUNT(*) as trade_count
        FROM (
            SELECT
                exit_date,
                CASE
                    WHEN trade_type = 'BUY' THEN (exit_price - entry_price) * number_shares - commissions
                    WHEN trade_type = 'SELL' THEN (entry_price - exit_price) * number_shares - commissions
                    ELSE 0
                END as calculated_pnl
            FROM stocks
            WHERE exit_price IS NOT NULL AND exit_date IS NOT NULL AND ({})

            UNION ALL

            SELECT
                exit_date,
                CASE
                    WHEN exit_price IS NOT NULL THEN
                        (exit_price - entry_price) * total_quantity * 100 - commissions
                    ELSE 0
                END as calculated_pnl
            FROM options
            WHERE status = 'closed' AND exit_price IS NOT NULL AND ({})
        )
        GROUP BY strftime('%Y-%m', exit_date)
        ORDER BY month
        "#,
        time_condition, time_condition
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

    let mut time_series = Vec::new();
    let mut cumulative_value = 0.0;

    while let Some(row) = rows.next().await? {
        let month = row.get::<String>(0).unwrap_or_default();

        // Safely handle the monthly_pnl conversion
        let monthly_pnl = match row.get::<libsql::Value>(1) {
            Ok(libsql::Value::Null) => 0.0,
            Ok(libsql::Value::Real(val)) => val,
            Ok(libsql::Value::Integer(val)) => val as f64,
            Ok(libsql::Value::Text(_)) => 0.0,
            Err(_) | Ok(_) => 0.0,
        };

        // Safely handle the trade_count conversion
        let trade_count = match row.get::<libsql::Value>(2) {
            Ok(libsql::Value::Null) => 0,
            Ok(libsql::Value::Integer(val)) => val as u32,
            Ok(libsql::Value::Real(val)) => val as u32,
            Ok(libsql::Value::Text(_)) => 0,
            Err(_) | Ok(_) => 0,
        };

        cumulative_value += monthly_pnl;

        time_series.push(TimeSeriesPoint {
            date: month,
            value: monthly_pnl,
            cumulative_value,
            trade_count,
        });
    }

    Ok(time_series)
}

/// Calculate rolling Sharpe ratio over specified window
async fn calculate_rolling_sharpe_ratio(
    conn: &Connection,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
    window_size: u32,
    risk_free_rate: f64,
) -> Result<Vec<TimeSeriesPoint>> {
    // Get daily returns
    let sql = format!(
        r#"
        SELECT
            DATE(exit_date) as trade_date,
            SUM(calculated_pnl) as daily_return,
            COUNT(*) as trade_count
        FROM (
            SELECT
                exit_date,
                CASE
                    WHEN trade_type = 'BUY' THEN (exit_price - entry_price) * number_shares - commissions
                    WHEN trade_type = 'SELL' THEN (entry_price - exit_price) * number_shares - commissions
                    ELSE 0
                END as calculated_pnl
            FROM stocks
            WHERE exit_price IS NOT NULL AND exit_date IS NOT NULL AND ({})

            UNION ALL

            SELECT
                exit_date,
                CASE
                    WHEN exit_price IS NOT NULL THEN
                        (exit_price - entry_price) * total_quantity * 100 - commissions
                    ELSE 0
                END as calculated_pnl
            FROM options
            WHERE status = 'closed' AND exit_price IS NOT NULL AND ({})
        )
        GROUP BY DATE(exit_date)
        ORDER BY trade_date
        "#,
        time_condition, time_condition
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

    let mut daily_returns = Vec::new();
    while let Some(row) = rows.next().await? {
        let date = row.get::<String>(0).unwrap_or_default();

        // Safely handle the daily_return conversion
        let daily_return = match row.get::<libsql::Value>(1) {
            Ok(libsql::Value::Null) => 0.0,
            Ok(libsql::Value::Real(val)) => val,
            Ok(libsql::Value::Integer(val)) => val as f64,
            Ok(libsql::Value::Text(_)) => 0.0,
            Err(_) | Ok(_) => 0.0,
        };

        // Safely handle the trade_count conversion
        let trade_count = match row.get::<libsql::Value>(2) {
            Ok(libsql::Value::Null) => 0,
            Ok(libsql::Value::Integer(val)) => val,
            Ok(libsql::Value::Real(val)) => val as i64,
            Ok(libsql::Value::Text(_)) => 0,
            Err(_) | Ok(_) => 0,
        };

        daily_returns.push((date, daily_return, trade_count));
    }

    // Calculate rolling Sharpe ratio
    let mut rolling_sharpe = Vec::new();
    for i in 0..daily_returns.len() {
        let start_idx = i.saturating_sub(window_size as usize);
        let end_idx = i + 1;

        if start_idx >= end_idx {
            // Not enough data yet
            rolling_sharpe.push(TimeSeriesPoint {
                date: daily_returns[i].0.clone(),
                value: 0.0,
                cumulative_value: 0.0,
                trade_count: 0,
            });
            continue;
        }

        let window_returns: Vec<f64> = daily_returns[start_idx..end_idx]
            .iter()
            .map(|(_, ret, _)| *ret)
            .collect();

        let sharpe = calculate_sharpe_ratio(&window_returns, risk_free_rate);
        let trade_count: u32 = daily_returns[start_idx..end_idx]
            .iter()
            .map(|(_, _, count)| *count as u32)
            .sum();

        rolling_sharpe.push(TimeSeriesPoint {
            date: daily_returns[i].0.clone(),
            value: sharpe,
            cumulative_value: sharpe,
            trade_count,
        });
    }

    Ok(rolling_sharpe)
}

/// Calculate Sharpe ratio for a series of returns
fn calculate_sharpe_ratio(returns: &[f64], risk_free_rate: f64) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }

    // Calculate mean return
    let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;

    // Calculate standard deviation
    let variance = returns
        .iter()
        .map(|r| {
            let diff = r - mean_return;
            diff * diff
        })
        .sum::<f64>()
        / returns.len() as f64;

    let std_dev = variance.sqrt();

    // Annualize assuming 252 trading days
    let annualized_return = mean_return * 252.0;
    let annualized_std = std_dev * (252.0_f64).sqrt();

    // Calculate Sharpe ratio
    if annualized_std > 0.0 {
        (annualized_return - risk_free_rate) / annualized_std
    } else {
        0.0
    }
}

/// Calculate rolling win rate over specified window
async fn calculate_rolling_win_rate(
    conn: &Connection,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
    window_size: u32,
) -> Result<Vec<TimeSeriesPoint>> {
    let sql = format!(
        r#"
        SELECT
            DATE(exit_date) as trade_date,
            SUM(CASE WHEN calculated_pnl > 0 THEN 1 ELSE 0 END) as wins,
            COUNT(*) as total_trades
        FROM (
            SELECT
                exit_date,
                CASE
                    WHEN trade_type = 'BUY' THEN (exit_price - entry_price) * number_shares - commissions
                    WHEN trade_type = 'SELL' THEN (entry_price - exit_price) * number_shares - commissions
                    ELSE 0
                END as calculated_pnl
            FROM stocks
            WHERE exit_price IS NOT NULL AND exit_date IS NOT NULL AND ({})

            UNION ALL

            SELECT
                exit_date,
                CASE
                    WHEN exit_price IS NOT NULL THEN
                        (exit_price - entry_price) * total_quantity * 100 - commissions
                    ELSE 0
                END as calculated_pnl
            FROM options
            WHERE status = 'closed' AND exit_price IS NOT NULL AND ({})
        )
        GROUP BY DATE(exit_date)
        ORDER BY trade_date
        "#,
        time_condition, time_condition
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

    let mut daily_data = Vec::new();
    while let Some(row) = rows.next().await? {
        let date = row.get::<String>(0).unwrap_or_default();

        // Safely handle the wins conversion
        let wins = match row.get::<libsql::Value>(1) {
            Ok(libsql::Value::Null) => 0,
            Ok(libsql::Value::Integer(val)) => val as u32,
            Ok(libsql::Value::Real(val)) => val as u32,
            Ok(libsql::Value::Text(_)) => 0,
            Err(_) | Ok(_) => 0,
        };

        // Safely handle the total_trades conversion
        let total_trades = match row.get::<libsql::Value>(2) {
            Ok(libsql::Value::Null) => 0,
            Ok(libsql::Value::Integer(val)) => val as u32,
            Ok(libsql::Value::Real(val)) => val as u32,
            Ok(libsql::Value::Text(_)) => 0,
            Err(_) | Ok(_) => 0,
        };

        daily_data.push((date, wins, total_trades));
    }

    // Calculate rolling win rate in Rust
    let mut rolling_win_rate = Vec::new();
    for i in 0..daily_data.len() {
        let start_idx = i.saturating_sub(window_size as usize);
        let end_idx = i + 1;

        let window_data = &daily_data[start_idx..end_idx];
        let total_wins: u32 = window_data.iter().map(|(_, wins, _)| wins).sum();
        let total_trades: u32 = window_data.iter().map(|(_, _, total)| total).sum();

        let win_rate = if total_trades > 0 {
            (total_wins as f64 / total_trades as f64) * 100.0
        } else {
            0.0
        };

        rolling_win_rate.push(TimeSeriesPoint {
            date: daily_data[i].0.clone(),
            value: win_rate,
            cumulative_value: win_rate,
            trade_count: window_data.iter().map(|(_, _, total)| total).sum(),
        });
    }

    Ok(rolling_win_rate)
}

/// Calculate profit by day of week
async fn calculate_profit_by_day_of_week(
    conn: &Connection,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<HashMap<String, f64>> {
    let sql = format!(
        r#"
        SELECT
            strftime('%w', exit_date) as day_of_week,
            SUM(calculated_pnl) as daily_profit
        FROM (
            SELECT
                exit_date,
                CASE
                    WHEN trade_type = 'BUY' THEN (exit_price - entry_price) * number_shares - commissions
                    WHEN trade_type = 'SELL' THEN (entry_price - exit_price) * number_shares - commissions
                    ELSE 0
                END as calculated_pnl
            FROM stocks
            WHERE exit_price IS NOT NULL AND exit_date IS NOT NULL AND ({})

            UNION ALL

            SELECT
                exit_date,
                CASE
                    WHEN exit_price IS NOT NULL THEN
                        (exit_price - entry_price) * total_quantity * 100 - commissions
                    ELSE 0
                END as calculated_pnl
            FROM options
            WHERE status = 'closed' AND exit_price IS NOT NULL AND ({})
        )
        GROUP BY strftime('%w', exit_date)
        "#,
        time_condition, time_condition
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

    let mut profit_by_day = HashMap::new();
    let day_names = [
        "Sunday",
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
    ];

    while let Some(row) = rows.next().await? {
        let day_num = row.get::<String>(0).unwrap_or_default();

        // Safely handle the profit conversion
        let profit = match row.get::<libsql::Value>(1) {
            Ok(libsql::Value::Null) => 0.0,
            Ok(libsql::Value::Real(val)) => val,
            Ok(libsql::Value::Integer(val)) => val as f64,
            Ok(libsql::Value::Text(_)) => 0.0,
            Err(_) | Ok(_) => 0.0,
        };

        if let Ok(day_index) = day_num.parse::<usize>()
            && day_index < day_names.len()
        {
            profit_by_day.insert(day_names[day_index].to_string(), profit);
        }
    }

    Ok(profit_by_day)
}

/// Calculate profit by month
async fn calculate_profit_by_month(
    conn: &Connection,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<HashMap<String, f64>> {
    let sql = format!(
        r#"
        SELECT
            strftime('%m', exit_date) as month,
            SUM(calculated_pnl) as monthly_profit
        FROM (
            SELECT
                exit_date,
                CASE
                    WHEN trade_type = 'BUY' THEN (exit_price - entry_price) * number_shares - commissions
                    WHEN trade_type = 'SELL' THEN (entry_price - exit_price) * number_shares - commissions
                    ELSE 0
                END as calculated_pnl
            FROM stocks
            WHERE exit_price IS NOT NULL AND exit_date IS NOT NULL AND ({})

            UNION ALL

            SELECT
                exit_date,
                CASE
                    WHEN exit_price IS NOT NULL THEN
                        (exit_price - entry_price) * total_quantity * 100 - commissions
                    ELSE 0
                END as calculated_pnl
            FROM options
            WHERE status = 'closed' AND exit_price IS NOT NULL AND ({})
        )
        GROUP BY strftime('%m', exit_date)
        "#,
        time_condition, time_condition
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

    let mut profit_by_month = HashMap::new();
    let month_names = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];

    while let Some(row) = rows.next().await? {
        let month_num = row.get::<String>(0).unwrap_or_default();

        // Safely handle the profit conversion
        let profit = match row.get::<libsql::Value>(1) {
            Ok(libsql::Value::Null) => 0.0,
            Ok(libsql::Value::Real(val)) => val,
            Ok(libsql::Value::Integer(val)) => val as f64,
            Ok(libsql::Value::Text(_)) => 0.0,
            Err(_) | Ok(_) => 0.0,
        };

        if let Ok(month_index) = month_num.parse::<usize>()
            && (1..=12).contains(&month_index)
        {
            profit_by_month.insert(month_names[month_index - 1].to_string(), profit);
        }
    }

    Ok(profit_by_month)
}

/// Calculate drawdown curve from daily PnL
async fn calculate_drawdown_curve(daily_pnl: &[TimeSeriesPoint]) -> Result<Vec<TimeSeriesPoint>> {
    let mut drawdown_curve = Vec::new();
    let mut peak: f64 = 0.0;

    for point in daily_pnl {
        peak = peak.max(point.cumulative_value);
        let drawdown = peak - point.cumulative_value;

        drawdown_curve.push(TimeSeriesPoint {
            date: point.date.clone(),
            value: drawdown,
            cumulative_value: drawdown,
            trade_count: point.trade_count,
        });
    }

    Ok(drawdown_curve)
}

impl Default for TimeSeriesData {
    fn default() -> Self {
        Self {
            daily_pnl: Vec::new(),
            weekly_pnl: Vec::new(),
            monthly_pnl: Vec::new(),
            rolling_win_rate_20: Vec::new(),
            rolling_win_rate_50: Vec::new(),
            rolling_win_rate_100: Vec::new(),
            rolling_sharpe_ratio_20: Vec::new(),
            rolling_sharpe_ratio_50: Vec::new(),
            profit_by_day_of_week: HashMap::new(),
            profit_by_month: HashMap::new(),
            drawdown_curve: Vec::new(),
            cumulative_return: 0.0,
            annualized_return: 0.0,
            total_return_percentage: 0.0,
            total_trades: 0,
        }
    }
}
