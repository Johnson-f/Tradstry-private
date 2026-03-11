use crate::models::analytics::{AnalyticsOptions, RiskMetrics};
use crate::models::stock::stocks::TimeRange;
use anyhow::Result;
use libsql::Connection;

/// Calculate risk-adjusted metrics including average risk per trade
pub async fn calculate_risk_metrics(
    conn: &Connection,
    time_range: &TimeRange,
    options: &AnalyticsOptions,
) -> Result<RiskMetrics> {
    let (time_condition, time_params) = time_range.to_sql_condition();

    // Calculate average risk per trade
    let avg_risk_per_trade =
        calculate_average_risk_per_trade(conn, &time_condition, &time_params).await?;

    // Calculate daily returns for Sharpe/Sortino ratios
    let daily_returns = calculate_daily_returns(conn, &time_condition, &time_params).await?;

    // Calculate drawdown metrics
    let drawdown_metrics = calculate_drawdown_metrics(&daily_returns).await?;

    // Calculate VaR and CVaR
    let var_metrics = calculate_var_metrics(&daily_returns, &options.confidence_levels).await?;

    // Calculate volatility metrics
    let volatility_metrics = calculate_volatility_metrics(&daily_returns).await?;

    // Calculate risk-adjusted ratios
    let ratios =
        calculate_risk_ratios(&daily_returns, options.risk_free_rate, &drawdown_metrics).await?;

    Ok(RiskMetrics {
        sharpe_ratio: ratios.sharpe_ratio,
        sortino_ratio: ratios.sortino_ratio,
        calmar_ratio: ratios.calmar_ratio,
        maximum_drawdown: drawdown_metrics.maximum_drawdown,
        maximum_drawdown_percentage: drawdown_metrics.maximum_drawdown_percentage,
        maximum_drawdown_duration_days: drawdown_metrics.maximum_drawdown_duration_days,
        current_drawdown: drawdown_metrics.current_drawdown,
        var_95: var_metrics.var_95,
        var_99: var_metrics.var_99,
        expected_shortfall_95: var_metrics.expected_shortfall_95,
        expected_shortfall_99: var_metrics.expected_shortfall_99,
        average_risk_per_trade: avg_risk_per_trade,
        risk_reward_ratio: 0.0, // Will be calculated from core metrics
        volatility: volatility_metrics.volatility,
        downside_deviation: volatility_metrics.downside_deviation,
        ulcer_index: drawdown_metrics.ulcer_index,
        recovery_factor: drawdown_metrics.recovery_factor,
        sterling_ratio: ratios.sterling_ratio,
    })
}

/// Calculate average risk per trade from stop loss data
async fn calculate_average_risk_per_trade(
    conn: &Connection,
    time_condition: &str,
    time_params: &[chrono::DateTime<chrono::Utc>],
) -> Result<f64> {
    // Calculate risk for stocks (entry_price - stop_loss) * number_shares
    let stocks_sql = format!(
        r#"
        SELECT AVG(ABS(entry_price - stop_loss) * number_shares) as avg_risk_stocks
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
        .prepare(&stocks_sql)
        .await?
        .query(libsql::params_from_iter(query_params.clone()))
        .await?;

    let mut stocks_avg_risk = 0.0;
    if let Some(row) = rows.next().await? {
        stocks_avg_risk = row.get::<f64>(0).unwrap_or(0.0);
    }

    // For options, risk is typically the premium paid (premium)
    let options_sql = format!(
        r#"
        SELECT AVG(premium) as avg_risk_options
        FROM options
        WHERE status = 'closed' AND ({})
        "#,
        time_condition
    );

    let mut rows = conn
        .prepare(&options_sql)
        .await?
        .query(libsql::params_from_iter(query_params))
        .await?;

    let mut options_avg_risk = 0.0;
    if let Some(row) = rows.next().await? {
        options_avg_risk = row.get::<f64>(0).unwrap_or(0.0);
    }

    // Return the average of both (could be weighted by trade count if needed)
    Ok((stocks_avg_risk + options_avg_risk) / 2.0)
}

/// Calculate daily returns from trade data
async fn calculate_daily_returns(
    conn: &Connection,
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
            WHERE exit_price IS NOT NULL AND exit_date IS NOT NULL AND ({})
            
            UNION ALL
            
            SELECT 
                *,
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
        let daily_pnl = row.get::<f64>(1).unwrap_or(0.0);
        daily_returns.push(daily_pnl);
    }

    Ok(daily_returns)
}

/// Calculate drawdown metrics from daily returns
async fn calculate_drawdown_metrics(daily_returns: &[f64]) -> Result<DrawdownMetrics> {
    if daily_returns.is_empty() {
        return Ok(DrawdownMetrics::default());
    }

    let mut cumulative_pnl = 0.0;
    let mut peak = 0.0;
    let mut max_drawdown: f64 = 0.0;
    let mut max_drawdown_percentage: f64 = 0.0;
    let mut current_drawdown_duration = 0;
    let mut max_drawdown_duration = 0;
    let mut ulcer_sum = 0.0;

    for &pnl in daily_returns {
        cumulative_pnl += pnl;

        if cumulative_pnl > peak {
            peak = cumulative_pnl;
            current_drawdown_duration = 0;
        } else {
            current_drawdown_duration += 1;
            max_drawdown_duration = max_drawdown_duration.max(current_drawdown_duration);
        }

        let drawdown = peak - cumulative_pnl;
        max_drawdown = max_drawdown.max(drawdown);

        if peak > 0.0 {
            let drawdown_percentage = (drawdown / peak) * 100.0;
            max_drawdown_percentage = max_drawdown_percentage.max(drawdown_percentage);
        }

        // Ulcer Index calculation
        if peak > 0.0 {
            ulcer_sum += ((drawdown / peak) * 100.0).powi(2);
        }
    }

    let ulcer_index = if !daily_returns.is_empty() {
        (ulcer_sum / daily_returns.len() as f64).sqrt()
    } else {
        0.0
    };

    let current_drawdown = peak - cumulative_pnl;
    let recovery_factor = if max_drawdown > 0.0 {
        cumulative_pnl / max_drawdown
    } else {
        0.0
    };

    Ok(DrawdownMetrics {
        maximum_drawdown: max_drawdown,
        maximum_drawdown_percentage: max_drawdown_percentage,
        maximum_drawdown_duration_days: max_drawdown_duration,
        current_drawdown,
        ulcer_index,
        recovery_factor,
    })
}

/// Calculate Value at Risk and Expected Shortfall
async fn calculate_var_metrics(
    daily_returns: &[f64],
    _confidence_levels: &[f64],
) -> Result<VarMetrics> {
    if daily_returns.is_empty() {
        return Ok(VarMetrics::default());
    }

    let mut sorted_returns = daily_returns.to_vec();
    sorted_returns.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let var_95 = calculate_var(&sorted_returns, 0.95);
    let var_99 = calculate_var(&sorted_returns, 0.99);

    let expected_shortfall_95 = calculate_expected_shortfall(&sorted_returns, 0.95);
    let expected_shortfall_99 = calculate_expected_shortfall(&sorted_returns, 0.99);

    Ok(VarMetrics {
        var_95,
        var_99,
        expected_shortfall_95,
        expected_shortfall_99,
    })
}

/// Calculate Value at Risk for given confidence level
fn calculate_var(sorted_returns: &[f64], confidence_level: f64) -> f64 {
    let index = ((1.0 - confidence_level) * sorted_returns.len() as f64) as usize;
    let clamped_index = index.min(sorted_returns.len() - 1);
    sorted_returns[clamped_index]
}

/// Calculate Expected Shortfall (CVaR) for given confidence level
fn calculate_expected_shortfall(sorted_returns: &[f64], confidence_level: f64) -> f64 {
    let var_threshold = calculate_var(sorted_returns, confidence_level);
    let tail_returns: Vec<f64> = sorted_returns
        .iter()
        .filter(|&&x| x <= var_threshold)
        .cloned()
        .collect();

    if tail_returns.is_empty() {
        0.0
    } else {
        tail_returns.iter().sum::<f64>() / tail_returns.len() as f64
    }
}

/// Calculate volatility metrics
async fn calculate_volatility_metrics(daily_returns: &[f64]) -> Result<VolatilityMetrics> {
    if daily_returns.len() < 2 {
        return Ok(VolatilityMetrics::default());
    }

    let mean = daily_returns.iter().sum::<f64>() / daily_returns.len() as f64;

    let variance = daily_returns
        .iter()
        .map(|&x| (x - mean).powi(2))
        .sum::<f64>()
        / (daily_returns.len() - 1) as f64;

    let volatility = variance.sqrt();

    // Calculate downside deviation (only negative returns)
    let negative_returns: Vec<f64> = daily_returns
        .iter()
        .filter(|&&x| x < 0.0)
        .cloned()
        .collect();

    let downside_deviation = if negative_returns.len() > 1 {
        let downside_mean = negative_returns.iter().sum::<f64>() / negative_returns.len() as f64;
        let downside_variance = negative_returns
            .iter()
            .map(|&x| (x - downside_mean).powi(2))
            .sum::<f64>()
            / (negative_returns.len() - 1) as f64;
        downside_variance.sqrt()
    } else {
        0.0
    };

    Ok(VolatilityMetrics {
        volatility,
        downside_deviation,
    })
}

/// Calculate risk-adjusted ratios
async fn calculate_risk_ratios(
    daily_returns: &[f64],
    risk_free_rate: f64,
    drawdown_metrics: &DrawdownMetrics,
) -> Result<RiskRatios> {
    if daily_returns.is_empty() {
        return Ok(RiskRatios::default());
    }

    let mean_return = daily_returns.iter().sum::<f64>() / daily_returns.len() as f64;
    let daily_risk_free_rate = risk_free_rate / 365.0; // Convert annual to daily
    let excess_return = mean_return - daily_risk_free_rate;

    // Calculate volatility for Sharpe ratio
    let variance = daily_returns
        .iter()
        .map(|&x| (x - mean_return).powi(2))
        .sum::<f64>()
        / daily_returns.len() as f64;
    let volatility = variance.sqrt();

    let sharpe_ratio = if volatility > 0.0 {
        excess_return / volatility
    } else {
        0.0
    };

    // Calculate downside deviation for Sortino ratio
    let negative_returns: Vec<f64> = daily_returns
        .iter()
        .filter(|&&x| x < 0.0)
        .cloned()
        .collect();

    let downside_deviation = if negative_returns.len() > 1 {
        let downside_mean = negative_returns.iter().sum::<f64>() / negative_returns.len() as f64;
        let downside_variance = negative_returns
            .iter()
            .map(|&x| (x - downside_mean).powi(2))
            .sum::<f64>()
            / (negative_returns.len() - 1) as f64;
        downside_variance.sqrt()
    } else {
        0.0
    };

    let sortino_ratio = if downside_deviation > 0.0 {
        excess_return / downside_deviation
    } else {
        0.0
    };

    let calmar_ratio = if drawdown_metrics.maximum_drawdown > 0.0 {
        mean_return * 365.0 / drawdown_metrics.maximum_drawdown // Annualized return / max drawdown
    } else {
        0.0
    };

    let sterling_ratio = if drawdown_metrics.maximum_drawdown > 0.0 {
        mean_return * 365.0 / drawdown_metrics.maximum_drawdown
    } else {
        0.0
    };

    Ok(RiskRatios {
        sharpe_ratio,
        sortino_ratio,
        calmar_ratio,
        sterling_ratio,
    })
}

// Helper structs for intermediate calculations
#[derive(Debug, Default)]
struct DrawdownMetrics {
    maximum_drawdown: f64,
    maximum_drawdown_percentage: f64,
    maximum_drawdown_duration_days: u32,
    current_drawdown: f64,
    ulcer_index: f64,
    recovery_factor: f64,
}

#[derive(Debug, Default)]
struct VarMetrics {
    var_95: f64,
    var_99: f64,
    expected_shortfall_95: f64,
    expected_shortfall_99: f64,
}

#[derive(Debug, Default)]
struct VolatilityMetrics {
    volatility: f64,
    downside_deviation: f64,
}

#[derive(Debug, Default)]
struct RiskRatios {
    sharpe_ratio: f64,
    sortino_ratio: f64,
    calmar_ratio: f64,
    sterling_ratio: f64,
}

impl Default for RiskMetrics {
    fn default() -> Self {
        Self {
            sharpe_ratio: 0.0,
            sortino_ratio: 0.0,
            calmar_ratio: 0.0,
            maximum_drawdown: 0.0,
            maximum_drawdown_percentage: 0.0,
            maximum_drawdown_duration_days: 0,
            current_drawdown: 0.0,
            var_95: 0.0,
            var_99: 0.0,
            expected_shortfall_95: 0.0,
            expected_shortfall_99: 0.0,
            average_risk_per_trade: 0.0,
            risk_reward_ratio: 0.0,
            volatility: 0.0,
            downside_deviation: 0.0,
            ulcer_index: 0.0,
            recovery_factor: 0.0,
            sterling_ratio: 0.0,
        }
    }
}
