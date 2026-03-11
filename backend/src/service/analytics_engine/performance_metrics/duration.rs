use super::utils::{get_f64_value, get_i64_value};
use crate::models::analytics::CoreMetrics;
use crate::models::stock::stocks::TimeRange;
use anyhow::Result;
use libsql::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurationPerformanceMetrics {
    pub duration_bucket: String,
    pub trade_count: u32,
    pub win_rate: f64,
    pub total_pnl: f64,
    pub avg_pnl: f64,
    pub avg_hold_time_days: f64,
    pub best_trade: f64,
    pub worst_trade: f64,
    pub profit_factor: f64,
    pub winning_trades: u32,
    pub losing_trades: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurationPerformanceResponse {
    pub duration_buckets: Vec<DurationPerformanceMetrics>,
    pub overall_metrics: CoreMetrics,
}

/// Calculate performance metrics grouped by trade duration
pub async fn calculate_duration_performance_metrics(
    conn: &Connection,
    time_range: &TimeRange,
) -> Result<DurationPerformanceResponse> {
    let (time_condition, time_params) = time_range.to_sql_condition();

    // Define duration buckets (in days)
    let duration_buckets = vec![
        ("0-1 days", 0.0, 1.0),
        ("1-7 days", 1.0, 7.0),
        ("1-4 weeks", 7.0, 28.0),
        ("1-3 months", 28.0, 90.0),
        ("3+ months", 90.0, f64::MAX),
    ];

    let mut bucket_metrics = Vec::new();

    for (bucket_name, min_days, max_days) in duration_buckets {
        let metrics = calculate_bucket_metrics(
            conn,
            &time_condition,
            &time_params,
            bucket_name,
            min_days,
            max_days,
        )
        .await?;

        if metrics.trade_count > 0 {
            bucket_metrics.push(metrics);
        }
    }

    // Calculate overall metrics for the time period
    let overall_metrics =
        crate::service::analytics_engine::core_metrics::calculate_core_metrics(conn, time_range)
            .await?;

    Ok(DurationPerformanceResponse {
        duration_buckets: bucket_metrics,
        overall_metrics,
    })
}

async fn calculate_bucket_metrics(
    conn: &Connection,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
    bucket_name: &str,
    min_days: f64,
    max_days: f64,
) -> Result<DurationPerformanceMetrics> {
    let duration_condition = if max_days == f64::MAX {
        format!(
            "AND (JULIANDAY(exit_date) - JULIANDAY(entry_date)) >= {}",
            min_days
        )
    } else {
        format!(
            "AND (JULIANDAY(exit_date) - JULIANDAY(entry_date)) >= {} AND (JULIANDAY(exit_date) - JULIANDAY(entry_date)) < {}",
            min_days, max_days
        )
    };

    // Combined query for stocks and options
    let sql = format!(
        r#"
        WITH combined_trades AS (
            -- Stock trades
            SELECT
                (CASE
                    WHEN trade_type = 'BUY' THEN (exit_price - entry_price) * number_shares - commissions
                    ELSE (entry_price - exit_price) * number_shares - commissions
                END) as net_pnl,
                (JULIANDAY(exit_date) - JULIANDAY(entry_date)) as hold_days,
                CASE
                    WHEN (trade_type = 'BUY' AND exit_price > entry_price) OR
                         (trade_type = 'SELL' AND exit_price < entry_price)
                    THEN 1 ELSE 0
                END as is_winner
            FROM stocks
            WHERE exit_price IS NOT NULL
                AND exit_date IS NOT NULL
                AND ({})
                {}

            UNION ALL

            -- Option trades
            SELECT
                (exit_price - entry_price) * total_quantity * 100 - commissions as net_pnl,
                (JULIANDAY(exit_date) - JULIANDAY(entry_date)) as hold_days,
                CASE WHEN exit_price > entry_price THEN 1 ELSE 0 END as is_winner
            FROM options
            WHERE status = 'closed'
                AND exit_date IS NOT NULL
                AND exit_price IS NOT NULL
                AND ({})
                {}
        )
        SELECT
            COUNT(*) as trade_count,
            SUM(is_winner) as winning_trades,
            COUNT(*) - SUM(is_winner) as losing_trades,
            CASE WHEN COUNT(*) > 0 THEN (SUM(is_winner) * 100.0 / COUNT(*)) ELSE 0 END as win_rate,
            SUM(net_pnl) as total_pnl,
            AVG(net_pnl) as avg_pnl,
            AVG(hold_days) as avg_hold_time_days,
            MAX(net_pnl) as best_trade,
            MIN(net_pnl) as worst_trade,
            CASE
                WHEN SUM(CASE WHEN net_pnl < 0 THEN ABS(net_pnl) ELSE 0 END) > 0
                THEN SUM(CASE WHEN net_pnl > 0 THEN net_pnl ELSE 0 END) / SUM(CASE WHEN net_pnl < 0 THEN ABS(net_pnl) ELSE 0 END)
                ELSE 0
            END as profit_factor
        FROM combined_trades
        "#,
        time_condition, duration_condition, time_condition, duration_condition
    );

    let mut query_params = Vec::new();
    for param in time_params {
        query_params.push(libsql::Value::Text(param.to_rfc3339()));
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
        Ok(DurationPerformanceMetrics {
            duration_bucket: bucket_name.to_string(),
            trade_count: get_i64_value(&row, 0) as u32,
            winning_trades: get_i64_value(&row, 1) as u32,
            losing_trades: get_i64_value(&row, 2) as u32,
            win_rate: get_f64_value(&row, 3),
            total_pnl: get_f64_value(&row, 4),
            avg_pnl: get_f64_value(&row, 5),
            avg_hold_time_days: get_f64_value(&row, 6),
            best_trade: get_f64_value(&row, 7),
            worst_trade: get_f64_value(&row, 8),
            profit_factor: get_f64_value(&row, 9),
        })
    } else {
        Ok(DurationPerformanceMetrics {
            duration_bucket: bucket_name.to_string(),
            trade_count: 0,
            winning_trades: 0,
            losing_trades: 0,
            win_rate: 0.0,
            total_pnl: 0.0,
            avg_pnl: 0.0,
            avg_hold_time_days: 0.0,
            best_trade: 0.0,
            worst_trade: 0.0,
            profit_factor: 0.0,
        })
    }
}
