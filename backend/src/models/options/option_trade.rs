use chrono::{DateTime, Utc};
use libsql::{Connection, params};
use serde::{Deserialize, Deserializer, Serialize};

/// Re-use the TimeRange enum from the stock model
use crate::models::stock::stocks::TimeRange;

/// Trade status enum matching the PostgreSQL enum in your schema
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TradeStatus {
    Open,
    Closed,
}

impl std::fmt::Display for TradeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TradeStatus::Open => write!(f, "open"),
            TradeStatus::Closed => write!(f, "closed"),
        }
    }
}

impl std::str::FromStr for TradeStatus {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "open" => Ok(TradeStatus::Open),
            "closed" => Ok(TradeStatus::Closed),
            _ => Err("Invalid trade status"),
        }
    }
}

/// Trade direction enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TradeDirection {
    Bullish,
    Bearish,
    Neutral,
}

impl std::fmt::Display for TradeDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TradeDirection::Bullish => write!(f, "Bullish"),
            TradeDirection::Bearish => write!(f, "Bearish"),
            TradeDirection::Neutral => write!(f, "Neutral"),
        }
    }
}

impl std::str::FromStr for TradeDirection {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Bullish" => Ok(TradeDirection::Bullish),
            "Bearish" => Ok(TradeDirection::Bearish),
            "Neutral" => Ok(TradeDirection::Neutral),
            _ => Err("Invalid trade direction"),
        }
    }
}

/// Option type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OptionType {
    Call,
    Put,
}

impl std::fmt::Display for OptionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OptionType::Call => write!(f, "Call"),
            OptionType::Put => write!(f, "Put"),
        }
    }
}

impl std::str::FromStr for OptionType {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Call" => Ok(OptionType::Call),
            "Put" => Ok(OptionType::Put),
            _ => Err("Invalid option type"),
        }
    }
}

/// Option trade model for user's isolated database
/// No user_id needed since each user has their own database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionTrade {
    pub id: i64,
    pub symbol: String,
    pub option_type: OptionType,
    pub strike_price: f64,
    pub expiration_date: DateTime<Utc>,
    pub entry_price: f64,
    pub exit_price: f64,
    pub premium: f64,
    pub entry_date: DateTime<Utc>,
    pub exit_date: DateTime<Utc>,
    pub status: TradeStatus,
    pub initial_target: Option<f64>,
    pub profit_target: Option<f64>,
    pub trade_ratings: Option<i32>,
    pub reviewed: bool,
    pub mistakes: Option<String>,
    pub brokerage_name: Option<String>,
    pub trade_group_id: Option<String>,
    pub parent_trade_id: Option<i64>,
    pub total_quantity: Option<f64>,
    pub transaction_sequence: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_deleted: bool,
}

/// Simplified response for open option trades (only essential fields)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenOptionTrade {
    pub symbol: String,
    pub entry_price: f64,
    pub entry_date: DateTime<Utc>,
}

/// Data Transfer Object for creating new option trades
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateOptionRequest {
    pub symbol: String,
    pub option_type: OptionType,
    pub strike_price: f64,
    pub expiration_date: DateTime<Utc>,
    pub entry_price: f64,
    pub exit_price: f64,
    pub premium: f64,
    pub entry_date: DateTime<Utc>,
    #[serde(deserialize_with = "deserialize_datetime")]
    pub exit_date: DateTime<Utc>,
    pub initial_target: Option<f64>,
    pub profit_target: Option<f64>,
    pub trade_ratings: Option<i32>,
    pub reviewed: Option<bool>,
    pub mistakes: Option<String>,
    pub brokerage_name: Option<String>,
    pub trade_group_id: Option<String>,
    pub parent_trade_id: Option<i64>,
    pub total_quantity: Option<f64>,
    pub transaction_sequence: Option<i32>,
}

/// Data Transfer Object for updating option trades
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOptionRequest {
    pub symbol: Option<String>,
    pub option_type: Option<OptionType>,
    pub strike_price: Option<f64>,
    pub expiration_date: Option<DateTime<Utc>>,
    pub entry_price: Option<f64>,
    pub exit_price: Option<f64>,
    pub premium: Option<f64>,
    pub entry_date: Option<DateTime<Utc>>,
    pub exit_date: Option<DateTime<Utc>>,
    pub status: Option<TradeStatus>,
    pub initial_target: Option<f64>,
    pub profit_target: Option<f64>,
    pub trade_ratings: Option<i32>,
    pub reviewed: Option<bool>,
    pub mistakes: Option<String>,
    pub brokerage_name: Option<String>,
    pub trade_group_id: Option<String>,
    pub parent_trade_id: Option<i64>,
    pub total_quantity: Option<f64>,
    pub transaction_sequence: Option<i32>,
}

/// Option query parameters for filtering and pagination
#[derive(Debug, Serialize, Deserialize)]
pub struct OptionQuery {
    pub symbol: Option<String>,
    pub option_type: Option<OptionType>,
    pub status: Option<TradeStatus>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub time_range: Option<TimeRange>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    #[serde(rename = "openOnly")]
    pub open_only: Option<bool>,
    pub trade_group_id: Option<String>,
    pub parent_trade_id: Option<i64>,
}

fn deserialize_datetime<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    DateTime::parse_from_rfc3339(&s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(serde::de::Error::custom)
}

/// Option operations implementation using libsql
impl OptionTrade {
    fn get_f64(
        row: &libsql::Row,
        idx: usize,
    ) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        let i = idx as i32;

        // Get the raw value first to check its type
        match row.get::<libsql::Value>(i) {
            Ok(libsql::Value::Real(v)) => Ok(v),
            Ok(libsql::Value::Integer(v)) => Ok(v as f64),
            Ok(libsql::Value::Text(s)) => s.parse::<f64>().or(Ok(0.0)),
            Ok(libsql::Value::Null) => Ok(0.0),
            Ok(_) => Ok(0.0), // For any other type (Blob, etc)
            Err(_) => Ok(0.0),
        }
    }

    fn get_opt_f64(
        row: &libsql::Row,
        idx: usize,
    ) -> Result<Option<f64>, Box<dyn std::error::Error + Send + Sync>> {
        let i = idx as i32;

        // Get the raw value first to check its type
        match row.get::<libsql::Value>(i) {
            Ok(libsql::Value::Real(v)) => Ok(Some(v)),
            Ok(libsql::Value::Integer(v)) => Ok(Some(v as f64)),
            Ok(libsql::Value::Text(s)) => Ok(s.parse::<f64>().ok()),
            Ok(libsql::Value::Null) => Ok(None),
            Ok(_) => Ok(None), // For any other type (Blob, etc)
            Err(_) => Ok(None),
        }
    }

    /// Create a new option trade in the user's database
    pub async fn create(
        conn: &Connection,
        request: CreateOptionRequest,
    ) -> Result<OptionTrade, Box<dyn std::error::Error + Send + Sync>> {
        let now = Utc::now().to_rfc3339();

        let mut rows = conn.prepare(
            r#"
            INSERT INTO options (
                symbol, option_type, strike_price, expiration_date, entry_price, exit_price,
                premium, entry_date, exit_date, status, initial_target, profit_target, trade_ratings,
                reviewed, mistakes, brokerage_name, trade_group_id, parent_trade_id,
                total_quantity, transaction_sequence, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id, symbol, option_type, strike_price, expiration_date, entry_price, exit_price,
                     premium, entry_date, exit_date, status, initial_target, profit_target, trade_ratings,
                     reviewed, mistakes, brokerage_name, trade_group_id, parent_trade_id, total_quantity,
                     transaction_sequence, created_at, updated_at, is_deleted
            "#,
        )
        .await?
        .query(params![
            request.symbol,
            request.option_type.to_string(),
            request.strike_price,
            request.expiration_date.to_rfc3339(),
            request.entry_price,
            request.exit_price,
            request.premium,
            request.entry_date.to_rfc3339(),
            request.exit_date.to_rfc3339(),
            TradeStatus::Open.to_string(),
            request.initial_target,
            request.profit_target,
            request.trade_ratings,
            request.reviewed.unwrap_or(false),
            request.mistakes,
            request.brokerage_name,
            request.trade_group_id,
            request.parent_trade_id,
            request.total_quantity,
            request.transaction_sequence,
            now.clone(),
            now
        ])
        .await?;

        if let Some(row) = rows.next().await? {
            Ok(OptionTrade::from_row(&row)?)
        } else {
            Err("Failed to create option trade".into())
        }
    }

    /// Find an option trade by ID in the user's database
    pub async fn find_by_id(
        conn: &Connection,
        option_id: i64,
    ) -> Result<Option<OptionTrade>, Box<dyn std::error::Error + Send + Sync>> {
        let mut rows = conn
            .prepare(
                r#"
                SELECT id, symbol, option_type, strike_price, expiration_date, entry_price, exit_price,
                       premium, entry_date, exit_date, status, initial_target, profit_target, trade_ratings,
                       reviewed, mistakes, brokerage_name, trade_group_id, parent_trade_id, total_quantity,
                       transaction_sequence, created_at, updated_at, is_deleted
                FROM options
                WHERE id = ?
                "#,
            )
            .await?
            .query(params![option_id])
            .await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(OptionTrade::from_row(&row)?))
        } else {
            Ok(None)
        }
    }

    /// Find all option trades with optional filtering
    pub async fn find_all(
        conn: &Connection,
        query: OptionQuery,
    ) -> Result<Vec<OptionTrade>, Box<dyn std::error::Error + Send + Sync>> {
        let mut sql = String::from(
            r#"
            SELECT id, symbol, option_type, strike_price, expiration_date, entry_price, exit_price,
                   premium, entry_date, exit_date, status, initial_target, profit_target, trade_ratings,
                   reviewed, mistakes, brokerage_name, trade_group_id, parent_trade_id, total_quantity,
                   transaction_sequence, created_at, updated_at, is_deleted
            FROM options
            WHERE 1=1
            "#,
        );

        let mut query_params = Vec::new();

        // Add optional filters
        if let Some(symbol) = &query.symbol {
            sql.push_str(" AND symbol = ?");
            query_params.push(libsql::Value::Text(symbol.clone()));
        }

        if let Some(option_type) = &query.option_type {
            sql.push_str(" AND option_type = ?");
            query_params.push(libsql::Value::Text(option_type.to_string()));
        }

        if let Some(trade_group_id) = &query.trade_group_id {
            sql.push_str(" AND trade_group_id = ?");
            query_params.push(libsql::Value::Text(trade_group_id.clone()));
        }

        if let Some(parent_trade_id) = query.parent_trade_id {
            sql.push_str(" AND parent_trade_id = ?");
            query_params.push(libsql::Value::Integer(parent_trade_id));
        }

        if let Some(status) = &query.status {
            sql.push_str(" AND status = ?");
            query_params.push(libsql::Value::Text(status.to_string()));
        }

        if let Some(start_date) = query.start_date {
            sql.push_str(" AND entry_date >= ?");
            query_params.push(libsql::Value::Text(start_date.to_rfc3339()));
        }

        if let Some(end_date) = query.end_date {
            sql.push_str(" AND entry_date <= ?");
            query_params.push(libsql::Value::Text(end_date.to_rfc3339()));
        }

        // Convert time_range to start_date/end_date if provided
        if let Some(time_range) = &query.time_range {
            let (start, end) = time_range.to_dates();
            if let Some(start_date) = start {
                sql.push_str(" AND entry_date >= ?");
                query_params.push(libsql::Value::Text(start_date.to_rfc3339()));
            }
            if let Some(end_date) = end {
                sql.push_str(" AND entry_date <= ?");
                query_params.push(libsql::Value::Text(end_date.to_rfc3339()));
            }
        }

        // Filter by open/closed trades
        if let Some(open_only) = query.open_only {
            if open_only {
                sql.push_str(" AND (status = 'open' OR exit_date IS NULL)");
            } else {
                sql.push_str(" AND (status = 'closed' OR exit_date IS NOT NULL)");
            }
        }

        sql.push_str(" ORDER BY entry_date DESC");

        // Add pagination
        if let Some(limit) = query.limit {
            sql.push_str(" LIMIT ?");
            query_params.push(libsql::Value::Integer(limit));
        }

        if let Some(offset) = query.offset {
            sql.push_str(" OFFSET ?");
            query_params.push(libsql::Value::Integer(offset));
        }

        let mut rows = conn
            .prepare(&sql)
            .await?
            .query(libsql::params_from_iter(query_params))
            .await?;

        let mut options = Vec::new();
        while let Some(row) = rows.next().await? {
            options.push(OptionTrade::from_row(&row)?);
        }

        Ok(options)
    }

    /// Find all open option trades with simplified response (only symbol, entry_price, entry_date)
    pub async fn find_all_open_summary(
        conn: &Connection,
        query: OptionQuery,
    ) -> Result<Vec<OpenOptionTrade>, Box<dyn std::error::Error + Send + Sync>> {
        let mut sql = String::from(
            r#"
            SELECT symbol, entry_price, entry_date
            FROM options 
            WHERE (status = 'open' OR exit_date IS NULL) AND is_deleted = 0
            "#,
        );

        let mut query_params = Vec::new();

        // Add optional filters (excluding open_only since we're already filtering for open)
        if let Some(symbol) = &query.symbol {
            sql.push_str(" AND symbol = ?");
            query_params.push(libsql::Value::Text(symbol.clone()));
        }

        if let Some(option_type) = &query.option_type {
            sql.push_str(" AND option_type = ?");
            query_params.push(libsql::Value::Text(option_type.to_string()));
        }

        if let Some(start_date) = query.start_date {
            sql.push_str(" AND entry_date >= ?");
            query_params.push(libsql::Value::Text(start_date.to_rfc3339()));
        }

        if let Some(end_date) = query.end_date {
            sql.push_str(" AND entry_date <= ?");
            query_params.push(libsql::Value::Text(end_date.to_rfc3339()));
        }

        // Convert time_range to start_date/end_date if provided
        if let Some(time_range) = &query.time_range {
            let (start, end) = time_range.to_dates();
            if let Some(start_date) = start {
                sql.push_str(" AND entry_date >= ?");
                query_params.push(libsql::Value::Text(start_date.to_rfc3339()));
            }
            if let Some(end_date) = end {
                sql.push_str(" AND entry_date <= ?");
                query_params.push(libsql::Value::Text(end_date.to_rfc3339()));
            }
        }

        sql.push_str(" ORDER BY entry_date DESC");

        // Add pagination
        if let Some(limit) = query.limit {
            sql.push_str(" LIMIT ?");
            query_params.push(libsql::Value::Integer(limit));
        }

        if let Some(offset) = query.offset {
            sql.push_str(" OFFSET ?");
            query_params.push(libsql::Value::Integer(offset));
        }

        let mut rows = conn
            .prepare(&sql)
            .await?
            .query(libsql::params_from_iter(query_params))
            .await?;

        let mut open_trades = Vec::new();
        while let Some(row) = rows.next().await? {
            let symbol: String = row.get(0)?;
            let entry_price = Self::get_f64(&row, 1)?;
            let entry_date_str: String = row.get(2)?;
            let entry_date = DateTime::parse_from_rfc3339(&entry_date_str)
                .map_err(|e| format!("Failed to parse entry_date: {}", e))?
                .with_timezone(&Utc);

            open_trades.push(OpenOptionTrade {
                symbol,
                entry_price,
                entry_date,
            });
        }

        Ok(open_trades)
    }

    /// Update an option trade
    /// Update an option trade
    pub async fn update(
        conn: &Connection,
        option_id: i64,
        request: UpdateOptionRequest,
    ) -> Result<Option<OptionTrade>, Box<dyn std::error::Error + Send + Sync>> {
        log::info!("=== Starting update for option_id: {} ===", option_id);
        log::info!("Update request: {:?}", request);

        // Check if option exists first
        let current_option = Self::find_by_id(conn, option_id).await?;

        if current_option.is_none() {
            log::warn!("Option {} not found", option_id);
            return Ok(None);
        }

        let now = Utc::now().to_rfc3339();
        log::info!("Generated timestamp: {}", now);

        log::info!("Preparing UPDATE query...");
        let mut rows = conn
            .prepare(
                r#"
                UPDATE options SET
                    symbol = COALESCE(?, symbol),
                    option_type = COALESCE(?, option_type),
                    strike_price = COALESCE(?, strike_price),
                    expiration_date = COALESCE(?, expiration_date),
                    entry_price = COALESCE(?, entry_price),
                    exit_price = COALESCE(?, exit_price),
                    premium = COALESCE(?, premium),
                    entry_date = COALESCE(?, entry_date),
                    exit_date = COALESCE(?, exit_date),
                    status = COALESCE(?, status),
                    initial_target = COALESCE(?, initial_target),
                    profit_target = COALESCE(?, profit_target),
                    trade_ratings = COALESCE(?, trade_ratings),
                    reviewed = COALESCE(?, reviewed),
                    mistakes = COALESCE(?, mistakes),
                    brokerage_name = COALESCE(?, brokerage_name),
                    trade_group_id = COALESCE(?, trade_group_id),
                    parent_trade_id = COALESCE(?, parent_trade_id),
                    total_quantity = COALESCE(?, total_quantity),
                    transaction_sequence = COALESCE(?, transaction_sequence),
                    updated_at = ?
                WHERE id = ?
                RETURNING id, symbol, option_type, strike_price, expiration_date, entry_price, exit_price,
                         premium, entry_date, exit_date, status, initial_target, profit_target, trade_ratings,
                         reviewed, mistakes, brokerage_name, trade_group_id, parent_trade_id, total_quantity,
                         transaction_sequence, created_at, updated_at, is_deleted
                "#,
            )
            .await?
            .query(params![
                request.symbol,
                request.option_type.map(|t| t.to_string()),
                request.strike_price,
                request.expiration_date.map(|d| d.to_rfc3339()),
                request.entry_price,
                request.exit_price,
                request.premium,
                request.entry_date.map(|d| d.to_rfc3339()),
                request.exit_date.map(|d| d.to_rfc3339()),
                request.status.map(|t| t.to_string()),
                request.initial_target,
                request.profit_target,
                request.trade_ratings,
                request.reviewed,
                request.mistakes,
                request.brokerage_name,
                request.trade_group_id,
                request.parent_trade_id,
                request.total_quantity,
                request.transaction_sequence,
                now,
                option_id
            ])
            .await?;

        log::info!("Query executed successfully, fetching results...");

        if let Some(row) = rows.next().await? {
            log::info!("Row returned from UPDATE query");

            // Log each column value
            for i in 0..24 {
                match row.get::<libsql::Value>(i) {
                    Ok(val) => log::info!("Column {}: {:?}", i, val),
                    Err(e) => log::warn!("Column {} error: {}", i, e),
                }
            }

            log::info!("Attempting to parse row...");
            match OptionTrade::from_row(&row) {
                Ok(option) => {
                    log::info!("Successfully parsed option with id: {}", option.id);
                    Ok(Some(option))
                }
                Err(e) => {
                    log::error!("Failed to parse row: {}", e);
                    Err(e)
                }
            }
        } else {
            log::warn!("No row returned from UPDATE query");
            Ok(None)
        }
    }

    /// Delete an option trade
    pub async fn delete(
        conn: &Connection,
        option_id: i64,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let result = conn
            .execute("DELETE FROM options WHERE id = ?", params![option_id])
            .await?;

        Ok(result > 0)
    }

    /// Get total count of options (for pagination)
    pub async fn count(
        conn: &Connection,
        query: &OptionQuery,
    ) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
        let mut sql = String::from("SELECT COUNT(*) FROM options WHERE 1=1");
        let mut query_params = Vec::new();

        // Add the same filters as in find_all
        if let Some(symbol) = &query.symbol {
            sql.push_str(" AND symbol = ?");
            query_params.push(libsql::Value::Text(symbol.clone()));
        }

        if let Some(option_type) = &query.option_type {
            sql.push_str(" AND option_type = ?");
            query_params.push(libsql::Value::Text(option_type.to_string()));
        }

        if let Some(trade_group_id) = &query.trade_group_id {
            sql.push_str(" AND trade_group_id = ?");
            query_params.push(libsql::Value::Text(trade_group_id.clone()));
        }

        if let Some(parent_trade_id) = query.parent_trade_id {
            sql.push_str(" AND parent_trade_id = ?");
            query_params.push(libsql::Value::Integer(parent_trade_id));
        }

        if let Some(status) = &query.status {
            sql.push_str(" AND status = ?");
            query_params.push(libsql::Value::Text(status.to_string()));
        }

        if let Some(start_date) = query.start_date {
            sql.push_str(" AND entry_date >= ?");
            query_params.push(libsql::Value::Text(start_date.to_rfc3339()));
        }

        if let Some(end_date) = query.end_date {
            sql.push_str(" AND entry_date <= ?");
            query_params.push(libsql::Value::Text(end_date.to_rfc3339()));
        }

        let mut rows = conn
            .prepare(&sql)
            .await?
            .query(libsql::params_from_iter(query_params))
            .await?;

        if let Some(row) = rows.next().await? {
            Ok(row.get::<i64>(0)?)
        } else {
            Ok(0)
        }
    }

    /// Calculate total P&L for all options in the user's database
    pub async fn calculate_total_pnl(
        conn: &Connection,
    ) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        let mut rows = conn
            .prepare(
                r#"
                SELECT SUM(
                    CASE
                        WHEN exit_price IS NOT NULL THEN
                            (exit_price - entry_price) * COALESCE(total_quantity, 0) * 100
                        ELSE 0
                    END
                ) as total_pnl
                FROM options
                "#,
            )
            .await?
            .query(params![])
            .await?;

        if let Some(row) = rows.next().await? {
            Ok(row.get::<Option<f64>>(0)?.unwrap_or(0.0))
        } else {
            Ok(0.0)
        }
    }

    /// Calculate profit factor (gross profit / gross loss)
    pub async fn calculate_profit_factor(
        conn: &Connection,
        time_range: TimeRange,
    ) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        let (time_condition, time_params) = time_range.to_sql_condition();

        let sql = format!(
            r#"
            WITH trade_profits AS (
                SELECT
                    (exit_price - entry_price) * COALESCE(total_quantity, 0) * 100 AS profit
                FROM options
                WHERE exit_date IS NOT NULL
                  AND exit_price IS NOT NULL
                  AND status = 'closed'
                  AND ({})
            ),
            profit_metrics AS (
                SELECT
                    SUM(CASE WHEN profit > 0 THEN profit ELSE 0 END) AS gross_profit,
                    ABS(SUM(CASE WHEN profit < 0 THEN profit ELSE 0 END)) AS gross_loss,
                    COUNT(*) AS total_trades
                FROM trade_profits
            )
            SELECT
                CASE
                    WHEN total_trades = 0 THEN 0
                    WHEN gross_loss = 0 AND gross_profit > 0 THEN 999.99
                    WHEN gross_loss = 0 THEN 0
                    ELSE ROUND(gross_profit / gross_loss, 2)
                END AS profit_factor
            FROM profit_metrics
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
            Ok(row.get::<Option<f64>>(0)?.unwrap_or(0.0))
        } else {
            Ok(0.0)
        }
    }

    /// Calculate win rate percentage
    pub async fn calculate_win_rate(
        conn: &Connection,
        time_range: TimeRange,
    ) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        let (time_condition, time_params) = time_range.to_sql_condition();

        let sql = format!(
            r#"
            WITH trade_results AS (
                SELECT
                    CASE
                        WHEN (exit_price - entry_price) > 0 THEN 1
                        ELSE 0
                    END AS is_winning_trade
                FROM options
                WHERE exit_date IS NOT NULL
                  AND exit_price IS NOT NULL
                  AND status = 'closed'
                  AND ({})
            ),
            win_rate_stats AS (
                SELECT
                    COUNT(*) AS total_trades,
                    SUM(is_winning_trade) AS winning_trades
                FROM trade_results
            )
            SELECT
                CASE
                    WHEN total_trades = 0 THEN 0
                    ELSE ROUND((CAST(winning_trades AS REAL) / CAST(total_trades AS REAL)) * 100, 2)
                END AS win_rate_percentage
            FROM win_rate_stats
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
            Ok(row.get::<Option<f64>>(0)?.unwrap_or(0.0))
        } else {
            Ok(0.0)
        }
    }

    /// Calculate average gain for winning trades
    pub async fn calculate_avg_gain(
        conn: &Connection,
        time_range: TimeRange,
    ) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        let (time_condition, time_params) = time_range.to_sql_condition();

        let sql = format!(
            r#"
            SELECT
                COALESCE(ROUND(AVG(
                    (exit_price - entry_price) * COALESCE(total_quantity, 0) * 100
                ), 2), 0) as avg_gain
            FROM options
            WHERE exit_date IS NOT NULL
              AND exit_price IS NOT NULL
              AND status = 'closed'
              AND (exit_price - entry_price) > 0
              AND ({})
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
            Ok(row.get::<Option<f64>>(0)?.unwrap_or(0.0))
        } else {
            Ok(0.0)
        }
    }

    /// Calculate average loss for losing trades
    pub async fn calculate_avg_loss(
        conn: &Connection,
        time_range: TimeRange,
    ) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        let (time_condition, time_params) = time_range.to_sql_condition();

        let sql = format!(
            r#"
            SELECT
                COALESCE(ROUND(AVG(
                    (exit_price - entry_price) * COALESCE(total_quantity, 0) * 100
                ), 2), 0) as avg_loss
            FROM options
            WHERE exit_date IS NOT NULL
              AND exit_price IS NOT NULL
              AND status = 'closed'
              AND (exit_price - entry_price) < 0
              AND ({})
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
            Ok(row.get::<Option<f64>>(0)?.unwrap_or(0.0))
        } else {
            Ok(0.0)
        }
    }

    /// Get biggest winner (highest profit trade)
    pub async fn calculate_biggest_winner(
        conn: &Connection,
        time_range: TimeRange,
    ) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        let (time_condition, time_params) = time_range.to_sql_condition();

        let sql = format!(
            r#"
            SELECT
                COALESCE(MAX(
                    (exit_price - entry_price) * COALESCE(total_quantity, 0) * 100
                ), 0) as biggest_winner
            FROM options
            WHERE exit_date IS NOT NULL
              AND exit_price IS NOT NULL
              AND status = 'closed'
              AND (exit_price - entry_price) > 0
              AND ({})
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
            Ok(row.get::<Option<f64>>(0)?.unwrap_or(0.0))
        } else {
            Ok(0.0)
        }
    }

    /// Get biggest loser (largest loss trade)
    pub async fn calculate_biggest_loser(
        conn: &Connection,
        time_range: TimeRange,
    ) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        let (time_condition, time_params) = time_range.to_sql_condition();

        let sql = format!(
            r#"
            SELECT
                COALESCE(MIN(
                    (exit_price - entry_price) * COALESCE(total_quantity, 0) * 100
                ), 0) as biggest_loser
            FROM options
            WHERE exit_date IS NOT NULL
              AND exit_price IS NOT NULL
              AND status = 'closed'
              AND (exit_price - entry_price) < 0
              AND ({})
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
            Ok(row.get::<Option<f64>>(0)?.unwrap_or(0.0))
        } else {
            Ok(0.0)
        }
    }

    /// Calculate average hold time for winning trades (in days)
    pub async fn calculate_avg_hold_time_winners(
        conn: &Connection,
        time_range: TimeRange,
    ) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        let (time_condition, time_params) = time_range.to_sql_condition();

        let sql = format!(
            r#"
            SELECT
                COALESCE(ROUND(AVG(
                    (julianday(exit_date) - julianday(entry_date))
                ), 2), 0) as avg_hold_days
            FROM options
            WHERE exit_date IS NOT NULL
              AND exit_price IS NOT NULL
              AND status = 'closed'
              AND (exit_price - entry_price) > 0
              AND ({})
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
            Ok(row.get::<Option<f64>>(0)?.unwrap_or(0.0))
        } else {
            Ok(0.0)
        }
    }

    /// Calculate average hold time for losing trades (in days)
    pub async fn calculate_avg_hold_time_losers(
        conn: &Connection,
        time_range: TimeRange,
    ) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        let (time_condition, time_params) = time_range.to_sql_condition();

        let sql = format!(
            r#"
            SELECT
                COALESCE(ROUND(AVG(
                    (julianday(exit_date) - julianday(entry_date))
                ), 2), 0) as avg_hold_days
            FROM options
            WHERE exit_date IS NOT NULL
              AND exit_price IS NOT NULL
              AND status = 'closed'
              AND (exit_price - entry_price) < 0
              AND ({})
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
            Ok(row.get::<Option<f64>>(0)?.unwrap_or(0.0))
        } else {
            Ok(0.0)
        }
    }

    /// Calculate risk-reward ratio
    pub async fn calculate_risk_reward_ratio(
        conn: &Connection,
        time_range: TimeRange,
    ) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        let avg_gain = Self::calculate_avg_gain(conn, time_range.clone()).await?;
        let avg_loss = Self::calculate_avg_loss(conn, time_range).await?;

        if avg_loss == 0.0 {
            Ok(0.0)
        } else {
            Ok((avg_gain / avg_loss.abs()).round())
        }
    }

    /// Calculate trade expectancy
    pub async fn calculate_trade_expectancy(
        conn: &Connection,
        time_range: TimeRange,
    ) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        let win_rate = Self::calculate_win_rate(conn, time_range.clone()).await?;
        let avg_gain = Self::calculate_avg_gain(conn, time_range.clone()).await?;
        let avg_loss = Self::calculate_avg_loss(conn, time_range).await?;

        let win_rate_decimal = win_rate / 100.0; // Convert percentage to decimal
        let loss_rate_decimal = 1.0 - win_rate_decimal;

        let expectancy = (win_rate_decimal * avg_gain) + (loss_rate_decimal * avg_loss);
        Ok((expectancy * 100.0).round() / 100.0)
    }

    /// Calculate average position size (premium paid)
    pub async fn calculate_avg_position_size(
        conn: &Connection,
        time_range: TimeRange,
    ) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        let (time_condition, time_params) = time_range.to_sql_condition();

        let sql = format!(
            r#"
            SELECT
                COALESCE(ROUND(AVG(premium), 2), 0) as avg_position_size
            FROM options
            WHERE status = 'closed'
              AND ({})
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
            Ok(row.get::<Option<f64>>(0)?.unwrap_or(0.0))
        } else {
            Ok(0.0)
        }
    }

    /// Calculate loss rate percentage
    pub async fn calculate_loss_rate(
        conn: &Connection,
        time_range: TimeRange,
    ) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        let win_rate = Self::calculate_win_rate(conn, time_range).await?;
        Ok((100.0 - win_rate).round())
    }

    /// Calculate net P&L for the time range
    pub async fn calculate_net_pnl(
        conn: &Connection,
        time_range: TimeRange,
    ) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        let (time_condition, time_params) = time_range.to_sql_condition();

        let sql = format!(
            r#"
            SELECT
                COALESCE(SUM(
                    CASE
                        WHEN exit_price IS NOT NULL THEN
                            (exit_price - entry_price) * COALESCE(total_quantity, 0) * 100
                        ELSE -premium * COALESCE(total_quantity, 1)  -- Unrealized loss for open positions (premium per contract * number of contracts)
                    END
                ), 0) as net_pnl
            FROM options
            WHERE ({})
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
            Ok(row.get::<Option<f64>>(0)?.unwrap_or(0.0))
        } else {
            Ok(0.0)
        }
    }

    /// Get playbook setups associated with this option trade
    #[allow(dead_code)]
    pub async fn get_playbooks(
        &self,
        conn: &Connection,
    ) -> Result<Vec<crate::models::playbook::Playbook>, Box<dyn std::error::Error + Send + Sync>>
    {
        crate::models::playbook::Playbook::get_option_trade_playbooks(conn, self.id).await
    }

    /// Tag this option trade with a playbook setup
    #[allow(dead_code)]
    pub async fn tag_with_playbook(
        &self,
        conn: &Connection,
        setup_id: &str,
    ) -> Result<
        crate::models::playbook::OptionTradePlaybook,
        Box<dyn std::error::Error + Send + Sync>,
    > {
        crate::models::playbook::Playbook::tag_option_trade(conn, self.id, setup_id).await
    }

    /// Remove a playbook tag from this option trade
    #[allow(dead_code)]
    pub async fn untag_playbook(
        &self,
        conn: &Connection,
        setup_id: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        crate::models::playbook::Playbook::untag_option_trade(conn, self.id, setup_id).await
    }

    /// Convert from libsql row to OptionTrade struct
    /// Column order: id, symbol, option_type, strike_price, expiration_date, entry_price, exit_price,
    /// premium, entry_date, exit_date, status, initial_target, profit_target, trade_ratings,
    /// reviewed, mistakes, brokerage_name, trade_group_id, parent_trade_id, total_quantity,
    /// transaction_sequence, created_at, updated_at, is_deleted
    fn from_row(
        row: &libsql::Row,
    ) -> Result<OptionTrade, Box<dyn std::error::Error + Send + Sync>> {
        // Get enum strings with fallback
        let option_type_str = match row.get::<libsql::Value>(2) {
            Ok(libsql::Value::Text(s)) => s,
            Ok(libsql::Value::Null) | Err(_) => "Call".to_string(),
            Ok(_) => "Call".to_string(),
        };

        let status_str = match row.get::<libsql::Value>(10) {
            Ok(libsql::Value::Text(s)) => s,
            Ok(libsql::Value::Null) | Err(_) => "open".to_string(),
            Ok(_) => "open".to_string(),
        };

        let option_type = option_type_str
            .parse::<OptionType>()
            .map_err(|e| format!("Invalid option type: {}", e))?;

        let status = status_str
            .parse::<TradeStatus>()
            .map_err(|e| format!("Invalid trade status: {}", e))?;

        // Parse datetime strings
        let expiration_date_str = match row.get::<libsql::Value>(4) {
            Ok(libsql::Value::Text(s)) => s,
            _ => return Err("Failed to get expiration_date".into()),
        };

        let entry_date_str = match row.get::<libsql::Value>(8) {
            Ok(libsql::Value::Text(s)) => s,
            _ => return Err("Failed to get entry_date".into()),
        };

        // Handle reviewed field
        let reviewed_val: Option<i64> = match row.get::<libsql::Value>(14) {
            Ok(libsql::Value::Integer(val)) => Some(val),
            Ok(libsql::Value::Null) => None,
            Ok(libsql::Value::Real(val)) => Some(val as i64),
            _ => None,
        };
        let reviewed = reviewed_val.map(|v| v != 0).unwrap_or(false);

        let mistakes_str: Option<String> = match row.get::<libsql::Value>(15) {
            Ok(libsql::Value::Text(s)) => Some(s),
            Ok(libsql::Value::Null) => None,
            _ => None,
        };

        let brokerage_name: Option<String> = match row.get::<libsql::Value>(16) {
            Ok(libsql::Value::Text(s)) => Some(s),
            Ok(libsql::Value::Null) => None,
            _ => None,
        };

        let trade_group_id: Option<String> = match row.get::<libsql::Value>(17) {
            Ok(libsql::Value::Text(s)) => Some(s),
            Ok(libsql::Value::Null) => None,
            _ => None,
        };

        let parent_trade_id: Option<i64> = match row.get::<libsql::Value>(18) {
            Ok(libsql::Value::Integer(val)) => Some(val),
            Ok(libsql::Value::Null) => None,
            Ok(libsql::Value::Real(val)) => Some(val as i64),
            _ => None,
        };

        let created_at_str = match row.get::<libsql::Value>(21) {
            Ok(libsql::Value::Text(s)) => s,
            Ok(val) => {
                log::error!("created_at unexpected type: {:?}", val);
                return Err("Failed to get created_at".into());
            }
            Err(e) => {
                log::error!("created_at error: {}", e);
                return Err("Failed to get created_at".into());
            }
        };

        let updated_at_str = match row.get::<libsql::Value>(22) {
            Ok(libsql::Value::Text(s)) => s,
            Ok(val) => {
                log::error!("updated_at unexpected type: {:?}", val);
                return Err(format!("Failed to get updated_at: unexpected type {:?}", val).into());
            }
            Err(e) => {
                log::error!("updated_at error: {}", e);
                return Err(format!("Failed to get updated_at: {}", e).into());
            }
        };

        // Handle is_deleted field
        let is_deleted = match row.get::<libsql::Value>(23) {
            Ok(libsql::Value::Integer(val)) => val != 0,
            Ok(libsql::Value::Null) => false,
            _ => false,
        };

        // Helper function to parse datetime
        let parse_datetime =
            |datetime_str: &str,
             field_name: &str|
             -> Result<DateTime<Utc>, Box<dyn std::error::Error + Send + Sync>> {
                if datetime_str.contains('T') {
                    Ok(DateTime::parse_from_rfc3339(datetime_str)
                        .map_err(|e| format!("Failed to parse {} as RFC3339: {}", field_name, e))?
                        .with_timezone(&Utc))
                } else {
                    Ok(
                        chrono::NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S")
                            .map_err(|e| {
                                format!("Failed to parse {} as SQLite datetime: {}", field_name, e)
                            })?
                            .and_utc(),
                    )
                }
            };

        let expiration_date = parse_datetime(&expiration_date_str, "expiration_date")?;
        let entry_date = parse_datetime(&entry_date_str, "entry_date")?;
        let exit_date_str: String = row.get(9)?;
        let exit_date = parse_datetime(&exit_date_str, "exit_date")?;
        let created_at = parse_datetime(&created_at_str, "created_at")?;
        let updated_at = parse_datetime(&updated_at_str, "updated_at")?;

        Ok(OptionTrade {
            id: match row.get::<libsql::Value>(0) {
                Ok(libsql::Value::Integer(val)) => val,
                _ => return Err("Failed to get id".into()),
            },
            symbol: match row.get::<libsql::Value>(1) {
                Ok(libsql::Value::Text(s)) => s,
                _ => return Err("Failed to get symbol".into()),
            },
            option_type,
            strike_price: Self::get_f64(row, 3)?,
            expiration_date,
            entry_price: Self::get_f64(row, 5)?,
            exit_price: Self::get_f64(row, 6)?,
            premium: Self::get_f64(row, 7)?,
            entry_date,
            exit_date,
            status,
            initial_target: Self::get_opt_f64(row, 11)?,
            profit_target: Self::get_opt_f64(row, 12)?,
            trade_ratings: match row.get::<libsql::Value>(13) {
                Ok(libsql::Value::Integer(val)) => Some(val as i32),
                Ok(libsql::Value::Null) => None,
                Ok(libsql::Value::Real(val)) => Some(val as i32),
                _ => None,
            },
            reviewed,
            mistakes: mistakes_str,
            brokerage_name,
            trade_group_id,
            parent_trade_id,
            total_quantity: Self::get_opt_f64(row, 19)?,
            transaction_sequence: match row.get::<libsql::Value>(20) {
                Ok(libsql::Value::Integer(val)) => Some(val as i32),
                Ok(libsql::Value::Null) => None,
                Ok(libsql::Value::Real(val)) => Some(val as i32),
                _ => None,
            },
            created_at,
            updated_at,
            is_deleted,
        })
    }
}
