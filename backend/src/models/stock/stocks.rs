use chrono::{DateTime, Utc};
use libsql::{Connection, params};
use serde::{Deserialize, Deserializer, Serialize};

/// Time range enum for calculations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TimeRange {
    #[serde(rename = "7d")]
    SevenDays,
    #[serde(rename = "30d")]
    ThirtyDays,
    #[serde(rename = "90d")]
    NinetyDays,
    #[serde(rename = "1y")]
    OneYear,
    #[serde(rename = "ytd")]
    YearToDate,
    #[serde(rename = "custom")]
    Custom {
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    },
    #[serde(rename = "all_time")]
    AllTime,
}

impl TimeRange {
    /// Convert TimeRange to SQL WHERE clause fragment
    pub fn to_sql_condition(&self) -> (String, Vec<DateTime<Utc>>) {
        match self {
            TimeRange::SevenDays => ("exit_date >= date('now', '-7 days')".to_string(), vec![]),
            TimeRange::ThirtyDays => ("exit_date >= date('now', '-30 days')".to_string(), vec![]),
            TimeRange::NinetyDays => ("exit_date >= date('now', '-90 days')".to_string(), vec![]),
            TimeRange::OneYear => ("exit_date >= date('now', '-1 year')".to_string(), vec![]),
            TimeRange::YearToDate => (
                "exit_date >= date('now', 'start of year')".to_string(),
                vec![],
            ),
            TimeRange::Custom {
                start_date,
                end_date,
            } => {
                let mut conditions = vec![];
                let mut params = vec![];

                if let Some(start) = start_date {
                    conditions.push("exit_date >= ?".to_string());
                    params.push(*start);
                }

                if let Some(end) = end_date {
                    conditions.push("exit_date <= ?".to_string());
                    params.push(*end);
                }

                if conditions.is_empty() {
                    ("1=1".to_string(), params)
                } else {
                    (conditions.join(" AND "), params)
                }
            }
            TimeRange::AllTime => ("1=1".to_string(), vec![]),
        }
    }

    /// Convert TimeRange to start_date and end_date for filtering by entry_date
    pub fn to_dates(&self) -> (Option<DateTime<Utc>>, Option<DateTime<Utc>>) {
        let now = Utc::now();

        match self {
            TimeRange::SevenDays => {
                let start = now - chrono::Duration::days(7);
                (Some(start), Some(now))
            }
            TimeRange::ThirtyDays => {
                let start = now - chrono::Duration::days(30);
                (Some(start), Some(now))
            }
            TimeRange::NinetyDays => {
                let start = now - chrono::Duration::days(90);
                (Some(start), Some(now))
            }
            TimeRange::OneYear => {
                let start = now - chrono::Duration::days(365);
                (Some(start), Some(now))
            }
            TimeRange::YearToDate => {
                let start = now.date_naive().and_hms_opt(1, 0, 0).unwrap().and_utc();
                (Some(start), Some(now))
            }
            TimeRange::Custom {
                start_date,
                end_date,
            } => (*start_date, *end_date),
            TimeRange::AllTime => (None, None),
        }
    }
}

/// Trade type enum matching the PostgreSQL enum in your schema
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
#[allow(clippy::upper_case_acronyms)]
pub enum TradeType {
    BUY,
    SELL,
}

impl std::fmt::Display for TradeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TradeType::BUY => write!(f, "BUY"),
            TradeType::SELL => write!(f, "SELL"),
        }
    }
}

impl std::str::FromStr for TradeType {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "BUY" => Ok(TradeType::BUY),
            "SELL" => Ok(TradeType::SELL),
            _ => Err("Invalid trade type"),
        }
    }
}

/// Order type enum matching the PostgreSQL enum in your schema
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
#[allow(clippy::upper_case_acronyms)]
pub enum OrderType {
    MARKET,
    LIMIT,
    STOP,
    #[serde(rename = "STOP_LIMIT")]
    StopLimit,
}

impl std::fmt::Display for OrderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderType::MARKET => write!(f, "MARKET"),
            OrderType::LIMIT => write!(f, "LIMIT"),
            OrderType::STOP => write!(f, "STOP"),
            OrderType::StopLimit => write!(f, "STOP_LIMIT"),
        }
    }
}

impl std::str::FromStr for OrderType {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "MARKET" => Ok(OrderType::MARKET),
            "LIMIT" => Ok(OrderType::LIMIT),
            "STOP" => Ok(OrderType::STOP),
            "STOP_LIMIT" => Ok(OrderType::StopLimit),
            _ => Err("Invalid order type"),
        }
    }
}

// Helper function to deserialize DateTime from various formats
fn deserialize_datetime<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    DateTime::parse_from_rfc3339(&s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(serde::de::Error::custom)
}

fn deserialize_optional_datetime<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    match opt {
        Some(s) => DateTime::parse_from_rfc3339(&s)
            .map(|dt| Some(dt.with_timezone(&Utc)))
            .map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}

/// Stock trade model for user's isolated database
/// No user_id needed since each user has their own database
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stock {
    pub id: i64,
    pub symbol: String,
    pub trade_type: TradeType,
    pub order_type: OrderType,
    pub entry_price: f64,
    pub exit_price: f64,
    pub stop_loss: f64,
    pub commissions: f64,
    pub number_shares: f64,
    pub take_profit: Option<f64>,
    pub initial_target: Option<f64>,
    pub profit_target: Option<f64>,
    pub trade_ratings: Option<i32>,
    pub entry_date: DateTime<Utc>,
    pub exit_date: DateTime<Utc>,
    pub reviewed: bool,
    pub mistakes: Option<String>,
    pub brokerage_name: Option<String>,
    pub trade_group_id: Option<String>,
    pub parent_trade_id: Option<i64>,
    pub total_quantity: Option<f64>,
    pub transaction_sequence: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Simplified response for open stock trades (only essential fields)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenStockTrade {
    pub symbol: String,
    pub entry_price: f64,
    pub entry_date: DateTime<Utc>,
}

/// Data Transfer Object for creating new stock trades
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateStockRequest {
    pub symbol: String,
    pub trade_type: TradeType,
    pub order_type: OrderType,
    pub entry_price: f64,
    pub exit_price: f64,
    pub stop_loss: f64,
    #[serde(default)] // Allow missing field, defaults to 0.0
    pub commissions: f64,
    pub number_shares: f64,
    pub take_profit: Option<f64>,
    pub initial_target: Option<f64>,
    pub profit_target: Option<f64>,
    pub trade_ratings: Option<i32>,
    #[serde(deserialize_with = "deserialize_datetime")]
    pub entry_date: DateTime<Utc>,
    #[serde(deserialize_with = "deserialize_datetime")]
    pub exit_date: DateTime<Utc>,
    #[serde(default)] // Allow missing field, defaults to false
    pub reviewed: Option<bool>,
    pub mistakes: Option<String>,
    pub brokerage_name: Option<String>,
    pub trade_group_id: Option<String>,
    pub parent_trade_id: Option<i64>,
    pub total_quantity: Option<f64>,
    pub transaction_sequence: Option<i32>,
}

/// Data Transfer Object for updating stock trades
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStockRequest {
    pub symbol: Option<String>,
    pub trade_type: Option<TradeType>,
    pub order_type: Option<OrderType>,
    pub entry_price: Option<f64>,
    pub exit_price: Option<f64>,
    pub stop_loss: Option<f64>,
    pub commissions: Option<f64>,
    pub number_shares: Option<f64>,
    pub take_profit: Option<f64>,
    pub initial_target: Option<f64>,
    pub profit_target: Option<f64>,
    pub trade_ratings: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_optional_datetime")]
    pub entry_date: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "deserialize_optional_datetime")]
    pub exit_date: Option<DateTime<Utc>>,
    pub reviewed: Option<bool>,
    pub mistakes: Option<String>,
    pub brokerage_name: Option<String>,
    pub trade_group_id: Option<String>,
    pub parent_trade_id: Option<i64>,
    pub total_quantity: Option<f64>,
    pub transaction_sequence: Option<i32>,
}

/// Stock query parameters for filtering and pagination
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockQuery {
    pub symbol: Option<String>,
    pub trade_type: Option<TradeType>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub updated_after: Option<DateTime<Utc>>,
    pub time_range: Option<TimeRange>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub open_only: Option<bool>,
    pub trade_group_id: Option<String>,
    pub parent_trade_id: Option<i64>,
}

/// Note to AI model: When creating an Helper function to handle different types of values from the database
// you need to get the value type first using libsql's value API and then use the appropriate method to get the value.
/// Stock operations implementation using LibSQL
impl Stock {
    fn get_f64(
        row: &libsql::Row,
        idx: usize,
    ) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        let i = idx as i32;
        match row.get_value(i) {
            Ok(libsql::Value::Null) => Ok(0.0),
            Ok(libsql::Value::Integer(n)) => Ok(n as f64),
            Ok(libsql::Value::Real(r)) => Ok(r),
            Ok(libsql::Value::Text(s)) => {
                let result = s.parse::<f64>().unwrap_or(0.0);
                Ok(result)
            }
            Ok(libsql::Value::Blob(_b)) => Ok(0.0),
            Err(_e) => Ok(0.0),
        }
    }

    fn get_opt_f64(
        row: &libsql::Row,
        idx: usize,
    ) -> Result<Option<f64>, Box<dyn std::error::Error + Send + Sync>> {
        let i = idx as i32;
        match row.get_value(i) {
            Ok(libsql::Value::Null) => Ok(None),
            Ok(libsql::Value::Integer(n)) => Ok(Some(n as f64)),
            Ok(libsql::Value::Real(r)) => Ok(Some(r)),
            Ok(libsql::Value::Text(s)) => {
                let result = s.parse::<f64>().ok();
                Ok(result)
            }
            Ok(libsql::Value::Blob(_b)) => Ok(None),
            Err(_e) => Ok(None),
        }
    }

    fn get_string(
        row: &libsql::Row,
        idx: usize,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let i = idx as i32;
        match row.get_value(i) {
            Ok(libsql::Value::Text(s)) => Ok(s),
            Ok(libsql::Value::Null) => Ok(String::new()),
            Ok(libsql::Value::Integer(n)) => Ok(n.to_string()),
            Ok(libsql::Value::Real(r)) => Ok(r.to_string()),
            Ok(libsql::Value::Blob(_b)) => Ok(String::new()),
            Err(_e) => Ok(String::new()),
        }
    }

    fn get_opt_string(
        row: &libsql::Row,
        idx: usize,
    ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        let i = idx as i32;
        match row.get_value(i) {
            Ok(libsql::Value::Text(s)) => Ok(Some(s)),
            Ok(libsql::Value::Null) => Ok(None),
            Ok(libsql::Value::Integer(n)) => Ok(Some(n.to_string())),
            Ok(libsql::Value::Real(r)) => Ok(Some(r.to_string())),
            Ok(libsql::Value::Blob(_b)) => Ok(None),
            Err(_e) => Ok(None),
        }
    }

    fn get_i64(
        row: &libsql::Row,
        idx: usize,
    ) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
        let i = idx as i32;
        match row.get_value(i) {
            Ok(libsql::Value::Integer(n)) => Ok(n),
            Ok(libsql::Value::Null) => Ok(0),
            Ok(libsql::Value::Real(r)) => Ok(r as i64),
            Ok(libsql::Value::Text(s)) => {
                let result = s.parse::<i64>().unwrap_or(0);
                Ok(result)
            }
            Ok(libsql::Value::Blob(_b)) => Ok(0),
            Err(_e) => Ok(0),
        }
    }

    fn get_opt_i64(
        row: &libsql::Row,
        idx: usize,
    ) -> Result<Option<i64>, Box<dyn std::error::Error + Send + Sync>> {
        let i = idx as i32;
        match row.get_value(i) {
            Ok(libsql::Value::Integer(n)) => Ok(Some(n)),
            Ok(libsql::Value::Null) => Ok(None),
            Ok(libsql::Value::Real(r)) => Ok(Some(r as i64)),
            Ok(libsql::Value::Text(s)) => {
                let result = s.parse::<i64>().ok();
                Ok(result)
            }
            Ok(libsql::Value::Blob(_b)) => Ok(None),
            Err(_e) => Ok(None),
        }
    }

    fn get_opt_i32(
        row: &libsql::Row,
        idx: usize,
    ) -> Result<Option<i32>, Box<dyn std::error::Error + Send + Sync>> {
        let i = idx as i32;
        match row.get_value(i) {
            Ok(libsql::Value::Integer(n)) => Ok(Some(n as i32)),
            Ok(libsql::Value::Null) => Ok(None),
            Ok(libsql::Value::Real(r)) => Ok(Some(r as i32)),
            Ok(libsql::Value::Text(s)) => {
                let result = s.parse::<i32>().ok();
                Ok(result)
            }
            Ok(libsql::Value::Blob(_b)) => Ok(None),
            Err(_e) => Ok(None),
        }
    }

    fn get_bool(
        row: &libsql::Row,
        idx: usize,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let i = idx as i32;
        // Try i64 (SQLite INTEGER for boolean)
        if let Ok(v) = row.get::<i64>(i) {
            return Ok(v != 0);
        }
        // Try Option<i64>
        if let Ok(Some(v)) = row.get::<Option<i64>>(i) {
            return Ok(v != 0);
        }
        // Try i32
        if let Ok(v) = row.get::<i32>(i) {
            return Ok(v != 0);
        }
        // Try Option<i32>
        if let Ok(Some(v)) = row.get::<Option<i32>>(i) {
            return Ok(v != 0);
        }
        // Try String (in case stored as "true"/"false" or "1"/"0")
        if let Ok(s) = row.get::<String>(i) {
            let s_lower = s.to_lowercase();
            if s_lower == "true" || s_lower == "1" {
                return Ok(true);
            }
            if s_lower == "false" || s_lower == "0" {
                return Ok(false);
            }
            // Try parsing as number
            if let Ok(num) = s.parse::<i64>() {
                return Ok(num != 0);
            }
        }
        // Fallback to false
        Ok(false)
    }
    /// Create a new stock trade in the user's database
    pub async fn create(
        conn: &Connection,
        request: CreateStockRequest,
    ) -> Result<Stock, Box<dyn std::error::Error + Send + Sync>> {
        let now = Utc::now().to_rfc3339();

        let mut rows = conn
            .prepare(
                r#"
            INSERT INTO stocks (
                symbol, trade_type, order_type, entry_price, exit_price,
                stop_loss, commissions, number_shares, take_profit, 
                initial_target, profit_target, trade_ratings,
                entry_date, exit_date, reviewed, mistakes, brokerage_name, trade_group_id,
                parent_trade_id, total_quantity, transaction_sequence, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id, symbol, trade_type, order_type, entry_price,
                     exit_price, stop_loss, commissions, number_shares, take_profit,
                     initial_target, profit_target, trade_ratings,
                     entry_date, exit_date, reviewed, mistakes, brokerage_name,
                     trade_group_id, parent_trade_id, total_quantity, transaction_sequence,
                     created_at, updated_at
            "#,
            )
            .await?
            .query(params![
                request.symbol,
                request.trade_type.to_string(),
                request.order_type.to_string(),
                request.entry_price,
                request.exit_price,
                request.stop_loss,
                request.commissions,
                request.number_shares,
                request.take_profit,
                request.initial_target,
                request.profit_target,
                request.trade_ratings,
                request.entry_date.to_rfc3339(),
                request.exit_date.to_rfc3339(),
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
            Ok(Stock::from_row(&row)?)
        } else {
            Err("Failed to create stock trade".into())
        }
    }

    /// Find a stock trade by ID in the user's database
    pub async fn find_by_id(
        conn: &Connection,
        stock_id: i64,
    ) -> Result<Option<Stock>, Box<dyn std::error::Error + Send + Sync>> {
        let mut rows = conn
            .prepare(
                r#"
            SELECT id, symbol, trade_type, order_type, entry_price,
                   exit_price, stop_loss, commissions, number_shares, take_profit,
                   initial_target, profit_target, trade_ratings,
                   entry_date, exit_date, reviewed, mistakes, brokerage_name,
                   trade_group_id, parent_trade_id, total_quantity, transaction_sequence,
                   created_at, updated_at
            FROM stocks 
            WHERE id = ?
            "#,
            )
            .await?
            .query(params![stock_id])
            .await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(Stock::from_row(&row)?))
        } else {
            Ok(None)
        }
    }

    /// Find all stock trades with optional filtering
    pub async fn find_all(
        conn: &Connection,
        query: StockQuery,
    ) -> Result<Vec<Stock>, Box<dyn std::error::Error + Send + Sync>> {
        let mut sql = String::from(
            r#"
            SELECT id, symbol, trade_type, order_type, entry_price,
                   exit_price, stop_loss, commissions, number_shares, take_profit,
                   initial_target, profit_target, trade_ratings,
                   entry_date, exit_date, reviewed, mistakes, brokerage_name,
                   trade_group_id, parent_trade_id, total_quantity, transaction_sequence,
                   created_at, updated_at
            FROM stocks 
            WHERE 1=1
            "#,
        );

        let mut query_params = Vec::new();

        // Add optional filters
        if let Some(symbol) = &query.symbol {
            sql.push_str(" AND symbol = ?");
            query_params.push(libsql::Value::Text(symbol.clone()));
        }

        if let Some(trade_type) = &query.trade_type {
            sql.push_str(" AND trade_type = ?");
            query_params.push(libsql::Value::Text(trade_type.to_string()));
        }

        if let Some(trade_group_id) = &query.trade_group_id {
            sql.push_str(" AND trade_group_id = ?");
            query_params.push(libsql::Value::Text(trade_group_id.clone()));
        }

        if let Some(parent_trade_id) = query.parent_trade_id {
            sql.push_str(" AND parent_trade_id = ?");
            query_params.push(libsql::Value::Integer(parent_trade_id));
        }

        if let Some(start_date) = query.start_date {
            sql.push_str(" AND entry_date >= ?");
            query_params.push(libsql::Value::Text(start_date.to_rfc3339()));
        }

        if let Some(end_date) = query.end_date {
            sql.push_str(" AND entry_date <= ?");
            query_params.push(libsql::Value::Text(end_date.to_rfc3339()));
        }

        if let Some(updated_after) = query.updated_after {
            sql.push_str(" AND updated_at >= ?");
            query_params.push(libsql::Value::Text(updated_after.to_rfc3339()));
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
                sql.push_str(" AND exit_date IS NULL");
            } else {
                sql.push_str(" AND exit_date IS NOT NULL");
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

        let mut stocks = Vec::new();
        while let Some(row) = rows.next().await? {
            stocks.push(Stock::from_row(&row)?);
        }

        Ok(stocks)
    }

    /// Find all open stock trades with simplified response (only symbol, entry_price, entry_date)
    pub async fn find_all_open_summary(
        conn: &Connection,
        query: StockQuery,
    ) -> Result<Vec<OpenStockTrade>, Box<dyn std::error::Error + Send + Sync>> {
        let mut sql = String::from(
            r#"
            SELECT symbol, entry_price, entry_date
            FROM stocks 
            WHERE exit_date IS NULL
            "#,
        );

        let mut query_params = Vec::new();

        // Add optional filters (excluding open_only since we're already filtering for open)
        if let Some(symbol) = &query.symbol {
            sql.push_str(" AND symbol = ?");
            query_params.push(libsql::Value::Text(symbol.clone()));
        }

        if let Some(trade_type) = &query.trade_type {
            sql.push_str(" AND trade_type = ?");
            query_params.push(libsql::Value::Text(trade_type.to_string()));
        }

        if let Some(start_date) = query.start_date {
            sql.push_str(" AND entry_date >= ?");
            query_params.push(libsql::Value::Text(start_date.to_rfc3339()));
        }

        if let Some(end_date) = query.end_date {
            sql.push_str(" AND entry_date <= ?");
            query_params.push(libsql::Value::Text(end_date.to_rfc3339()));
        }

        if let Some(updated_after) = query.updated_after {
            sql.push_str(" AND updated_at >= ?");
            query_params.push(libsql::Value::Text(updated_after.to_rfc3339()));
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

            open_trades.push(OpenStockTrade {
                symbol,
                entry_price,
                entry_date,
            });
        }

        Ok(open_trades)
    }

    /// Update a stock trade
    pub async fn update(
        conn: &Connection,
        stock_id: i64,
        request: UpdateStockRequest,
    ) -> Result<Option<Stock>, Box<dyn std::error::Error + Send + Sync>> {
        // Check if stock exists first
        let current_stock = Self::find_by_id(conn, stock_id).await?;

        if current_stock.is_none() {
            return Ok(None);
        }

        let now = Utc::now().to_rfc3339();

        let mut rows = conn
            .prepare(
                r#"
            UPDATE stocks SET 
                symbol = COALESCE(?, symbol),
                trade_type = COALESCE(?, trade_type),
                order_type = COALESCE(?, order_type),
                entry_price = COALESCE(?, entry_price),
                exit_price = COALESCE(?, exit_price),
                stop_loss = COALESCE(?, stop_loss),
                commissions = COALESCE(?, commissions),
                number_shares = COALESCE(?, number_shares),
                take_profit = COALESCE(?, take_profit),
                initial_target = COALESCE(?, initial_target),
                profit_target = COALESCE(?, profit_target),
                trade_ratings = COALESCE(?, trade_ratings),
                entry_date = COALESCE(?, entry_date),
                exit_date = COALESCE(?, exit_date),
                reviewed = COALESCE(?, reviewed),
                mistakes = COALESCE(?, mistakes),
                brokerage_name = COALESCE(?, brokerage_name),
                trade_group_id = COALESCE(?, trade_group_id),
                parent_trade_id = COALESCE(?, parent_trade_id),
                total_quantity = COALESCE(?, total_quantity),
                transaction_sequence = COALESCE(?, transaction_sequence),
                updated_at = ?
            WHERE id = ?
            RETURNING id, symbol, trade_type, order_type, entry_price,
                     exit_price, stop_loss, commissions, number_shares, take_profit,
                     initial_target, profit_target, trade_ratings,
                     entry_date, exit_date, reviewed, mistakes, brokerage_name,
                     trade_group_id, parent_trade_id, total_quantity, transaction_sequence,
                     created_at, updated_at
            "#,
            )
            .await?
            .query(params![
                request.symbol,
                request.trade_type.map(|t| t.to_string()),
                request.order_type.map(|t| t.to_string()),
                request.entry_price,
                request.exit_price,
                request.stop_loss,
                request.commissions,
                request.number_shares,
                request.take_profit,
                request.initial_target,
                request.profit_target,
                request.trade_ratings,
                request.entry_date.map(|d| d.to_rfc3339()),
                request.exit_date.map(|d| d.to_rfc3339()),
                request.reviewed,
                request.mistakes,
                request.brokerage_name,
                request.trade_group_id,
                request.parent_trade_id,
                request.total_quantity,
                request.transaction_sequence,
                now,
                stock_id
            ])
            .await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(Stock::from_row(&row)?))
        } else {
            Ok(None)
        }
    }

    /// Delete a stock trade
    pub async fn delete(
        conn: &Connection,
        stock_id: i64,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let result = conn
            .execute("DELETE FROM stocks WHERE id = ?", params![stock_id])
            .await?;

        Ok(result > 0)
    }

    /// Get total count of stocks (for pagination)
    pub async fn count(
        conn: &Connection,
        query: &StockQuery,
    ) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
        let mut sql = String::from("SELECT COUNT(*) FROM stocks WHERE 1=1");
        let mut query_params = Vec::new();

        // Add the same filters as in find_all
        if let Some(symbol) = &query.symbol {
            sql.push_str(" AND symbol = ?");
            query_params.push(libsql::Value::Text(symbol.clone()));
        }

        if let Some(trade_type) = &query.trade_type {
            sql.push_str(" AND trade_type = ?");
            query_params.push(libsql::Value::Text(trade_type.to_string()));
        }

        if let Some(start_date) = query.start_date {
            sql.push_str(" AND entry_date >= ?");
            query_params.push(libsql::Value::Text(start_date.to_rfc3339()));
        }

        if let Some(end_date) = query.end_date {
            sql.push_str(" AND entry_date <= ?");
            query_params.push(libsql::Value::Text(end_date.to_rfc3339()));
        }

        if let Some(updated_after) = query.updated_after {
            sql.push_str(" AND updated_at >= ?");
            query_params.push(libsql::Value::Text(updated_after.to_rfc3339()));
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

    /// Calculations here
    /// Calculate total P&L for all stocks in the user's database
    pub async fn calculate_total_pnl(
        conn: &Connection,
    ) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        let mut rows = conn
            .prepare(
            r#"
            SELECT SUM(
                CASE 
                    WHEN exit_price IS NOT NULL THEN 
                        CASE 
                            WHEN trade_type = 'BUY' THEN (exit_price - entry_price) * number_shares - commissions
                            WHEN trade_type = 'SELL' THEN (entry_price - exit_price) * number_shares - commissions
                        END
                    ELSE 0
                END
            ) as total_pnl
            FROM stocks
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
                    CASE
                        WHEN trade_type = 'BUY' THEN (COALESCE(exit_price, 0) - entry_price) * number_shares - COALESCE(commissions, 0)
                        WHEN trade_type = 'SELL' THEN (entry_price - COALESCE(exit_price, 0)) * number_shares - COALESCE(commissions, 0)
                    END AS profit
                FROM stocks
                WHERE exit_date IS NOT NULL 
                  AND exit_price IS NOT NULL
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
                        WHEN (trade_type = 'BUY' AND exit_price > entry_price) OR
                             (trade_type = 'SELL' AND exit_price < entry_price) THEN 1
                        ELSE 0
                    END AS is_winning_trade
                FROM stocks
                WHERE exit_date IS NOT NULL
                  AND exit_price IS NOT NULL
                  AND entry_price IS NOT NULL
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
            FROM stocks
            WHERE exit_date IS NOT NULL
              AND exit_price IS NOT NULL
              AND (
                  (trade_type = 'BUY' AND exit_price > entry_price) OR
                  (trade_type = 'SELL' AND exit_price < entry_price)
              )
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
            FROM stocks
            WHERE exit_date IS NOT NULL
              AND exit_price IS NOT NULL
              AND (
                  (trade_type = 'BUY' AND exit_price < entry_price) OR
                  (trade_type = 'SELL' AND exit_price > entry_price)
              )
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
                    CASE 
                        WHEN trade_type = 'BUY' THEN (exit_price - entry_price) * number_shares - COALESCE(commissions, 0)
                        WHEN trade_type = 'SELL' THEN (entry_price - exit_price) * number_shares - COALESCE(commissions, 0)
                    END
                ), 0) as biggest_winner
            FROM stocks
            WHERE exit_date IS NOT NULL
              AND exit_price IS NOT NULL
              AND (
                  (trade_type = 'BUY' AND exit_price > entry_price) OR
                  (trade_type = 'SELL' AND exit_price < entry_price)
              )
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
                    CASE 
                        WHEN trade_type = 'BUY' THEN (exit_price - entry_price) * number_shares - COALESCE(commissions, 0)
                        WHEN trade_type = 'SELL' THEN (entry_price - exit_price) * number_shares - COALESCE(commissions, 0)
                    END
                ), 0) as biggest_loser
            FROM stocks
            WHERE exit_date IS NOT NULL
              AND exit_price IS NOT NULL
              AND (
                  (trade_type = 'BUY' AND exit_price < entry_price) OR
                  (trade_type = 'SELL' AND exit_price > entry_price)
              )
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
                    CASE 
                        WHEN trade_type = 'BUY' THEN (exit_price - entry_price) * number_shares - COALESCE(commissions, 0)
                        WHEN trade_type = 'SELL' THEN (entry_price - exit_price) * number_shares - COALESCE(commissions, 0)
                    END
                ), 2), 0) as avg_gain
            FROM stocks
            WHERE exit_date IS NOT NULL
              AND exit_price IS NOT NULL
              AND (
                  (trade_type = 'BUY' AND exit_price > entry_price) OR
                  (trade_type = 'SELL' AND exit_price < entry_price)
              )
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
                    CASE 
                        WHEN trade_type = 'BUY' THEN (exit_price - entry_price) * number_shares - COALESCE(commissions, 0)
                        WHEN trade_type = 'SELL' THEN (entry_price - exit_price) * number_shares - COALESCE(commissions, 0)
                    END
                ), 2), 0) as avg_loss
            FROM stocks
            WHERE exit_date IS NOT NULL
              AND exit_price IS NOT NULL
              AND (
                  (trade_type = 'BUY' AND exit_price < entry_price) OR
                  (trade_type = 'SELL' AND exit_price > entry_price)
              )
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

    /// Calculate average position size
    pub async fn calculate_avg_position_size(
        conn: &Connection,
        time_range: TimeRange,
    ) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        let (time_condition, time_params) = time_range.to_sql_condition();

        let sql = format!(
            r#"
            SELECT 
                COALESCE(ROUND(AVG(entry_price * number_shares), 2), 0) as avg_position_size
            FROM stocks
            WHERE exit_date IS NOT NULL
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

    /// Calculate average risk per trade (based on stop loss)
    #[allow(dead_code)]
    pub async fn calculate_avg_risk_per_trade(
        conn: &Connection,
        time_range: TimeRange,
    ) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        let (time_condition, time_params) = time_range.to_sql_condition();

        let sql = format!(
            r#"
            SELECT 
                COALESCE(ROUND(AVG(
                    CASE 
                        WHEN trade_type = 'BUY' THEN (entry_price - stop_loss) * number_shares
                        WHEN trade_type = 'SELL' THEN (stop_loss - entry_price) * number_shares
                    END
                ), 2), 0) as avg_risk
            FROM stocks
            WHERE stop_loss IS NOT NULL
              AND exit_date IS NOT NULL
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
                        WHEN trade_type = 'BUY' THEN (COALESCE(exit_price, entry_price) - entry_price) * number_shares - COALESCE(commissions, 0)
                        WHEN trade_type = 'SELL' THEN (entry_price - COALESCE(exit_price, entry_price)) * number_shares - COALESCE(commissions, 0)
                    END
                ), 0) as net_pnl
            FROM stocks
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

    /// Convert from libsql row to Stock struct
    /// Get playbook setups associated with this stock trade
    #[allow(dead_code)]
    pub async fn get_playbooks(
        &self,
        conn: &Connection,
    ) -> Result<Vec<crate::models::playbook::Playbook>, Box<dyn std::error::Error + Send + Sync>>
    {
        crate::models::playbook::Playbook::get_stock_trade_playbooks(conn, self.id).await
    }

    /// Tag this stock trade with a playbook setup
    #[allow(dead_code)]
    pub async fn tag_with_playbook(
        &self,
        conn: &Connection,
        setup_id: &str,
    ) -> Result<crate::models::playbook::StockTradePlaybook, Box<dyn std::error::Error + Send + Sync>>
    {
        crate::models::playbook::Playbook::tag_stock_trade(conn, self.id, setup_id).await
    }

    /// Remove a playbook tag from this stock trade
    #[allow(dead_code)]
    pub async fn untag_playbook(
        &self,
        conn: &Connection,
        setup_id: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        crate::models::playbook::Playbook::untag_stock_trade(conn, self.id, setup_id).await
    }

    fn from_row(row: &libsql::Row) -> Result<Stock, Box<dyn std::error::Error + Send + Sync>> {
        let id = Self::get_i64(row, 0)?;
        let symbol = Self::get_string(row, 1)?;
        let trade_type_str = Self::get_string(row, 2)?;
        let order_type_str = Self::get_string(row, 3)?;

        let trade_type = if trade_type_str.is_empty() {
            TradeType::BUY // Default fallback
        } else {
            trade_type_str
                .parse::<TradeType>()
                .map_err(|e| format!("Invalid trade type: {}", e))?
        };

        let order_type = if order_type_str.is_empty() {
            OrderType::MARKET // Default fallback
        } else {
            order_type_str
                .parse::<OrderType>()
                .map_err(|e| format!("Invalid order type: {}", e))?
        };

        // Parse datetime strings (support RFC3339 and SQLite's CURRENT_TIMESTAMP format)
        let entry_date_str = Self::get_string(row, 13)?;
        let exit_date_str = Self::get_string(row, 14)?;
        let reviewed = Self::get_bool(row, 15)?;
        let mistakes_str = Self::get_opt_string(row, 16)?;
        let brokerage_name = Self::get_opt_string(row, 17)?;
        let trade_group_id = Self::get_opt_string(row, 18)?;
        let parent_trade_id = Self::get_opt_i64(row, 19)?;
        let total_quantity = Self::get_opt_f64(row, 20)?;
        let transaction_sequence = Self::get_opt_i32(row, 21)?;
        let created_at_str = Self::get_string(row, 22)?;
        let updated_at_str = Self::get_string(row, 23)?;

        fn parse_dt_any(
            s: &str,
        ) -> Result<DateTime<Utc>, Box<dyn std::error::Error + Send + Sync>> {
            if s.is_empty() {
                return Ok(Utc::now()); // Default to current time if empty
            }
            if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
                return Ok(dt.with_timezone(&Utc));
            }
            if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
                return Ok(DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            }
            if let Ok(date) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                let ndt = date.and_hms_opt(0, 0, 0).ok_or("invalid date")?;
                return Ok(DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            }
            Err(format!("Unsupported datetime format: {}", s).into())
        }

        let entry_date = parse_dt_any(&entry_date_str)
            .map_err(|e| format!("Failed to parse entry_date: {}", e))?;

        // Handle null exit_date for open trades - use entry_date as fallback
        let exit_date = if exit_date_str.is_empty() {
            entry_date // For open trades, use entry_date as fallback
        } else {
            parse_dt_any(&exit_date_str).map_err(|e| format!("Failed to parse exit_date: {}", e))?
        };

        let created_at = parse_dt_any(&created_at_str)
            .map_err(|e| format!("Failed to parse created_at: {}", e))?;
        let updated_at = parse_dt_any(&updated_at_str)
            .map_err(|e| format!("Failed to parse updated_at: {}", e))?;

        Ok(Stock {
            id,
            symbol,
            trade_type,
            order_type,
            entry_price: Self::get_f64(row, 4)?,
            exit_price: Self::get_f64(row, 5)?,
            stop_loss: Self::get_f64(row, 6)?,
            commissions: Self::get_f64(row, 7)?,
            number_shares: Self::get_f64(row, 8)?,
            take_profit: Self::get_opt_f64(row, 9)?,
            initial_target: Self::get_opt_f64(row, 10)?,
            profit_target: Self::get_opt_f64(row, 11)?,
            trade_ratings: Self::get_opt_i32(row, 12)?,
            entry_date,
            exit_date,
            reviewed,
            mistakes: mistakes_str,
            brokerage_name,
            trade_group_id,
            parent_trade_id,
            total_quantity,
            transaction_sequence,
            created_at,
            updated_at,
        })
    }
}
