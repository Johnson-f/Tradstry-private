use anyhow::Result;
use chrono::{DateTime, Utc};
use libsql::{Connection, params};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeTag {
    pub id: String,
    pub category: String,
    pub name: String,
    pub color: Option<String>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTagRequest {
    pub category: String,
    pub name: String,
    pub color: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTagRequest {
    pub category: Option<String>,
    pub name: Option<String>,
    pub color: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TagQuery {
    pub category: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl TradeTag {
    /// Helper function to safely get String from database row (for NOT NULL fields)
    fn get_string(row: &libsql::Row, idx: i32) -> Result<String> {
        match row.get_value(idx)? {
            libsql::Value::Text(s) => Ok(s),
            libsql::Value::Null => {
                // For NOT NULL fields, this shouldn't happen, but handle gracefully
                anyhow::bail!("Unexpected NULL value at index {}", idx)
            }
            libsql::Value::Integer(n) => Ok(n.to_string()),
            libsql::Value::Real(r) => Ok(r.to_string()),
            libsql::Value::Blob(_) => {
                anyhow::bail!("Unexpected BLOB value at index {} (expected TEXT)", idx)
            }
        }
    }

    /// Helper function to safely get Option<String> from database row
    fn get_opt_string(row: &libsql::Row, idx: i32) -> Result<Option<String>> {
        match row.get_value(idx)? {
            libsql::Value::Text(s) => Ok(Some(s)),
            libsql::Value::Null => Ok(None),
            libsql::Value::Integer(n) => Ok(Some(n.to_string())),
            libsql::Value::Real(r) => Ok(Some(r.to_string())),
            libsql::Value::Blob(_) => Ok(None),
        }
    }

    /// Helper function to convert DateTime<Utc> to database-compatible string (RFC3339)
    fn to_db_datetime(dt: DateTime<Utc>) -> String {
        dt.to_rfc3339()
    }

    /// Helper function to parse datetime from database (handles both RFC3339 and SQLite format)
    fn parse_db_datetime(datetime_str: &str, field_name: &str) -> Result<DateTime<Utc>> {
        if datetime_str.contains('T') {
            // RFC3339 format: "2025-10-15T20:55:53.148886+00:00"
            DateTime::parse_from_rfc3339(datetime_str)
                .map_err(|e| anyhow::anyhow!("Failed to parse {} as RFC3339: {}", field_name, e))
                .map(|dt| dt.with_timezone(&Utc))
        } else {
            // SQLite datetime format: "2025-10-29 07:17:16"
            chrono::NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S")
                .map_err(|e| {
                    anyhow::anyhow!("Failed to parse {} as SQLite datetime: {}", field_name, e)
                })
                .map(|ndt| ndt.and_utc())
        }
    }

    pub async fn create(conn: &Connection, req: CreateTagRequest) -> Result<Self> {
        info!(
            "[TradeTag::create] Starting tag creation with request: category={}, name={}, color={:?}, description={:?}",
            req.category, req.name, req.color, req.description
        );

        let id = Uuid::new_v4().to_string();
        info!("[TradeTag::create] Generated tag ID: {}", id);

        // Use RFC3339 format consistently with other models (ISO8601/RFC3339)
        let now = Self::to_db_datetime(Utc::now());
        debug!("[TradeTag::create] Generated timestamp: {}", now);

        // Log the actual values being inserted
        info!(
            "[TradeTag::create] Preparing INSERT with values: id={}, category={}, name={}, color={:?}, description={:?}, created_at={}, updated_at={}",
            id, req.category, req.name, req.color, req.description, now, now
        );

        // Prepare parameters - pass String values directly, clone where needed
        let category_val = req.category.clone();
        let name_val = req.name.clone();
        let color_val = req.color.as_deref();
        let desc_val = req.description.as_deref();
        let now_val = now.clone();

        debug!("[TradeTag::create] Parameter values prepared:");
        debug!("  id: {} (type: String, len: {})", id, id.len());
        debug!(
            "  category: {} (type: String, len: {})",
            category_val,
            category_val.len()
        );
        debug!(
            "  name: {} (type: String, len: {})",
            name_val,
            name_val.len()
        );
        debug!("  color: {:?} (type: Option<&str>)", color_val);
        debug!("  description: {:?} (type: Option<&str>)", desc_val);
        debug!(
            "  created_at: {} (type: String, len: {})",
            now_val,
            now_val.len()
        );
        debug!(
            "  updated_at: {} (type: String, len: {})",
            now_val,
            now_val.len()
        );

        let insert_result = conn.execute(
            "INSERT INTO trade_tags (id, category, name, color, description, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                id.clone(),            // Clone String
                category_val,          // String
                name_val,              // String
                color_val,             // Option<&str> for NULL
                desc_val,              // Option<&str> for NULL
                now_val.clone(),       // Clone String
                now_val                // String
            ],
        )
        .await;

        match &insert_result {
            Ok(_) => {
                info!(
                    "[TradeTag::create] ✓ INSERT executed successfully for tag ID: {}",
                    id
                );
            }
            Err(e) => {
                error!(
                    "[TradeTag::create] ✗ INSERT failed for tag ID {}: {}",
                    id, e
                );
                error!(
                    "[TradeTag::create] INSERT parameters were: id={}, category={}, name={}, color={:?}, description={:?}, created_at={}, updated_at={}",
                    id, req.category, req.name, req.color, req.description, now, now
                );
                return Err(anyhow::anyhow!("Failed to insert tag into database: {}", e));
            }
        }

        // Check if we need to unwrap the result
        insert_result?;

        info!("[TradeTag::create] Fetching created tag with ID: {}", id);
        match Self::find_by_id(conn, &id).await {
            Ok(tag) => {
                info!(
                    "[TradeTag::create] ✓ Successfully created and retrieved tag: id={}, category={}, name={}",
                    tag.id, tag.category, tag.name
                );
                Ok(tag)
            }
            Err(e) => {
                error!(
                    "[TradeTag::create] ✗ Failed to retrieve created tag with ID {}: {}",
                    id, e
                );
                Err(anyhow::anyhow!(
                    "Tag was created but failed to retrieve: {}",
                    e
                ))
            }
        }
    }

    pub async fn find_by_id(conn: &Connection, id: &str) -> Result<Self> {
        let stmt = conn
            .prepare("SELECT id, category, name, color, description, created_at, updated_at FROM trade_tags WHERE id = ?")
            .await?;
        let mut rows = stmt.query(params![id]).await?;

        if let Some(row) = rows.next().await? {
            Ok(Self::from_row(&row)?)
        } else {
            anyhow::bail!("Tag not found: {}", id)
        }
    }

    pub async fn find_all(conn: &Connection, query: Option<TagQuery>) -> Result<Vec<Self>> {
        let query = query.unwrap_or_default();
        let limit = query.limit.unwrap_or(100);
        let offset = query.offset.unwrap_or(0);

        if let Some(category) = &query.category {
            let stmt = conn
                .prepare("SELECT id, category, name, color, description, created_at, updated_at FROM trade_tags WHERE category = ? ORDER BY category, name LIMIT ? OFFSET ?")
                .await?;
            let mut rows = stmt.query(params![category.clone(), limit, offset]).await?;

            let mut tags = Vec::new();
            while let Some(row) = rows.next().await? {
                tags.push(Self::from_row(&row)?);
            }
            Ok(tags)
        } else {
            let stmt = conn
                .prepare("SELECT id, category, name, color, description, created_at, updated_at FROM trade_tags ORDER BY category, name LIMIT ? OFFSET ?")
                .await?;
            let mut rows = stmt.query(params![limit, offset]).await?;

            let mut tags = Vec::new();
            while let Some(row) = rows.next().await? {
                tags.push(Self::from_row(&row)?);
            }
            Ok(tags)
        }
    }

    #[allow(dead_code)]
    pub async fn find_by_category(conn: &Connection, category: &str) -> Result<Vec<Self>> {
        let stmt = conn
            .prepare("SELECT id, category, name, color, description, created_at, updated_at FROM trade_tags WHERE category = ? ORDER BY name")
            .await?;
        let mut rows = stmt.query(params![category]).await?;

        let mut tags = Vec::new();
        while let Some(row) = rows.next().await? {
            tags.push(Self::from_row(&row)?);
        }

        Ok(tags)
    }

    pub async fn get_categories(conn: &Connection) -> Result<Vec<String>> {
        let stmt = conn
            .prepare("SELECT DISTINCT category FROM trade_tags ORDER BY category")
            .await?;
        let mut rows = stmt.query(params![]).await?;

        let mut categories = Vec::new();
        while let Some(row) = rows.next().await? {
            categories.push(row.get(0)?);
        }

        Ok(categories)
    }

    pub async fn update(conn: &Connection, id: &str, req: UpdateTagRequest) -> Result<Self> {
        // Get existing tag
        let existing = Self::find_by_id(conn, id).await?;

        let category = req.category.unwrap_or(existing.category);
        let name = req.name.unwrap_or(existing.name);
        let color = req.color.or(existing.color);
        let description = req.description.or(existing.description);
        // Use RFC3339 format consistently with other models
        let updated_at = Self::to_db_datetime(Utc::now());

        conn.execute(
            "UPDATE trade_tags SET category = ?, name = ?, color = ?, description = ?, updated_at = ? WHERE id = ?",
            params![
                category,
                name,
                color.as_deref(),  // Option<String> -> Option<&str> for proper NULL handling
                description.as_deref(),  // Option<String> -> Option<&str> for proper NULL handling
                updated_at,
                id
            ],
        )
        .await?;

        Self::find_by_id(conn, id).await
    }

    pub async fn delete(conn: &Connection, id: &str) -> Result<bool> {
        let result = conn
            .execute("DELETE FROM trade_tags WHERE id = ?", params![id])
            .await?;

        Ok(result > 0)
    }

    fn from_row(row: &libsql::Row) -> Result<Self> {
        // Use helper functions for safe type conversion
        let id = Self::get_string(row, 0)?;
        let category = Self::get_string(row, 1)?;
        let name = Self::get_string(row, 2)?;
        let color = Self::get_opt_string(row, 3)?;
        let description = Self::get_opt_string(row, 4)?;

        // Safely get datetime strings
        let created_at_str = Self::get_string(row, 5)?;
        let updated_at_str = Self::get_string(row, 6)?;

        // Parse datetimes using helper function (handles both RFC3339 and SQLite format)
        let created_at = Self::parse_db_datetime(&created_at_str, "created_at")?;
        let updated_at = Self::parse_db_datetime(&updated_at_str, "updated_at")?;

        Ok(Self {
            id,
            category,
            name,
            color,
            description,
            created_at,
            updated_at,
        })
    }
}

impl Default for TagQuery {
    fn default() -> Self {
        Self {
            category: None,
            limit: Some(100),
            offset: Some(0),
        }
    }
}
