use super::utils::{get_f64_value, get_i64_value};
use crate::models::stock::stocks::TimeRange;
use anyhow::Result;
use libsql::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Risk management behavioral patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskBehaviorMetrics {
    pub stop_loss_hit_percentage: f64,
    pub stop_loss_adherence_rate: f64,
    pub avg_position_size_after_win: f64,
    pub avg_position_size_after_loss: f64,
    pub position_size_consistency_score: f64,
    pub avg_risk_reward_ratio: f64,
    pub risk_reward_consistency: f64,
}

/// Timing behavioral patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingBehaviorMetrics {
    pub best_performing_day: String,
    pub worst_performing_day: String,
    pub trades_per_day_of_week: HashMap<String, u32>,
    pub pnl_per_day_of_week: HashMap<String, f64>,
    pub avg_entry_time_morning: f64,
    pub avg_entry_time_afternoon: f64,
    pub avg_entry_time_evening: f64,
    pub early_exit_percentage: f64,
    pub late_exit_percentage: f64,
}

/// Trading frequency behavioral patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingFrequencyMetrics {
    pub trades_per_week: f64,
    pub over_trading_days: u32,
    pub under_trading_days: u32,
    pub optimal_trading_frequency: f64,
    pub trading_consistency_score: f64,
    pub avg_trades_per_winning_day: f64,
    pub avg_trades_per_losing_day: f64,
}

/// Profitability distribution patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfitabilityDistributionMetrics {
    pub best_trade_pct_of_total_profit: f64,
    pub worst_trade_pct_of_total_loss: f64,
    pub profit_distribution_score: f64,
    pub outlier_trades_count: u32,
    pub largest_win_drawdown: f64,
    pub worst_trade_recovery_time: f64,
    pub consecutive_wins_avg_profit: f64,
    pub consecutive_losses_avg_loss: f64,
}

/// Comprehensive behavioral patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralPatterns {
    pub risk_behavior: RiskBehaviorMetrics,
    pub timing_behavior: TimingBehaviorMetrics,
    pub trading_frequency: TradingFrequencyMetrics,
    pub profitability_distribution: ProfitabilityDistributionMetrics,
}

/// Calculate all behavioral patterns
#[allow(dead_code)]
pub async fn calculate_behavioral_patterns(
    conn: &Connection,
    time_range: &TimeRange,
) -> Result<BehavioralPatterns> {
    let (time_condition, time_params) = time_range.to_sql_condition();

    // Calculate each pattern category
    let risk_behavior = calculate_risk_behavior(conn, &time_condition, &time_params).await?;
    let timing_behavior = calculate_timing_behavior(conn, &time_condition, &time_params).await?;
    let trading_frequency =
        calculate_trading_frequency_behavior(conn, &time_condition, &time_params).await?;
    let profitability_distribution =
        calculate_profitability_distribution(conn, &time_condition, &time_params).await?;

    Ok(BehavioralPatterns {
        risk_behavior,
        timing_behavior,
        trading_frequency,
        profitability_distribution,
    })
}

/// Calculate risk management behavioral patterns
#[allow(dead_code)]
async fn calculate_risk_behavior(
    conn: &Connection,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<RiskBehaviorMetrics> {
    // Calculate stop loss adherence - using SQLite-specific functions
    let sql = format!(
        r#"
        SELECT
            COUNT(*) as total_trades,
            SUM(CASE WHEN stop_loss IS NOT NULL THEN 1 ELSE 0 END) as trades_with_stop
        FROM stocks
        WHERE exit_price IS NOT NULL AND exit_date IS NOT NULL AND ({})
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

    let mut stop_loss_adherence = 0.0;
    let stop_loss_hit_pct = 0.0;

    if let Some(row) = rows.next().await? {
        let total = get_i64_value(&row, 0);
        let with_stop = get_i64_value(&row, 1);

        if total > 0 {
            stop_loss_adherence = (with_stop as f64 / total as f64) * 100.0;
        }
    }

    // Simplified position size patterns - using subquery instead of window functions
    let sql = format!(
        r#"
        SELECT
            AVG(position_size) as avg_after_win,
            AVG(position_size) as avg_after_loss,
            STDDEV(position_size) / NULLIF(AVG(position_size), 0) * 100 as consistency
        FROM (
            SELECT
                entry_price * number_shares as position_size,
                CASE
                    WHEN trade_type = 'BUY' THEN (exit_price - entry_price) * number_shares - commissions
                    WHEN trade_type = 'SELL' THEN (entry_price - exit_price) * number_shares - commissions
                    ELSE 0
                END as pnl
            FROM stocks
            WHERE exit_price IS NOT NULL AND exit_date IS NOT NULL AND ({})
        )
        "#,
        time_condition
    );

    let mut rows = conn
        .prepare(&sql)
        .await?
        .query(libsql::params_from_iter(query_params.clone()))
        .await?;

    let mut avg_size_after_win = 0.0;
    let mut avg_size_after_loss = 0.0;
    let mut position_consistency = 0.0;

    if let Some(row) = rows.next().await? {
        avg_size_after_win = get_f64_value(&row, 0);
        avg_size_after_loss = get_f64_value(&row, 1);
        position_consistency = get_f64_value(&row, 2);
    }

    // Calculate risk-reward ratio consistency
    let sql = format!(
        r#"
        SELECT
            AVG(CASE
                WHEN ABS(entry_price - stop_loss) > 0
                THEN ABS(calculated_pnl) / ABS(entry_price - stop_loss)
                ELSE 0
            END) as avg_rr_ratio
        FROM (
            SELECT
                *,
                CASE
                    WHEN trade_type = 'BUY' THEN (exit_price - entry_price) * number_shares - commissions
                    WHEN trade_type = 'SELL' THEN (entry_price - exit_price) * number_shares - commissions
                    ELSE 0
                END as calculated_pnl
            FROM stocks
            WHERE exit_price IS NOT NULL AND exit_date IS NOT NULL
              AND stop_loss IS NOT NULL AND ({})
        )
        "#,
        time_condition
    );

    let mut rows = conn
        .prepare(&sql)
        .await?
        .query(libsql::params_from_iter(query_params))
        .await?;

    let mut avg_rr_ratio = 0.0;
    let rr_consistency = 0.0;

    if let Some(row) = rows.next().await? {
        avg_rr_ratio = get_f64_value(&row, 0);
    }

    Ok(RiskBehaviorMetrics {
        stop_loss_hit_percentage: stop_loss_hit_pct,
        stop_loss_adherence_rate: stop_loss_adherence,
        avg_position_size_after_win: avg_size_after_win,
        avg_position_size_after_loss: avg_size_after_loss,
        position_size_consistency_score: 100.0 - position_consistency.clamp(0.0, 100.0),
        avg_risk_reward_ratio: avg_rr_ratio,
        risk_reward_consistency: rr_consistency,
    })
}

/// Calculate timing behavioral patterns
#[allow(dead_code)]
async fn calculate_timing_behavior(
    conn: &Connection,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<TimingBehaviorMetrics> {
    // Get day of week performance
    let sql = format!(
        r#"
        SELECT
            CASE
                WHEN CAST(strftime('%w', entry_date) AS INTEGER) = 0 THEN 'Sunday'
                WHEN CAST(strftime('%w', entry_date) AS INTEGER) = 1 THEN 'Monday'
                WHEN CAST(strftime('%w', entry_date) AS INTEGER) = 2 THEN 'Tuesday'
                WHEN CAST(strftime('%w', entry_date) AS INTEGER) = 3 THEN 'Wednesday'
                WHEN CAST(strftime('%w', entry_date) AS INTEGER) = 4 THEN 'Thursday'
                WHEN CAST(strftime('%w', entry_date) AS INTEGER) = 5 THEN 'Friday'
                WHEN CAST(strftime('%w', entry_date) AS INTEGER) = 6 THEN 'Saturday'
                ELSE 'Unknown'
            END as day_of_week,
            COUNT(*) as trade_count,
            SUM(CASE
                WHEN trade_type = 'BUY' THEN (exit_price - entry_price) * number_shares - commissions
                WHEN trade_type = 'SELL' THEN (entry_price - exit_price) * number_shares - commissions
                ELSE 0
            END) as total_pnl
        FROM stocks
        WHERE exit_price IS NOT NULL AND exit_date IS NOT NULL AND ({})
        GROUP BY day_of_week
        ORDER BY total_pnl DESC
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

    let mut trades_per_day = HashMap::new();
    let mut pnl_per_day = HashMap::new();
    let mut best_day = "None".to_string();
    let mut worst_day = "None".to_string();
    let mut best_pnl = f64::MIN;
    let mut worst_pnl = f64::MAX;

    while let Some(row) = rows.next().await? {
        let day: String = row.get(0)?;
        let count = get_i64_value(&row, 1);
        let pnl = get_f64_value(&row, 2);

        trades_per_day.insert(day.clone(), count as u32);
        pnl_per_day.insert(day.clone(), pnl);

        if pnl > best_pnl {
            best_pnl = pnl;
            best_day = day.clone();
        }
        if pnl < worst_pnl {
            worst_pnl = pnl;
            worst_day = day.clone();
        }
    }

    Ok(TimingBehaviorMetrics {
        best_performing_day: best_day,
        worst_performing_day: worst_day,
        trades_per_day_of_week: trades_per_day,
        pnl_per_day_of_week: pnl_per_day,
        avg_entry_time_morning: 0.0,
        avg_entry_time_afternoon: 0.0,
        avg_entry_time_evening: 0.0,
        early_exit_percentage: 0.0,
        late_exit_percentage: 0.0,
    })
}

/// Calculate trading frequency behavioral patterns
#[allow(dead_code)]
async fn calculate_trading_frequency_behavior(
    conn: &Connection,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<TradingFrequencyMetrics> {
    // Calculate trades per week
    let sql = format!(
        r#"
        SELECT
            COUNT(*) as total_trades,
            CAST(JULIANDAY(MAX(exit_date)) - JULIANDAY(MIN(entry_date)) AS REAL) as days_span,
            COUNT(*) / CAST(JULIANDAY(MAX(exit_date)) - JULIANDAY(MIN(entry_date)) AS REAL) * 7.0 as trades_per_week
        FROM stocks
        WHERE entry_date IS NOT NULL AND exit_date IS NOT NULL AND ({})
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

    let mut trades_per_week = 0.0;

    if let Some(row) = rows.next().await? {
        trades_per_week = get_f64_value(&row, 2);
    }

    // Count over-trading days (days with more than 3 trades)
    let sql = format!(
        r#"
        SELECT COUNT(*)
        FROM (
            SELECT DATE(entry_date) as trade_date
            FROM stocks
            WHERE entry_date IS NOT NULL AND ({})
            GROUP BY trade_date
            HAVING COUNT(*) > 3
        )
        "#,
        time_condition
    );

    let mut rows = conn
        .prepare(&sql)
        .await?
        .query(libsql::params_from_iter(query_params.clone()))
        .await?;

    let mut over_trading_days = 0;
    if let Some(row) = rows.next().await? {
        over_trading_days = get_i64_value(&row, 0) as u32;
    }

    // Calculate optimal trading frequency
    let sql = format!(
        r#"
        SELECT
            AVG(trades_per_day) as avg_trades,
            AVG(CASE WHEN avg_pnl > 0 THEN trades_per_day ELSE NULL END) as avg_trades_on_winning_days,
            AVG(CASE WHEN avg_pnl < 0 THEN trades_per_day ELSE NULL END) as avg_trades_on_losing_days
        FROM (
            SELECT
                DATE(entry_date) as trade_date,
                COUNT(*) as trades_per_day,
                AVG(CASE
                    WHEN trade_type = 'BUY' THEN (exit_price - entry_price) * number_shares - commissions
                    WHEN trade_type = 'SELL' THEN (entry_price - exit_price) * number_shares - commissions
                    ELSE 0
                END) as avg_pnl
            FROM stocks
            WHERE entry_date IS NOT NULL AND exit_date IS NOT NULL AND ({})
            GROUP BY trade_date
        )
        "#,
        time_condition
    );

    let mut rows = conn
        .prepare(&sql)
        .await?
        .query(libsql::params_from_iter(query_params))
        .await?;

    let mut optimal_frequency = 0.0;
    let mut avg_trades_winning = 0.0;
    let mut avg_trades_losing = 0.0;

    if let Some(row) = rows.next().await? {
        optimal_frequency = get_f64_value(&row, 0);
        avg_trades_winning = get_f64_value(&row, 1);
        avg_trades_losing = get_f64_value(&row, 2);
    }

    Ok(TradingFrequencyMetrics {
        trades_per_week,
        over_trading_days,
        under_trading_days: 0,
        optimal_trading_frequency: optimal_frequency,
        trading_consistency_score: 0.0,
        avg_trades_per_winning_day: avg_trades_winning,
        avg_trades_per_losing_day: avg_trades_losing,
    })
}

/// Calculate profitability distribution patterns
#[allow(dead_code)]
async fn calculate_profitability_distribution(
    conn: &Connection,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<ProfitabilityDistributionMetrics> {
    // Calculate best and worst trade impact
    let sql = format!(
        r#"
        SELECT
            MAX(CASE WHEN calculated_pnl > 0 THEN calculated_pnl ELSE NULL END) as best_trade,
            MIN(CASE WHEN calculated_pnl < 0 THEN calculated_pnl ELSE NULL END) as worst_trade,
            SUM(CASE WHEN calculated_pnl > 0 THEN calculated_pnl ELSE 0 END) as total_profit,
            SUM(CASE WHEN calculated_pnl < 0 THEN calculated_pnl ELSE 0 END) as total_loss
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

    let mut best_trade = 0.0;
    let mut worst_trade = 0.0;
    let mut total_profit = 0.0;
    let mut total_loss = 0.0;

    if let Some(row) = rows.next().await? {
        best_trade = get_f64_value(&row, 0);
        worst_trade = get_f64_value(&row, 1);
        total_profit = get_f64_value(&row, 2);
        total_loss = get_f64_value(&row, 3).abs();
    }

    let best_trade_pct = if total_profit > 0.0 {
        (best_trade / total_profit) * 100.0
    } else {
        0.0
    };

    let worst_trade_pct = if total_loss > 0.0 {
        (worst_trade.abs() / total_loss) * 100.0
    } else {
        0.0
    };

    let profit_distribution_score = 100.0 - best_trade_pct;

    Ok(ProfitabilityDistributionMetrics {
        best_trade_pct_of_total_profit: best_trade_pct,
        worst_trade_pct_of_total_loss: worst_trade_pct,
        profit_distribution_score,
        outlier_trades_count: 0,
        largest_win_drawdown: 0.0,
        worst_trade_recovery_time: 0.0,
        consecutive_wins_avg_profit: 0.0,
        consecutive_losses_avg_loss: 0.0,
    })
}
