use anyhow::Result;
use chrono::{DateTime, Utc};
use libsql::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookTag {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub is_favorite: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
    pub color: Option<String>,
    pub is_favorite: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTagRequest {
    pub name: Option<String>,
    pub color: Option<String>,
    pub is_favorite: Option<bool>,
}

impl NotebookTag {
    fn get_string(row: &libsql::Row, idx: i32) -> Result<String> {
        match row.get_value(idx)? {
            libsql::Value::Text(s) => Ok(s),
            libsql::Value::Null => anyhow::bail!("Unexpected NULL value at index {}", idx),
            libsql::Value::Integer(n) => Ok(n.to_string()),
            libsql::Value::Real(r) => Ok(r.to_string()),
            libsql::Value::Blob(_) => anyhow::bail!("Unexpected BLOB value at index {}", idx),
        }
    }

    fn get_opt_string(row: &libsql::Row, idx: i32) -> Result<Option<String>> {
        match row.get_value(idx)? {
            libsql::Value::Text(s) => Ok(Some(s)),
            libsql::Value::Null => Ok(None),
            libsql::Value::Integer(n) => Ok(Some(n.to_string())),
            libsql::Value::Real(r) => Ok(Some(r.to_string())),
            libsql::Value::Blob(_) => Ok(None),
        }
    }

    fn get_bool(row: &libsql::Row, idx: i32) -> Result<bool> {
        match row.get_value(idx)? {
            libsql::Value::Integer(n) => Ok(n != 0),
            libsql::Value::Text(s) => Ok(s == "true" || s == "1"),
            libsql::Value::Null => Ok(false),
            _ => Ok(false),
        }
    }

    fn to_db_datetime(dt: DateTime<Utc>) -> String {
        dt.to_rfc3339()
    }

    fn parse_db_datetime(datetime_str: &str, field_name: &str) -> Result<DateTime<Utc>> {
        if datetime_str.contains('T') {
            DateTime::parse_from_rfc3339(datetime_str)
                .map_err(|e| anyhow::anyhow!("Failed to parse {} as RFC3339: {}", field_name, e))
                .map(|dt| dt.with_timezone(&Utc))
        } else {
            chrono::NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S")
                .map_err(|e| anyhow::anyhow!("Failed to parse {} as SQLite datetime: {}", field_name, e))
                .map(|ndt| ndt.and_utc())
        }
    }

    pub async fn create(conn: &Connection, req: CreateTagRequest) -> Result<Self> {
        let id = Uuid::new_v4().to_string();
        let now = Self::to_db_datetime(Utc::now());
        let is_favorite = req.is_favorite.unwrap_or(false);

        conn.execute(
            "INSERT INTO notebook_tags (id, name, color, is_favorite, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)",
            params![
                id.clone(),
                req.name,
                req.color.as_deref(),
                is_favorite,
                now.clone(),
                now
            ],
        )
        .await?;

        Self::find_by_id(conn, &id).await
    }

    pub async fn find_by_id(conn: &Connection, id: &str) -> Result<Self> {
        let stmt = conn
            .prepare("SELECT id, name, color, is_favorite, created_at, updated_at FROM notebook_tags WHERE id = ?")
            .await?;
        let mut rows = stmt.query(params![id]).await?;

        if let Some(row) = rows.next().await? {
            Self::from_row(&row)
        } else {
            anyhow::bail!("Tag not found: {}", id)
        }
    }

    pub async fn find_by_name(conn: &Connection, name: &str) -> Result<Option<Self>> {
        let stmt = conn
            .prepare("SELECT id, name, color, is_favorite, created_at, updated_at FROM notebook_tags WHERE name = ?")
            .await?;
        let mut rows = stmt.query(params![name]).await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(Self::from_row(&row)?))
        } else {
            Ok(None)
        }
    }

    pub async fn find_all(conn: &Connection) -> Result<Vec<Self>> {
        let stmt = conn
            .prepare("SELECT id, name, color, is_favorite, created_at, updated_at FROM notebook_tags ORDER BY is_favorite DESC, name")
            .await?;
        let mut rows = stmt.query(params![]).await?;

        let mut tags = Vec::new();
        while let Some(row) = rows.next().await? {
            tags.push(Self::from_row(&row)?);
        }
        Ok(tags)
    }

    pub async fn find_favorites(conn: &Connection) -> Result<Vec<Self>> {
        let stmt = conn
            .prepare("SELECT id, name, color, is_favorite, created_at, updated_at FROM notebook_tags WHERE is_favorite = 1 ORDER BY name")
            .await?;
        let mut rows = stmt.query(params![]).await?;

        let mut tags = Vec::new();
        while let Some(row) = rows.next().await? {
            tags.push(Self::from_row(&row)?);
        }
        Ok(tags)
    }

    pub async fn update(conn: &Connection, id: &str, req: UpdateTagRequest) -> Result<Self> {
        let existing = Self::find_by_id(conn, id).await?;
        let name = req.name.unwrap_or(existing.name);
        let color = req.color.or(existing.color);
        let is_favorite = req.is_favorite.unwrap_or(existing.is_favorite);
        let updated_at = Self::to_db_datetime(Utc::now());

        conn.execute(
            "UPDATE notebook_tags SET name = ?, color = ?, is_favorite = ?, updated_at = ? WHERE id = ?",
            params![name, color.as_deref(), is_favorite, updated_at, id],
        )
        .await?;

        Self::find_by_id(conn, id).await
    }

    pub async fn delete(conn: &Connection, id: &str) -> Result<bool> {
        let result = conn
            .execute("DELETE FROM notebook_tags WHERE id = ?", params![id])
            .await?;
        Ok(result > 0)
    }

    // Junction table operations
    pub async fn tag_note(conn: &Connection, note_id: &str, tag_id: &str) -> Result<()> {
        let now = Self::to_db_datetime(Utc::now());
        conn.execute(
            "INSERT OR IGNORE INTO notebook_note_tags (note_id, tag_id, created_at) VALUES (?, ?, ?)",
            params![note_id, tag_id, now],
        )
        .await?;
        Ok(())
    }

    pub async fn untag_note(conn: &Connection, note_id: &str, tag_id: &str) -> Result<bool> {
        let result = conn
            .execute(
                "DELETE FROM notebook_note_tags WHERE note_id = ? AND tag_id = ?",
                params![note_id, tag_id],
            )
            .await?;
        Ok(result > 0)
    }

    pub async fn get_note_tags(conn: &Connection, note_id: &str) -> Result<Vec<Self>> {
        let stmt = conn
            .prepare(
                "SELECT t.id, t.name, t.color, t.is_favorite, t.created_at, t.updated_at 
                 FROM notebook_tags t 
                 INNER JOIN notebook_note_tags nt ON t.id = nt.tag_id 
                 WHERE nt.note_id = ? 
                 ORDER BY t.name"
            )
            .await?;
        let mut rows = stmt.query(params![note_id]).await?;

        let mut tags = Vec::new();
        while let Some(row) = rows.next().await? {
            tags.push(Self::from_row(&row)?);
        }
        Ok(tags)
    }

    pub async fn get_notes_by_tag(conn: &Connection, tag_id: &str) -> Result<Vec<String>> {
        let stmt = conn
            .prepare("SELECT note_id FROM notebook_note_tags WHERE tag_id = ?")
            .await?;
        let mut rows = stmt.query(params![tag_id]).await?;

        let mut note_ids = Vec::new();
        while let Some(row) = rows.next().await? {
            note_ids.push(Self::get_string(&row, 0)?);
        }
        Ok(note_ids)
    }

    pub async fn set_note_tags(conn: &Connection, note_id: &str, tag_ids: &[String]) -> Result<()> {
        // Remove all existing tags
        conn.execute(
            "DELETE FROM notebook_note_tags WHERE note_id = ?",
            params![note_id],
        )
        .await?;

        // Add new tags
        let now = Self::to_db_datetime(Utc::now());
        for tag_id in tag_ids {
            conn.execute(
                "INSERT INTO notebook_note_tags (note_id, tag_id, created_at) VALUES (?, ?, ?)",
                params![note_id, tag_id.clone(), now.clone()],
            )
            .await?;
        }
        Ok(())
    }

    fn from_row(row: &libsql::Row) -> Result<Self> {
        let id = Self::get_string(row, 0)?;
        let name = Self::get_string(row, 1)?;
        let color = Self::get_opt_string(row, 2)?;
        let is_favorite = Self::get_bool(row, 3)?;
        let created_at_str = Self::get_string(row, 4)?;
        let updated_at_str = Self::get_string(row, 5)?;

        Ok(Self {
            id,
            name,
            color,
            is_favorite,
            created_at: Self::parse_db_datetime(&created_at_str, "created_at")?,
            updated_at: Self::parse_db_datetime(&updated_at_str, "updated_at")?,
        })
    }
}
