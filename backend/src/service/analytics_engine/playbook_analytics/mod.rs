use crate::models::stock::stocks::TimeRange;
use anyhow::Result;
use libsql::Connection;
use serde::{Deserialize, Serialize};

/// Playbook analytics metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybookAnalytics {
    pub playbook_id: String,
    pub playbook_name: String,
    pub total_trades: u32,
    pub stock_trades: u32,
    pub option_trades: u32,
    pub win_rate: f64,
    pub net_pnl: f64,
    pub missed_trades: u32,
    pub profit_factor: f64,
    pub expectancy: f64,
    pub average_winner: f64,
    pub average_loser: f64,
    // Compliance stats
    pub total_trades_with_compliance_data: u32,
    pub fully_compliant_trades: u32,
    pub fully_compliant_win_rate: f64,
    pub partially_compliant_trades: u32,
    pub non_compliant_trades: u32,
}

/// Core metrics for playbook
#[derive(Debug, Clone)]
struct PlaybookCoreMetrics {
    total_trades: u32,
    stock_trades: u32,
    option_trades: u32,
    winning_trades: u32,
    losing_trades: u32,
    total_pnl: f64,
    gross_profit: f64,
    gross_loss: f64,
    average_win: f64,
    average_loss: f64,
}

/// Compliance statistics
#[derive(Debug, Clone)]
struct ComplianceStats {
    total_trades_with_compliance_data: u32,
    fully_compliant_trades: u32,
    #[allow(dead_code)]
    fully_compliant_win_rate: f64,
    partially_compliant_trades: u32,
    non_compliant_trades: u32,
}

/// Calculate analytics for a specific playbook
pub async fn calculate_playbook_analytics(
    conn: &Connection,
    playbook_id: &str,
    time_range: &TimeRange,
) -> Result<PlaybookAnalytics> {
    let (time_condition, time_params) = time_range.to_sql_condition();

    // Get playbook name
    let playbook_name = get_playbook_name(conn, playbook_id).await?;

    // Calculate core metrics for stocks and options
    let core_metrics =
        calculate_playbook_core_metrics(conn, playbook_id, &time_condition, &time_params).await?;

    // Calculate missed trades count
    let missed_trades =
        calculate_missed_trades_count(conn, playbook_id, &time_condition, &time_params).await?;

    // Calculate compliance stats
    let compliance_stats =
        calculate_compliance_stats(conn, playbook_id, &time_condition, &time_params).await?;

    // Calculate derived metrics
    let win_rate = if core_metrics.total_trades > 0 {
        (core_metrics.winning_trades as f64 / core_metrics.total_trades as f64) * 100.0
    } else {
        0.0
    };

    let profit_factor = if core_metrics.gross_loss != 0.0 {
        core_metrics.gross_profit.abs() / core_metrics.gross_loss.abs()
    } else if core_metrics.gross_profit > 0.0 {
        f64::INFINITY
    } else {
        0.0
    };

    let expectancy = if core_metrics.total_trades > 0 {
        (core_metrics.average_win
            * (core_metrics.winning_trades as f64 / core_metrics.total_trades as f64))
            + (core_metrics.average_loss
                * (core_metrics.losing_trades as f64 / core_metrics.total_trades as f64))
    } else {
        0.0
    };

    let fully_compliant_win_rate = if compliance_stats.fully_compliant_trades > 0 {
        let fully_compliant_wins =
            calculate_fully_compliant_wins(conn, playbook_id, &time_condition, &time_params)
                .await?;
        (fully_compliant_wins as f64 / compliance_stats.fully_compliant_trades as f64) * 100.0
    } else {
        0.0
    };

    Ok(PlaybookAnalytics {
        playbook_id: playbook_id.to_string(),
        playbook_name,
        total_trades: core_metrics.total_trades,
        stock_trades: core_metrics.stock_trades,
        option_trades: core_metrics.option_trades,
        win_rate,
        net_pnl: core_metrics.total_pnl,
        missed_trades,
        profit_factor,
        expectancy,
        average_winner: core_metrics.average_win,
        average_loser: core_metrics.average_loss,
        total_trades_with_compliance_data: compliance_stats.total_trades_with_compliance_data,
        fully_compliant_trades: compliance_stats.fully_compliant_trades,
        fully_compliant_win_rate,
        partially_compliant_trades: compliance_stats.partially_compliant_trades,
        non_compliant_trades: compliance_stats.non_compliant_trades,
    })
}

/// Helper function to safely extract f64 from libsql::Value
fn get_f64_value(row: &libsql::Row, index: usize) -> f64 {
    match row.get::<libsql::Value>(index as i32) {
        Ok(libsql::Value::Integer(i)) => i as f64,
        Ok(libsql::Value::Real(f)) => f,
        Ok(libsql::Value::Null) => 0.0,
        _ => 0.0,
    }
}

/// Helper function to safely extract i64 from libsql::Value
fn get_i64_value(row: &libsql::Row, index: usize) -> i64 {
    match row.get::<libsql::Value>(index as i32) {
        Ok(libsql::Value::Integer(i)) => i,
        Ok(libsql::Value::Null) => 0,
        _ => 0,
    }
}

/// Get playbook name by ID
pub async fn get_playbook_name(conn: &Connection, playbook_id: &str) -> Result<String> {
    let mut rows = conn
        .prepare("SELECT name FROM playbook WHERE id = ?")
        .await?
        .query(libsql::params![playbook_id])
        .await?;

    if let Some(row) = rows.next().await? {
        Ok(row.get(0)?)
    } else {
        Ok("Unknown".to_string())
    }
}

/// Calculate core metrics for a playbook
async fn calculate_playbook_core_metrics(
    conn: &Connection,
    playbook_id: &str,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<PlaybookCoreMetrics> {
    // Calculate stocks metrics
    let sql = format!(
        r#"
        SELECT 
            COUNT(*) as total_trades,
            SUM(CASE WHEN calculated_pnl > 0 THEN 1 ELSE 0 END) as winning_trades,
            SUM(CASE WHEN calculated_pnl < 0 THEN 1 ELSE 0 END) as losing_trades,
            SUM(calculated_pnl) as total_pnl,
            SUM(CASE WHEN calculated_pnl > 0 THEN calculated_pnl ELSE 0 END) as gross_profit,
            SUM(CASE WHEN calculated_pnl < 0 THEN calculated_pnl ELSE 0 END) as gross_loss,
            AVG(CASE WHEN calculated_pnl > 0 THEN calculated_pnl END) as average_win,
            AVG(CASE WHEN calculated_pnl < 0 THEN calculated_pnl END) as average_loss
        FROM (
            SELECT 
                *,
                CASE 
                    WHEN trade_type = 'BUY' THEN (exit_price - entry_price) * number_shares - commissions
                    WHEN trade_type = 'SELL' THEN (entry_price - exit_price) * number_shares - commissions
                    ELSE 0
                END as calculated_pnl
            FROM stocks s
            JOIN stock_trade_playbook stp ON s.id = stp.stock_trade_id
            WHERE stp.setup_id = ? AND s.exit_price IS NOT NULL AND s.exit_date IS NOT NULL AND ({})
        )
        "#,
        time_condition
    );

    let mut query_params = vec![libsql::Value::Text(playbook_id.to_string())];
    for param in time_params {
        query_params.push(libsql::Value::Text(param.to_rfc3339()));
    }

    let mut rows = conn
        .prepare(&sql)
        .await?
        .query(libsql::params_from_iter(query_params.clone()))
        .await?;

    let mut stocks_metrics = PlaybookCoreMetrics {
        total_trades: 0,
        stock_trades: 0,
        option_trades: 0,
        winning_trades: 0,
        losing_trades: 0,
        total_pnl: 0.0,
        gross_profit: 0.0,
        gross_loss: 0.0,
        average_win: 0.0,
        average_loss: 0.0,
    };

    if let Some(row) = rows.next().await? {
        let total_trades = get_i64_value(&row, 0) as u32;
        let winning_trades = get_i64_value(&row, 1) as u32;
        let losing_trades = get_i64_value(&row, 2) as u32;
        let total_pnl = get_f64_value(&row, 3);
        let gross_profit = get_f64_value(&row, 4);
        let gross_loss = get_f64_value(&row, 5);
        let average_win = get_f64_value(&row, 6);
        let average_loss = get_f64_value(&row, 7);

        stocks_metrics = PlaybookCoreMetrics {
            total_trades,
            stock_trades: total_trades,
            option_trades: 0,
            winning_trades,
            losing_trades,
            total_pnl,
            gross_profit,
            gross_loss,
            average_win,
            average_loss,
        };
    }

    // Calculate options metrics
    let sql = format!(
        r#"
        SELECT 
            COUNT(*) as total_trades,
            SUM(CASE WHEN calculated_pnl > 0 THEN 1 ELSE 0 END) as winning_trades,
            SUM(CASE WHEN calculated_pnl < 0 THEN 1 ELSE 0 END) as losing_trades,
            SUM(calculated_pnl) as total_pnl,
            SUM(CASE WHEN calculated_pnl > 0 THEN calculated_pnl ELSE 0 END) as gross_profit,
            SUM(CASE WHEN calculated_pnl < 0 THEN calculated_pnl ELSE 0 END) as gross_loss,
            AVG(CASE WHEN calculated_pnl > 0 THEN calculated_pnl END) as average_win,
            AVG(CASE WHEN calculated_pnl < 0 THEN calculated_pnl END) as average_loss
        FROM (
            SELECT 
                *,
                CASE 
                    WHEN exit_price IS NOT NULL THEN 
                        (exit_price - entry_price) * total_quantity * 100 - commissions
                    ELSE 0
                END as calculated_pnl
            FROM options o
            JOIN option_trade_playbook otp ON o.id = otp.option_trade_id
            WHERE otp.setup_id = ? AND o.status = 'closed' AND ({})
        )
        "#,
        time_condition
    );

    let mut query_params = vec![libsql::Value::Text(playbook_id.to_string())];
    for param in time_params {
        query_params.push(libsql::Value::Text(param.to_rfc3339()));
    }

    let mut rows = conn
        .prepare(&sql)
        .await?
        .query(libsql::params_from_iter(query_params))
        .await?;

    let mut options_metrics = PlaybookCoreMetrics {
        total_trades: 0,
        stock_trades: 0,
        option_trades: 0,
        winning_trades: 0,
        losing_trades: 0,
        total_pnl: 0.0,
        gross_profit: 0.0,
        gross_loss: 0.0,
        average_win: 0.0,
        average_loss: 0.0,
    };

    if let Some(row) = rows.next().await? {
        let total_trades = get_i64_value(&row, 0) as u32;
        let winning_trades = get_i64_value(&row, 1) as u32;
        let losing_trades = get_i64_value(&row, 2) as u32;
        let total_pnl = get_f64_value(&row, 3);
        let gross_profit = get_f64_value(&row, 4);
        let gross_loss = get_f64_value(&row, 5);
        let average_win = get_f64_value(&row, 6);
        let average_loss = get_f64_value(&row, 7);

        options_metrics = PlaybookCoreMetrics {
            total_trades,
            stock_trades: 0,
            option_trades: total_trades,
            winning_trades,
            losing_trades,
            total_pnl,
            gross_profit,
            gross_loss,
            average_win,
            average_loss,
        };
    }

    // Combine metrics
    Ok(PlaybookCoreMetrics {
        total_trades: stocks_metrics.total_trades + options_metrics.total_trades,
        stock_trades: stocks_metrics.stock_trades,
        option_trades: options_metrics.option_trades,
        winning_trades: stocks_metrics.winning_trades + options_metrics.winning_trades,
        losing_trades: stocks_metrics.losing_trades + options_metrics.losing_trades,
        total_pnl: stocks_metrics.total_pnl + options_metrics.total_pnl,
        gross_profit: stocks_metrics.gross_profit + options_metrics.gross_profit,
        gross_loss: stocks_metrics.gross_loss + options_metrics.gross_loss,
        average_win: if stocks_metrics.winning_trades + options_metrics.winning_trades > 0 {
            (stocks_metrics.average_win * stocks_metrics.winning_trades as f64
                + options_metrics.average_win * options_metrics.winning_trades as f64)
                / (stocks_metrics.winning_trades + options_metrics.winning_trades) as f64
        } else {
            0.0
        },
        average_loss: if stocks_metrics.losing_trades + options_metrics.losing_trades > 0 {
            (stocks_metrics.average_loss * stocks_metrics.losing_trades as f64
                + options_metrics.average_loss * options_metrics.losing_trades as f64)
                / (stocks_metrics.losing_trades + options_metrics.losing_trades) as f64
        } else {
            0.0
        },
    })
}

/// Calculate missed trades count
async fn calculate_missed_trades_count(
    conn: &Connection,
    playbook_id: &str,
    _time_condition: &str,
    _time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<u32> {
    let sql = r#"
        SELECT COUNT(*) as missed_count
        FROM missed_trades
        WHERE playbook_id = ?
        "#;

    let mut rows = conn
        .prepare(sql)
        .await?
        .query(libsql::params![playbook_id])
        .await?;

    if let Some(row) = rows.next().await? {
        Ok(get_i64_value(&row, 0) as u32)
    } else {
        Ok(0)
    }
}

/// Calculate compliance statistics
async fn calculate_compliance_stats(
    conn: &Connection,
    playbook_id: &str,
    _time_condition: &str,
    _time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<ComplianceStats> {
    // Get total number of rules for this playbook
    let total_rules = get_total_rules_count(conn, playbook_id).await?;

    if total_rules == 0 {
        return Ok(ComplianceStats {
            total_trades_with_compliance_data: 0,
            fully_compliant_trades: 0,
            fully_compliant_win_rate: 0.0,
            partially_compliant_trades: 0,
            non_compliant_trades: 0,
        });
    }

    // Get trades with compliance data (need to check both stock and option compliance tables)
    let sql = r#"
        SELECT DISTINCT trade_id, trade_type
        FROM (
            SELECT stock_trade_id as trade_id, 'stock' as trade_type
            FROM stock_trade_rule_compliance
            WHERE playbook_id = ?
            
            UNION ALL
            
            SELECT option_trade_id as trade_id, 'option' as trade_type
            FROM option_trade_rule_compliance
            WHERE playbook_id = ?
        )
        "#
    .to_string();

    let query_params = vec![
        libsql::Value::Text(playbook_id.to_string()),
        libsql::Value::Text(playbook_id.to_string()),
    ];

    let mut rows = conn
        .prepare(&sql)
        .await?
        .query(libsql::params_from_iter(query_params))
        .await?;

    let mut trades_with_compliance = Vec::new();
    while let Some(row) = rows.next().await? {
        trades_with_compliance.push((
            get_i64_value(&row, 0),
            row.get::<String>(1)
                .unwrap_or_else(|_| "unknown".to_string()),
        ));
    }

    let total_trades_with_compliance_data = trades_with_compliance.len() as u32;

    // Count compliant trades
    let mut fully_compliant = 0;
    let mut partially_compliant = 0;
    let mut non_compliant = 0;

    for (trade_id, trade_type) in trades_with_compliance {
        let compliance_rate =
            get_trade_compliance_rate(conn, trade_id, &trade_type, playbook_id, total_rules)
                .await?;

        if compliance_rate >= 1.0 {
            fully_compliant += 1;
        } else if compliance_rate > 0.0 {
            partially_compliant += 1;
        } else {
            non_compliant += 1;
        }
    }

    Ok(ComplianceStats {
        total_trades_with_compliance_data,
        fully_compliant_trades: fully_compliant,
        fully_compliant_win_rate: 0.0, // Will be calculated later
        partially_compliant_trades: partially_compliant,
        non_compliant_trades: non_compliant,
    })
}

/// Get total number of rules for a playbook
async fn get_total_rules_count(conn: &Connection, playbook_id: &str) -> Result<u32> {
    let mut rows = conn
        .prepare("SELECT COUNT(*) FROM playbook_rules WHERE playbook_id = ?")
        .await?
        .query(libsql::params![playbook_id])
        .await?;

    if let Some(row) = rows.next().await? {
        Ok(get_i64_value(&row, 0) as u32)
    } else {
        Ok(0)
    }
}

/// Get compliance rate for a specific trade
async fn get_trade_compliance_rate(
    conn: &Connection,
    trade_id: i64,
    trade_type: &str,
    playbook_id: &str,
    total_rules: u32,
) -> Result<f64> {
    let (table_name, id_column) = match trade_type {
        "stock" => ("stock_trade_rule_compliance", "stock_trade_id"),
        "option" => ("option_trade_rule_compliance", "option_trade_id"),
        _ => return Ok(0.0),
    };

    let sql = format!(
        r#"
        SELECT 
            COUNT(*) as total_rules_checked,
            SUM(CASE WHEN is_followed = 1 THEN 1 ELSE 0 END) as followed_rules
        FROM {}
        WHERE {} = ? AND playbook_id = ?
        "#,
        table_name, id_column
    );

    let mut rows = conn
        .prepare(&sql)
        .await?
        .query(libsql::params![trade_id, playbook_id])
        .await?;

    if let Some(row) = rows.next().await? {
        let total_checked = get_i64_value(&row, 0) as f64;
        let followed = get_i64_value(&row, 1) as f64;

        if total_checked > 0.0 && total_checked == total_rules as f64 {
            Ok(followed / total_checked)
        } else {
            Ok(0.0)
        }
    } else {
        Ok(0.0)
    }
}

/// Calculate fully compliant wins
async fn calculate_fully_compliant_wins(
    conn: &Connection,
    playbook_id: &str,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<u32> {
    // Get total number of rules
    let total_rules = get_total_rules_count(conn, playbook_id).await?;

    if total_rules == 0 {
        return Ok(0);
    }

    // Get all trades that are fully compliant
    let sql = r#"
        SELECT DISTINCT stock_trade_id
        FROM stock_trade_rule_compliance
        WHERE playbook_id = ?
        GROUP BY stock_trade_id
        HAVING COUNT(*) = ? AND SUM(CASE WHEN is_followed = 1 THEN 1 ELSE 0 END) = ?
        "#
    .to_string();

    let query_params = vec![
        libsql::Value::Text(playbook_id.to_string()),
        libsql::Value::Integer(total_rules as i64),
        libsql::Value::Integer(total_rules as i64),
    ];

    let mut rows = conn
        .prepare(&sql)
        .await?
        .query(libsql::params_from_iter(query_params))
        .await?;

    let mut fully_compliant_stock_trades = Vec::new();
    while let Some(row) = rows.next().await? {
        fully_compliant_stock_trades.push(get_i64_value(&row, 0));
    }

    // Check if these stock trades are winners
    let winning_count = if !fully_compliant_stock_trades.is_empty() {
        let placeholders: Vec<String> = fully_compliant_stock_trades
            .iter()
            .map(|_| "?".to_string())
            .collect();

        let sql = format!(
            r#"
            SELECT COUNT(*)
            FROM stocks
            WHERE id IN ({}) AND exit_price IS NOT NULL AND exit_date IS NOT NULL AND ({})
              AND (
                  (trade_type = 'BUY' AND exit_price > entry_price) OR
                  (trade_type = 'SELL' AND exit_price < entry_price)
              )
            "#,
            placeholders.join(", "),
            time_condition
        );

        let mut query_params = Vec::new();
        for trade_id in &fully_compliant_stock_trades {
            query_params.push(libsql::Value::Integer(*trade_id));
        }
        for param in time_params {
            query_params.push(libsql::Value::Text(param.to_rfc3339()));
        }

        let mut rows = conn
            .prepare(&sql)
            .await?
            .query(libsql::params_from_iter(query_params))
            .await?;

        if let Some(row) = rows.next().await? {
            get_i64_value(&row, 0) as u32
        } else {
            0
        }
    } else {
        0
    };

    // Do the same for options
    let sql = r#"
        SELECT DISTINCT option_trade_id
        FROM option_trade_rule_compliance
        WHERE playbook_id = ?
        GROUP BY option_trade_id
        HAVING COUNT(*) = ? AND SUM(CASE WHEN is_followed = 1 THEN 1 ELSE 0 END) = ?
        "#
    .to_string();

    let query_params = vec![
        libsql::Value::Text(playbook_id.to_string()),
        libsql::Value::Integer(total_rules as i64),
        libsql::Value::Integer(total_rules as i64),
    ];

    let mut rows = conn
        .prepare(&sql)
        .await?
        .query(libsql::params_from_iter(query_params))
        .await?;

    let mut fully_compliant_option_trades = Vec::new();
    while let Some(row) = rows.next().await? {
        fully_compliant_option_trades.push(get_i64_value(&row, 0));
    }

    let winning_options_count = if !fully_compliant_option_trades.is_empty() {
        let placeholders: Vec<String> = fully_compliant_option_trades
            .iter()
            .map(|_| "?".to_string())
            .collect();

        let sql = format!(
            r#"
            SELECT COUNT(*)
            FROM options
            WHERE id IN ({}) AND status = 'closed' AND exit_price IS NOT NULL AND ({})
              AND exit_price > entry_price
            "#,
            placeholders.join(", "),
            time_condition
        );

        let mut query_params = Vec::new();
        for trade_id in &fully_compliant_option_trades {
            query_params.push(libsql::Value::Integer(*trade_id));
        }
        for param in time_params {
            query_params.push(libsql::Value::Text(param.to_rfc3339()));
        }

        let mut rows = conn
            .prepare(&sql)
            .await?
            .query(libsql::params_from_iter(query_params))
            .await?;

        if let Some(row) = rows.next().await? {
            get_i64_value(&row, 0) as u32
        } else {
            0
        }
    } else {
        0
    };

    Ok(winning_count + winning_options_count)
}
