use chrono::{DateTime, Utc};
use libsql::{Connection, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Trade note model for user's isolated database
/// No user_id needed since each user has their own database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeNote {
    pub id: String,
    pub name: String,
    pub content: String,
    pub trade_type: Option<String>, // 'stock' or 'option'
    pub stock_trade_id: Option<i64>,
    pub option_trade_id: Option<i64>,
    pub ai_metadata: Option<String>, // JSON string
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Data Transfer Object for creating new trade notes
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTradeNoteRequest {
    pub name: String,
    pub content: String,
}

/// Data Transfer Object for updating trade notes
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateTradeNoteRequest {
    pub name: Option<String>,
    pub content: Option<String>,
}

/// Trade note query parameters for filtering and pagination
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TradeNoteQuery {
    pub name: Option<String>,
    pub search: Option<String>, // Search in both name and content
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Trade note operations implementation using libsql
impl TradeNote {
    /// Create a new trade note in the user's database
    pub async fn create(
        conn: &Connection,
        request: CreateTradeNoteRequest,
    ) -> Result<TradeNote, Box<dyn std::error::Error + Send + Sync>> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let mut rows = conn
            .prepare(
                r#"
                INSERT INTO trade_notes (
                    id, name, content, created_at, updated_at
                ) VALUES (?, ?, ?, ?, ?)
                RETURNING id, name, content, created_at, updated_at
                "#,
            )
            .await?
            .query(params![id, request.name, request.content, now.clone(), now])
            .await?;

        if let Some(row) = rows.next().await? {
            Ok(TradeNote::from_row(&row)?)
        } else {
            Err("Failed to create trade note".into())
        }
    }

    /// Find a trade note by ID in the user's database
    pub async fn find_by_id(
        conn: &Connection,
        note_id: &str,
    ) -> Result<Option<TradeNote>, Box<dyn std::error::Error + Send + Sync>> {
        let mut rows = conn
            .prepare(
                r#"
                SELECT id, name, content, trade_type, stock_trade_id, option_trade_id, ai_metadata, created_at, updated_at
                FROM trade_notes 
                WHERE id = ?
                "#,
            )
            .await?
            .query(params![note_id])
            .await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(TradeNote::from_row(&row)?))
        } else {
            Ok(None)
        }
    }

    /// Find a trade note by stock trade ID
    pub async fn find_by_stock_trade_id(
        conn: &Connection,
        stock_trade_id: i64,
    ) -> Result<Option<TradeNote>, Box<dyn std::error::Error + Send + Sync>> {
        let mut rows = conn
            .prepare(
                r#"
                SELECT id, name, content, trade_type, stock_trade_id, option_trade_id, ai_metadata, created_at, updated_at
                FROM trade_notes 
                WHERE stock_trade_id = ?
                "#,
            )
            .await?
            .query(params![stock_trade_id])
            .await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(TradeNote::from_row(&row)?))
        } else {
            Ok(None)
        }
    }

    /// Find a trade note by option trade ID
    pub async fn find_by_option_trade_id(
        conn: &Connection,
        option_trade_id: i64,
    ) -> Result<Option<TradeNote>, Box<dyn std::error::Error + Send + Sync>> {
        let mut rows = conn
            .prepare(
                r#"
                SELECT id, name, content, trade_type, stock_trade_id, option_trade_id, ai_metadata, created_at, updated_at
                FROM trade_notes 
                WHERE option_trade_id = ?
                "#,
            )
            .await?
            .query(params![option_trade_id])
            .await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(TradeNote::from_row(&row)?))
        } else {
            Ok(None)
        }
    }

    /// Upsert a trade note linked to a trade (create if not exists, update if exists)
    pub async fn upsert_for_trade(
        conn: &Connection,
        trade_type: &str,
        trade_id: i64,
        name: String,
        content: String,
        ai_metadata: Option<String>,
    ) -> Result<TradeNote, Box<dyn std::error::Error + Send + Sync>> {
        let now = Utc::now().to_rfc3339();

        // Check if note already exists for this trade
        let existing_note = match trade_type {
            "stock" => Self::find_by_stock_trade_id(conn, trade_id).await?,
            "option" => Self::find_by_option_trade_id(conn, trade_id).await?,
            _ => return Err("Invalid trade_type. Must be 'stock' or 'option'".into()),
        };

        if let Some(_note) = existing_note {
            // Update existing note
            let update_sql = match trade_type {
                "stock" => {
                    r#"
                    UPDATE trade_notes 
                    SET name = ?, content = ?, ai_metadata = ?, updated_at = ?
                    WHERE stock_trade_id = ?
                    RETURNING id, name, content, trade_type, stock_trade_id, option_trade_id, ai_metadata, created_at, updated_at
                "#
                }
                "option" => {
                    r#"
                    UPDATE trade_notes 
                    SET name = ?, content = ?, ai_metadata = ?, updated_at = ?
                    WHERE option_trade_id = ?
                    RETURNING id, name, content, trade_type, stock_trade_id, option_trade_id, ai_metadata, created_at, updated_at
                "#
                }
                _ => return Err("Invalid trade_type".into()),
            };

            let mut rows = conn
                .prepare(update_sql)
                .await?
                .query(params![name, content, ai_metadata, now, trade_id])
                .await?;

            if let Some(row) = rows.next().await? {
                Ok(TradeNote::from_row(&row)?)
            } else {
                Err("Failed to update trade note".into())
            }
        } else {
            // Create new note
            let id = Uuid::new_v4().to_string();
            let (stock_id, option_id) = match trade_type {
                "stock" => (Some(trade_id), None),
                "option" => (None, Some(trade_id)),
                _ => return Err("Invalid trade_type".into()),
            };

            let mut rows = conn
                .prepare(
                    r#"
                    INSERT INTO trade_notes (
                        id, name, content, trade_type, stock_trade_id, option_trade_id, ai_metadata, created_at, updated_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                    RETURNING id, name, content, trade_type, stock_trade_id, option_trade_id, ai_metadata, created_at, updated_at
                    "#,
                )
                .await?
                .query(params![
                    id,
                    name,
                    content,
                    trade_type,
                    stock_id,
                    option_id,
                    ai_metadata,
                    now.clone(),
                    now
                ])
                .await?;

            if let Some(row) = rows.next().await? {
                Ok(TradeNote::from_row(&row)?)
            } else {
                Err("Failed to create trade note".into())
            }
        }
    }

    /// Find all trade notes with optional filtering
    pub async fn find_all(
        conn: &Connection,
        query: TradeNoteQuery,
    ) -> Result<Vec<TradeNote>, Box<dyn std::error::Error + Send + Sync>> {
        let mut sql = String::from(
            r#"
            SELECT id, name, content, trade_type, stock_trade_id, option_trade_id, ai_metadata, created_at, updated_at
            FROM trade_notes 
            WHERE 1=1
            "#,
        );

        let mut query_params: Vec<libsql::Value> = Vec::new();

        // Add optional filters
        if let Some(name) = &query.name {
            sql.push_str(" AND name LIKE ?");
            query_params.push(libsql::Value::Text(format!("%{}%", name)));
        }

        if let Some(search) = &query.search {
            sql.push_str(" AND (name LIKE ? OR content LIKE ?)");
            let pattern = format!("%{}%", search);
            query_params.push(libsql::Value::Text(pattern.clone()));
            query_params.push(libsql::Value::Text(pattern));
        }

        if let Some(start_date) = query.start_date {
            sql.push_str(" AND created_at >= ?");
            query_params.push(libsql::Value::Text(start_date.to_rfc3339()));
        }

        if let Some(end_date) = query.end_date {
            sql.push_str(" AND created_at <= ?");
            query_params.push(libsql::Value::Text(end_date.to_rfc3339()));
        }

        sql.push_str(" ORDER BY updated_at DESC");

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

        let mut notes = Vec::new();
        while let Some(row) = rows.next().await? {
            notes.push(TradeNote::from_row(&row)?);
        }

        Ok(notes)
    }

    /// Update a trade note
    pub async fn update(
        conn: &Connection,
        note_id: &str,
        request: UpdateTradeNoteRequest,
    ) -> Result<Option<TradeNote>, Box<dyn std::error::Error + Send + Sync>> {
        // Ensure the note exists
        let current_note = Self::find_by_id(conn, note_id).await?;
        if current_note.is_none() {
            return Ok(None);
        }

        let now = Utc::now().to_rfc3339();

        let mut rows = conn
            .prepare(
                r#"
                UPDATE trade_notes SET 
                    name = COALESCE(?, name),
                    content = COALESCE(?, content),
                    updated_at = ?
                WHERE id = ?
                RETURNING id, name, content, trade_type, stock_trade_id, option_trade_id, ai_metadata, created_at, updated_at
                "#,
            )
            .await?
            .query(params![
                request.name,
                request.content,
                now,
                note_id
            ])
            .await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(TradeNote::from_row(&row)?))
        } else {
            Ok(None)
        }
    }

    /// Delete a trade note
    pub async fn delete(
        conn: &Connection,
        note_id: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let result = conn
            .execute("DELETE FROM trade_notes WHERE id = ?", params![note_id])
            .await?;

        Ok(result > 0)
    }

    /// Get total count of trade notes (for pagination)
    pub async fn count(
        conn: &Connection,
        query: &TradeNoteQuery,
    ) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
        let mut sql = String::from("SELECT COUNT(*) FROM trade_notes WHERE 1=1");
        let mut query_params: Vec<libsql::Value> = Vec::new();

        // Add the same filters as in find_all
        if let Some(name) = &query.name {
            sql.push_str(" AND name LIKE ?");
            query_params.push(libsql::Value::Text(format!("%{}%", name)));
        }

        if let Some(search) = &query.search {
            sql.push_str(" AND (name LIKE ? OR content LIKE ?)");
            let pattern = format!("%{}%", search);
            query_params.push(libsql::Value::Text(pattern.clone()));
            query_params.push(libsql::Value::Text(pattern));
        }

        if let Some(start_date) = query.start_date {
            sql.push_str(" AND created_at >= ?");
            query_params.push(libsql::Value::Text(start_date.to_rfc3339()));
        }

        if let Some(end_date) = query.end_date {
            sql.push_str(" AND created_at <= ?");
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

    /// Search trade notes by content
    pub async fn search_by_content(
        conn: &Connection,
        search_term: &str,
        limit: Option<i64>,
    ) -> Result<Vec<TradeNote>, Box<dyn std::error::Error + Send + Sync>> {
        let mut sql = String::from(
            r#"
            SELECT id, name, content, trade_type, stock_trade_id, option_trade_id, ai_metadata, created_at, updated_at
            FROM trade_notes 
            WHERE content LIKE ?
            ORDER BY updated_at DESC
            "#,
        );

        let mut params_vec: Vec<libsql::Value> =
            vec![libsql::Value::Text(format!("%{}%", search_term))];

        if let Some(limit) = limit {
            sql.push_str(" LIMIT ?");
            params_vec.push(libsql::Value::Integer(limit));
        }

        let mut rows = conn
            .prepare(&sql)
            .await?
            .query(libsql::params_from_iter(params_vec))
            .await?;

        let mut notes = Vec::new();
        while let Some(row) = rows.next().await? {
            notes.push(TradeNote::from_row(&row)?);
        }

        Ok(notes)
    }

    /// Get recent trade notes (last N notes)
    pub async fn get_recent(
        conn: &Connection,
        limit: i64,
    ) -> Result<Vec<TradeNote>, Box<dyn std::error::Error + Send + Sync>> {
        let mut rows = conn
            .prepare(
                r#"
                SELECT id, name, content, trade_type, stock_trade_id, option_trade_id, ai_metadata, created_at, updated_at
                FROM trade_notes 
                ORDER BY updated_at DESC
                LIMIT ?
                "#,
            )
            .await?
            .query(params![limit])
            .await?;

        let mut notes = Vec::new();
        while let Some(row) = rows.next().await? {
            notes.push(TradeNote::from_row(&row)?);
        }

        Ok(notes)
    }

    /// Get trade notes created in a specific date range
    #[allow(dead_code)]
    pub async fn get_by_date_range(
        conn: &Connection,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<TradeNote>, Box<dyn std::error::Error + Send + Sync>> {
        let mut rows = conn
            .prepare(
                r#"
                SELECT id, name, content, created_at, updated_at
                FROM trade_notes 
                WHERE created_at >= ? AND created_at <= ?
                ORDER BY created_at DESC
                "#,
            )
            .await?
            .query(params![start_date.to_rfc3339(), end_date.to_rfc3339()])
            .await?;

        let mut notes = Vec::new();
        while let Some(row) = rows.next().await? {
            notes.push(TradeNote::from_row(&row)?);
        }

        Ok(notes)
    }

    /// Get trade notes updated in a specific date range
    #[allow(dead_code)]
    pub async fn get_updated_in_range(
        conn: &Connection,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<TradeNote>, Box<dyn std::error::Error + Send + Sync>> {
        let mut rows = conn
            .prepare(
                r#"
                SELECT id, name, content, created_at, updated_at
                FROM trade_notes 
                WHERE updated_at >= ? AND updated_at <= ?
                ORDER BY updated_at DESC
                "#,
            )
            .await?
            .query(params![start_date.to_rfc3339(), end_date.to_rfc3339()])
            .await?;

        let mut notes = Vec::new();
        while let Some(row) = rows.next().await? {
            notes.push(TradeNote::from_row(&row)?);
        }

        Ok(notes)
    }

    /// Get total count of all trade notes
    pub async fn total_count(
        conn: &Connection,
    ) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
        let mut rows = conn
            .prepare("SELECT COUNT(*) FROM trade_notes")
            .await?
            .query(params![])
            .await?;

        if let Some(row) = rows.next().await? {
            Ok(row.get::<i64>(0)?)
        } else {
            Ok(0)
        }
    }

    /// Check if a trade note exists by ID
    #[allow(dead_code)]
    pub async fn exists(
        conn: &Connection,
        note_id: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let mut rows = conn
            .prepare("SELECT COUNT(*) FROM trade_notes WHERE id = ?")
            .await?
            .query(params![note_id])
            .await?;

        if let Some(row) = rows.next().await? {
            let count: i64 = row.get(0)?;
            Ok(count > 0)
        } else {
            Ok(false)
        }
    }

    /// Get trade notes with pagination
    #[allow(dead_code)]
    pub async fn get_paginated(
        conn: &Connection,
        page: i64,
        page_size: i64,
    ) -> Result<Vec<TradeNote>, Box<dyn std::error::Error + Send + Sync>> {
        let offset = (page - 1) * page_size;

        let mut rows = conn
            .prepare(
                r#"
                SELECT id, name, content, created_at, updated_at
                FROM trade_notes 
                ORDER BY updated_at DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .await?
            .query(params![page_size, offset])
            .await?;

        let mut notes = Vec::new();
        while let Some(row) = rows.next().await? {
            notes.push(TradeNote::from_row(&row)?);
        }

        Ok(notes)
    }
}

/// Convert from libsql row to TradeNote struct
impl TradeNote {
    pub fn from_row(
        row: &libsql::Row,
    ) -> Result<TradeNote, Box<dyn std::error::Error + Send + Sync>> {
        // Helper function to parse datetime from various formats (RFC3339, SQLite format, etc.)
        fn parse_dt_any(
            s: &str,
        ) -> Result<DateTime<Utc>, Box<dyn std::error::Error + Send + Sync>> {
            // Handle empty strings
            if s.is_empty() {
                return Err("Empty datetime string".into());
            }

            // Try RFC3339 format first (e.g., "2024-01-01T12:00:00Z")
            if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
                return Ok(dt.with_timezone(&Utc));
            }

            // Try SQLite's datetime format (e.g., "2024-01-01 12:00:00")
            if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
                return Ok(DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            }

            // Try datetime with microseconds (e.g., "2024-01-01 12:00:00.123456")
            if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f") {
                return Ok(DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            }

            // Try date-only format (e.g., "2024-01-01")
            if let Ok(date) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                let ndt = date
                    .and_hms_opt(0, 0, 0)
                    .ok_or_else(|| "Invalid date".to_string())?;
                return Ok(DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
            }

            Err(format!("Unsupported datetime format: {}", s).into())
        }

        // Try to get column count to determine schema version
        let column_count = row.column_count();

        // New schema with trade linking fields (column_count >= 9)
        if column_count >= 9 {
            let created_at_str: String = row.get(7)?; // created_at at index 7
            let updated_at_str: String = row.get(8)?; // updated_at at index 8

            let created_at = parse_dt_any(&created_at_str)
                .map_err(|e| format!("Failed to parse created_at: {}", e))?;

            let updated_at = parse_dt_any(&updated_at_str)
                .map_err(|e| format!("Failed to parse updated_at: {}", e))?;

            Ok(TradeNote {
                id: row.get(0)?,
                name: row.get(1)?,
                content: row.get(2)?,
                trade_type: row.get(3).ok(),
                stock_trade_id: row.get(4).ok(),
                option_trade_id: row.get(5).ok(),
                ai_metadata: row.get(6).ok(),
                created_at,
                updated_at,
            })
        } else {
            // Legacy schema (column_count == 5)
            let created_at_str: String = row.get(3)?;
            let updated_at_str: String = row.get(4)?;

            let created_at = parse_dt_any(&created_at_str)
                .map_err(|e| format!("Failed to parse created_at: {}", e))?;

            let updated_at = parse_dt_any(&updated_at_str)
                .map_err(|e| format!("Failed to parse updated_at: {}", e))?;

            Ok(TradeNote {
                id: row.get(0)?,
                name: row.get(1)?,
                content: row.get(2)?,
                trade_type: None,
                stock_trade_id: None,
                option_trade_id: None,
                ai_metadata: None,
                created_at,
                updated_at,
            })
        }
    }
}
