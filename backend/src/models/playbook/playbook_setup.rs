use chrono::{DateTime, NaiveDateTime, Utc};
use libsql::Connection;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Playbook setup for trading strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playbook {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub emoji: Option<String>,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Data Transfer Object for creating new playbook setups
#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePlaybookRequest {
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub emoji: Option<String>,
    pub color: Option<String>,
}

/// Data Transfer Object for updating playbook setups
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdatePlaybookRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub emoji: Option<String>,
    pub color: Option<String>,
}

/// Playbook query parameters for filtering and pagination
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlaybookQuery {
    pub name: Option<String>,
    pub search: Option<String>, // Search in both name and description
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Stock trade playbook association
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockTradePlaybook {
    pub stock_trade_id: i64,
    pub setup_id: String,
    pub created_at: DateTime<Utc>,
}

/// Option trade playbook association
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionTradePlaybook {
    pub option_trade_id: i64,
    pub setup_id: String,
    pub created_at: DateTime<Utc>,
}

/// Data Transfer Object for tagging trades with playbook setups
#[derive(Debug, Serialize, Deserialize)]
pub struct TagTradeRequest {
    pub trade_id: i64,
    pub setup_id: String,
    pub trade_type: TradeType,
}

/// Trade type enum for tagging
#[derive(Debug, Serialize, Deserialize)]
pub enum TradeType {
    #[serde(rename = "stock")]
    Stock,
    #[serde(rename = "option")]
    Option,
}

/// Playbook operations implementation using libsql
impl Playbook {
    /// Create a new playbook setup in the user's database
    pub async fn create(
        conn: &Connection,
        request: CreatePlaybookRequest,
    ) -> Result<Playbook, Box<dyn std::error::Error + Send + Sync>> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO playbook (id, name, description, icon, emoji, color, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            libsql::params![
                id.clone(),
                request.name,
                request.description,
                request.icon,
                request.emoji,
                request.color,
                now.clone(),
                now
            ],
        ).await?;

        Self::find_by_id(conn, &id)
            .await?
            .ok_or_else(|| "Failed to retrieve created playbook".into())
    }

    /// Find a playbook by ID
    pub async fn find_by_id(
        conn: &Connection,
        playbook_id: &str,
    ) -> Result<Option<Playbook>, Box<dyn std::error::Error + Send + Sync>> {
        let mut rows = conn
            .prepare("SELECT id, name, description, icon, emoji, color, created_at, updated_at FROM playbook WHERE id = ?")
            .await?
            .query(libsql::params![playbook_id])
            .await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(Self::from_row(&row)?))
        } else {
            Ok(None)
        }
    }

    /// Find all playbooks with optional filtering
    pub async fn find_all(
        conn: &Connection,
        query: PlaybookQuery,
    ) -> Result<Vec<Playbook>, Box<dyn std::error::Error + Send + Sync>> {
        let mut sql = "SELECT id, name, description, icon, emoji, color, created_at, updated_at FROM playbook".to_string();
        let mut params = Vec::new();
        let mut conditions = Vec::new();

        if let Some(name) = query.name {
            conditions.push("name LIKE ?");
            params.push(format!("%{}%", name));
        }

        if let Some(search) = query.search {
            conditions.push("(name LIKE ? OR description LIKE ?)");
            params.push(format!("%{}%", search));
            params.push(format!("%{}%", search));
        }

        if !conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&conditions.join(" AND "));
        }

        sql.push_str(" ORDER BY updated_at DESC, name");

        if let Some(limit) = query.limit {
            sql.push_str(" LIMIT ?");
            params.push(limit.to_string());
        }

        if let Some(offset) = query.offset {
            sql.push_str(" OFFSET ?");
            params.push(offset.to_string());
        }

        let mut rows = conn
            .prepare(&sql)
            .await?
            .query(libsql::params_from_iter(params))
            .await?;

        let mut playbooks = Vec::new();
        while let Some(row) = rows.next().await? {
            playbooks.push(Self::from_row(&row)?);
        }

        Ok(playbooks)
    }

    /// Update a playbook setup
    pub async fn update(
        conn: &Connection,
        playbook_id: &str,
        request: UpdatePlaybookRequest,
    ) -> Result<Option<Playbook>, Box<dyn std::error::Error + Send + Sync>> {
        let mut set_clauses = Vec::new();
        let mut params = Vec::new();

        if let Some(name) = request.name {
            set_clauses.push("name = ?");
            params.push(name);
        }

        if let Some(description) = request.description {
            set_clauses.push("description = ?");
            params.push(description);
        }

        if let Some(icon) = request.icon {
            set_clauses.push("icon = ?");
            params.push(icon);
        }

        if let Some(emoji) = request.emoji {
            set_clauses.push("emoji = ?");
            params.push(emoji);
        }

        if let Some(color) = request.color {
            set_clauses.push("color = ?");
            params.push(color);
        }

        if set_clauses.is_empty() {
            return Self::find_by_id(conn, playbook_id).await;
        }

        set_clauses.push("updated_at = ?");
        params.push(Utc::now().to_rfc3339());
        params.push(playbook_id.to_string());

        let sql = format!(
            "UPDATE playbook SET {} WHERE id = ?",
            set_clauses.join(", ")
        );

        conn.execute(&sql, libsql::params_from_iter(params)).await?;
        Self::find_by_id(conn, playbook_id).await
    }

    /// Delete a playbook setup
    pub async fn delete(
        conn: &Connection,
        playbook_id: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let result = conn
            .execute(
                "DELETE FROM playbook WHERE id = ?",
                libsql::params![playbook_id],
            )
            .await?;

        Ok(result > 0)
    }

    /// Count playbooks with optional filtering
    #[allow(dead_code)]
    pub async fn count(
        conn: &Connection,
        query: &PlaybookQuery,
    ) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
        let mut sql = "SELECT COUNT(*) FROM playbook".to_string();
        let mut params = Vec::new();
        let mut conditions = Vec::new();

        if let Some(name) = &query.name {
            conditions.push("name LIKE ?");
            params.push(format!("%{}%", name));
        }

        if let Some(search) = &query.search {
            conditions.push("(name LIKE ? OR description LIKE ?)");
            params.push(format!("%{}%", search));
            params.push(format!("%{}%", search));
        }

        if !conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&conditions.join(" AND "));
        }

        let mut rows = conn
            .prepare(&sql)
            .await?
            .query(libsql::params_from_iter(params))
            .await?;

        if let Some(row) = rows.next().await? {
            Ok(row.get(0)?)
        } else {
            Ok(0)
        }
    }

    /// Get total count of all playbooks
    pub async fn total_count(
        conn: &Connection,
    ) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
        let mut rows = conn
            .prepare("SELECT COUNT(*) FROM playbook")
            .await?
            .query(libsql::params![])
            .await?;

        if let Some(row) = rows.next().await? {
            Ok(row.get(0)?)
        } else {
            Ok(0)
        }
    }

    /// Check if a playbook exists
    #[allow(dead_code)]
    pub async fn exists(
        conn: &Connection,
        playbook_id: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let mut rows = conn
            .prepare("SELECT 1 FROM playbook WHERE id = ? LIMIT 1")
            .await?
            .query(libsql::params![playbook_id])
            .await?;

        Ok(rows.next().await?.is_some())
    }

    /// Get playbooks with pagination
    #[allow(dead_code)]
    pub async fn get_paginated(
        conn: &Connection,
        page: i64,
        page_size: i64,
    ) -> Result<Vec<Playbook>, Box<dyn std::error::Error + Send + Sync>> {
        let offset = (page - 1) * page_size;

        let mut rows = conn
            .prepare("SELECT id, name, description, icon, emoji, color, created_at, updated_at FROM playbook ORDER BY updated_at DESC, name LIMIT ? OFFSET ?")
            .await?
            .query(libsql::params![page_size, offset])
            .await?;

        let mut playbooks = Vec::new();
        while let Some(row) = rows.next().await? {
            playbooks.push(Self::from_row(&row)?);
        }

        Ok(playbooks)
    }

    /// Tag a stock trade with a playbook setup
    pub async fn tag_stock_trade(
        conn: &Connection,
        stock_trade_id: i64,
        setup_id: &str,
    ) -> Result<StockTradePlaybook, Box<dyn std::error::Error + Send + Sync>> {
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT OR IGNORE INTO stock_trade_playbook (stock_trade_id, setup_id, created_at) VALUES (?, ?, ?)",
            libsql::params![stock_trade_id, setup_id, now],
        ).await?;

        let mut rows = conn
            .prepare("SELECT stock_trade_id, setup_id, created_at FROM stock_trade_playbook WHERE stock_trade_id = ? AND setup_id = ?")
            .await?
            .query(libsql::params![stock_trade_id, setup_id])
            .await?;

        if let Some(row) = rows.next().await? {
            Ok(StockTradePlaybook {
                stock_trade_id: row.get(0)?,
                setup_id: row.get(1)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<String>(2)?)?
                    .with_timezone(&Utc),
            })
        } else {
            Err("Failed to create stock trade playbook association".into())
        }
    }

    /// Tag an option trade with a playbook setup
    pub async fn tag_option_trade(
        conn: &Connection,
        option_trade_id: i64,
        setup_id: &str,
    ) -> Result<OptionTradePlaybook, Box<dyn std::error::Error + Send + Sync>> {
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT OR IGNORE INTO option_trade_playbook (option_trade_id, setup_id, created_at) VALUES (?, ?, ?)",
            libsql::params![option_trade_id, setup_id, now],
        ).await?;

        let mut rows = conn
            .prepare("SELECT option_trade_id, setup_id, created_at FROM option_trade_playbook WHERE option_trade_id = ? AND setup_id = ?")
            .await?
            .query(libsql::params![option_trade_id, setup_id])
            .await?;

        if let Some(row) = rows.next().await? {
            Ok(OptionTradePlaybook {
                option_trade_id: row.get(0)?,
                setup_id: row.get(1)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<String>(2)?)?
                    .with_timezone(&Utc),
            })
        } else {
            Err("Failed to create option trade playbook association".into())
        }
    }

    /// Remove a playbook tag from a stock trade
    pub async fn untag_stock_trade(
        conn: &Connection,
        stock_trade_id: i64,
        setup_id: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let result = conn
            .execute(
                "DELETE FROM stock_trade_playbook WHERE stock_trade_id = ? AND setup_id = ?",
                libsql::params![stock_trade_id, setup_id],
            )
            .await?;

        Ok(result > 0)
    }

    /// Remove a playbook tag from an option trade
    pub async fn untag_option_trade(
        conn: &Connection,
        option_trade_id: i64,
        setup_id: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let result = conn
            .execute(
                "DELETE FROM option_trade_playbook WHERE option_trade_id = ? AND setup_id = ?",
                libsql::params![option_trade_id, setup_id],
            )
            .await?;

        Ok(result > 0)
    }

    /// Get all playbook setups for a stock trade
    pub async fn get_stock_trade_playbooks(
        conn: &Connection,
        stock_trade_id: i64,
    ) -> Result<Vec<Playbook>, Box<dyn std::error::Error + Send + Sync>> {
        let mut rows = conn
            .prepare(
                "SELECT p.id, p.name, p.description, p.icon, p.emoji, p.color, p.created_at, p.updated_at
                 FROM playbook p
                 INNER JOIN stock_trade_playbook stp ON p.id = stp.setup_id
                 WHERE stp.stock_trade_id = ?
                 ORDER BY p.name"
            )
            .await?
            .query(libsql::params![stock_trade_id])
            .await?;

        let mut playbooks = Vec::new();
        while let Some(row) = rows.next().await? {
            playbooks.push(Self::from_row(&row)?);
        }

        Ok(playbooks)
    }

    /// Get all playbook setups for an option trade
    pub async fn get_option_trade_playbooks(
        conn: &Connection,
        option_trade_id: i64,
    ) -> Result<Vec<Playbook>, Box<dyn std::error::Error + Send + Sync>> {
        let mut rows = conn
            .prepare(
                "SELECT p.id, p.name, p.description, p.icon, p.emoji, p.color, p.created_at, p.updated_at
                 FROM playbook p
                 INNER JOIN option_trade_playbook otp ON p.id = otp.setup_id
                 WHERE otp.option_trade_id = ?
                 ORDER BY p.name"
            )
            .await?
            .query(libsql::params![option_trade_id])
            .await?;

        let mut playbooks = Vec::new();
        while let Some(row) = rows.next().await? {
            playbooks.push(Self::from_row(&row)?);
        }

        Ok(playbooks)
    }

    /// Get all trades tagged with a specific playbook setup
    pub async fn get_playbook_trades(
        conn: &Connection,
        setup_id: &str,
    ) -> Result<(Vec<i64>, Vec<i64>), Box<dyn std::error::Error + Send + Sync>> {
        // Get stock trades
        let mut stock_rows = conn
            .prepare("SELECT stock_trade_id FROM stock_trade_playbook WHERE setup_id = ?")
            .await?
            .query(libsql::params![setup_id])
            .await?;

        let mut stock_trades = Vec::new();
        while let Some(row) = stock_rows.next().await? {
            stock_trades.push(row.get(0)?);
        }

        // Get option trades
        let mut option_rows = conn
            .prepare("SELECT option_trade_id FROM option_trade_playbook WHERE setup_id = ?")
            .await?
            .query(libsql::params![setup_id])
            .await?;

        let mut option_trades = Vec::new();
        while let Some(row) = option_rows.next().await? {
            option_trades.push(row.get(0)?);
        }

        Ok((stock_trades, option_trades))
    }

    /// Helper method to convert database row to Playbook struct
    fn from_row(row: &libsql::Row) -> Result<Playbook, Box<dyn std::error::Error + Send + Sync>> {
        let id: String = row.get(0)?;
        let name: String = row.get(1)?;
        let description: Option<String> = row.get(2)?;
        let icon: Option<String> = row.get(3)?;
        let emoji: Option<String> = row.get(4)?;
        let color: Option<String> = row.get(5)?;
        let created_at_str: String = row.get(6)?;
        let updated_at_str: String = row.get(7)?;

        // Parse timestamps with flexible parsing and informative errors
        let created_at = parse_flexible_datetime(&created_at_str).map_err(|e| {
            format!(
                "Failed to parse created_at '{}' for playbook '{}' (id: {}): {}",
                created_at_str, name, id, e
            )
        })?;

        let updated_at = parse_flexible_datetime(&updated_at_str).map_err(|e| {
            format!(
                "Failed to parse updated_at '{}' for playbook '{}' (id: {}): {}",
                updated_at_str, name, id, e
            )
        })?;

        Ok(Playbook {
            id,
            name,
            description,
            icon,
            emoji,
            color,
            created_at,
            updated_at,
        })
    }
}

/// Rule type enum for playbook rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleType {
    #[serde(rename = "entry_criteria")]
    EntryCriteria,
    #[serde(rename = "exit_criteria")]
    ExitCriteria,
    #[serde(rename = "market_factor")]
    MarketFactor,
}

/// Playbook rule for trading strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybookRule {
    pub id: String,
    pub playbook_id: String,
    pub rule_type: RuleType,
    pub title: String,
    pub description: Option<String>,
    pub order_position: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Data Transfer Object for creating rules
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateRuleRequest {
    pub rule_type: RuleType,
    pub title: String,
    pub description: Option<String>,
    pub order_position: Option<i32>,
}

/// Data Transfer Object for updating rules
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateRuleRequest {
    pub rule_type: Option<RuleType>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub order_position: Option<i32>,
}

/// Trade rule compliance tracking
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRuleCompliance {
    pub id: String,
    pub trade_id: i64,
    pub playbook_id: String,
    pub rule_id: String,
    pub is_followed: bool,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Data Transfer Object for updating compliance
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateRuleComplianceRequest {
    pub rule_id: String,
    pub is_followed: bool,
    pub notes: Option<String>,
}

/// Missed trade opportunity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissedTrade {
    pub id: String,
    pub playbook_id: String,
    pub symbol: String,
    pub trade_type: String,
    pub reason: String,
    pub potential_entry_price: Option<f64>,
    pub opportunity_date: DateTime<Utc>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Data Transfer Object for creating missed trades
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateMissedTradeRequest {
    pub playbook_id: String,
    pub symbol: String,
    pub trade_type: String,
    pub reason: String,
    pub potential_entry_price: Option<f64>,
    pub opportunity_date: DateTime<Utc>,
    pub notes: Option<String>,
}

impl PlaybookRule {
    /// Create a new playbook rule
    #[allow(dead_code)]
    pub async fn create(
        conn: &Connection,
        playbook_id: &str,
        request: CreateRuleRequest,
    ) -> Result<PlaybookRule, Box<dyn std::error::Error + Send + Sync>> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO playbook_rules (id, playbook_id, rule_type, title, description, order_position, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            libsql::params![
                id.clone(),
                playbook_id,
                serde_json::to_string(&request.rule_type)?,
                request.title,
                request.description,
                request.order_position.unwrap_or(0),
                now.clone(),
                now
            ],
        ).await?;

        Self::find_by_id(conn, &id)
            .await?
            .ok_or_else(|| "Failed to retrieve created rule".into())
    }

    /// Find a rule by ID
    #[allow(dead_code)]
    pub async fn find_by_id(
        conn: &Connection,
        rule_id: &str,
    ) -> Result<Option<PlaybookRule>, Box<dyn std::error::Error + Send + Sync>> {
        let mut rows = conn
            .prepare("SELECT id, playbook_id, rule_type, title, description, order_position, created_at, updated_at FROM playbook_rules WHERE id = ?")
            .await?
            .query(libsql::params![rule_id])
            .await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(Self::from_row(&row)?))
        } else {
            Ok(None)
        }
    }

    /// Find all rules for a playbook
    #[allow(dead_code)]
    pub async fn find_by_playbook_id(
        conn: &Connection,
        playbook_id: &str,
    ) -> Result<Vec<PlaybookRule>, Box<dyn std::error::Error + Send + Sync>> {
        let mut rows = conn
            .prepare("SELECT id, playbook_id, rule_type, title, description, order_position, created_at, updated_at FROM playbook_rules WHERE playbook_id = ? ORDER BY order_position")
            .await?
            .query(libsql::params![playbook_id])
            .await?;

        let mut rules = Vec::new();
        while let Some(row) = rows.next().await? {
            rules.push(Self::from_row(&row)?);
        }

        Ok(rules)
    }

    /// Update a rule
    #[allow(dead_code)]
    pub async fn update(
        conn: &Connection,
        rule_id: &str,
        request: UpdateRuleRequest,
    ) -> Result<PlaybookRule, Box<dyn std::error::Error + Send + Sync>> {
        let mut updates = Vec::new();
        let mut params: Vec<libsql::Value> = Vec::new();

        if let Some(rule_type) = request.rule_type {
            updates.push("rule_type = ?");
            params.push(libsql::Value::Text(serde_json::to_string(&rule_type)?));
        }

        if let Some(title) = request.title {
            updates.push("title = ?");
            params.push(libsql::Value::Text(title));
        }

        if let Some(description) = request.description {
            updates.push("description = ?");
            params.push(libsql::Value::Text(description));
        }

        if let Some(order_position) = request.order_position {
            updates.push("order_position = ?");
            params.push(libsql::Value::Integer(order_position as i64));
        }

        if !updates.is_empty() {
            params.push(libsql::Value::Text(rule_id.to_string()));
            let sql = format!(
                "UPDATE playbook_rules SET {} WHERE id = ?",
                updates.join(", ")
            );
            conn.execute(&sql, libsql::params_from_iter(params)).await?;
        }

        Self::find_by_id(conn, rule_id)
            .await?
            .ok_or_else(|| "Failed to retrieve updated rule".into())
    }

    /// Delete a rule
    #[allow(dead_code)]
    pub async fn delete(
        conn: &Connection,
        rule_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        conn.execute(
            "DELETE FROM playbook_rules WHERE id = ?",
            libsql::params![rule_id],
        )
        .await?;
        Ok(())
    }

    #[allow(dead_code)]
    fn from_row(
        row: &libsql::Row,
    ) -> Result<PlaybookRule, Box<dyn std::error::Error + Send + Sync>> {
        Ok(PlaybookRule {
            id: row.get(0)?,
            playbook_id: row.get(1)?,
            rule_type: serde_json::from_str(&row.get::<String>(2)?)?,
            title: row.get(3)?,
            description: row.get(4)?,
            order_position: row.get(5)?,
            created_at: parse_flexible_datetime(&row.get::<String>(6)?)?,
            updated_at: parse_flexible_datetime(&row.get::<String>(7)?)?,
        })
    }
}

impl MissedTrade {
    /// Create a new missed trade
    #[allow(dead_code)]
    pub async fn create(
        conn: &Connection,
        request: CreateMissedTradeRequest,
    ) -> Result<MissedTrade, Box<dyn std::error::Error + Send + Sync>> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO missed_trades (id, playbook_id, symbol, trade_type, reason, potential_entry_price, opportunity_date, notes, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            libsql::params![
                id.clone(),
                request.playbook_id,
                request.symbol,
                request.trade_type,
                request.reason,
                request.potential_entry_price,
                request.opportunity_date.to_rfc3339(),
                request.notes,
                now
            ],
        ).await?;

        Self::find_by_id(conn, &id)
            .await?
            .ok_or_else(|| "Failed to retrieve created missed trade".into())
    }

    /// Find a missed trade by ID
    #[allow(dead_code)]
    pub async fn find_by_id(
        conn: &Connection,
        missed_id: &str,
    ) -> Result<Option<MissedTrade>, Box<dyn std::error::Error + Send + Sync>> {
        let mut rows = conn
            .prepare("SELECT id, playbook_id, symbol, trade_type, reason, potential_entry_price, opportunity_date, notes, created_at FROM missed_trades WHERE id = ?")
            .await?
            .query(libsql::params![missed_id])
            .await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(Self::from_row(&row)?))
        } else {
            Ok(None)
        }
    }

    /// Find all missed trades for a playbook
    #[allow(dead_code)]
    pub async fn find_by_playbook_id(
        conn: &Connection,
        playbook_id: &str,
    ) -> Result<Vec<MissedTrade>, Box<dyn std::error::Error + Send + Sync>> {
        let mut rows = conn
            .prepare("SELECT id, playbook_id, symbol, trade_type, reason, potential_entry_price, opportunity_date, notes, created_at FROM missed_trades WHERE playbook_id = ? ORDER BY opportunity_date DESC")
            .await?
            .query(libsql::params![playbook_id])
            .await?;

        let mut trades = Vec::new();
        while let Some(row) = rows.next().await? {
            trades.push(Self::from_row(&row)?);
        }

        Ok(trades)
    }

    /// Delete a missed trade
    #[allow(dead_code)]
    pub async fn delete(
        conn: &Connection,
        missed_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        conn.execute(
            "DELETE FROM missed_trades WHERE id = ?",
            libsql::params![missed_id],
        )
        .await?;
        Ok(())
    }

    #[allow(dead_code)]
    fn from_row(
        row: &libsql::Row,
    ) -> Result<MissedTrade, Box<dyn std::error::Error + Send + Sync>> {
        Ok(MissedTrade {
            id: row.get(0)?,
            playbook_id: row.get(1)?,
            symbol: row.get(2)?,
            trade_type: row.get(3)?,
            reason: row.get(4)?,
            potential_entry_price: row.get(5)?,
            opportunity_date: parse_flexible_datetime(&row.get::<String>(6)?)?,
            notes: row.get(7)?,
            created_at: parse_flexible_datetime(&row.get::<String>(8)?)?,
        })
    }
}

/// Parse a timestamp string in multiple common formats into UTC DateTime
fn parse_flexible_datetime(
    s: &str,
) -> Result<DateTime<Utc>, Box<dyn std::error::Error + Send + Sync>> {
    // Try RFC3339 first (e.g., 2025-10-28T14:42:54.714421+00:00 or ...Z)
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }

    // Try SQLite-like: "YYYY-MM-DD HH:MM:SS[.fraction]" and assume UTC
    if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f") {
        return Ok(DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
    }

    Err(format!("Unable to parse datetime: '{}'", s).into())
}
