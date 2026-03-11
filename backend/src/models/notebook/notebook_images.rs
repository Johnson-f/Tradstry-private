use anyhow::Result;
use chrono::{DateTime, Utc};
use libsql::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookImage {
    pub id: String,
    pub note_id: String,
    pub src: String,
    pub storage_path: Option<String>,
    pub alt_text: Option<String>,
    pub caption: Option<String>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub position: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateImageRequest {
    pub note_id: String,
    pub src: String,
    pub storage_path: Option<String>,
    pub alt_text: Option<String>,
    pub caption: Option<String>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub position: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateImageRequest {
    pub src: Option<String>,
    pub alt_text: Option<String>,
    pub caption: Option<String>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub position: Option<i64>,
}

impl NotebookImage {
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

    fn get_i64(row: &libsql::Row, idx: i32) -> Result<i64> {
        match row.get_value(idx)? {
            libsql::Value::Integer(n) => Ok(n),
            libsql::Value::Text(s) => s.parse().map_err(|e| anyhow::anyhow!("Failed to parse i64: {}", e)),
            libsql::Value::Null => Ok(0),
            _ => Ok(0),
        }
    }

    fn get_opt_i64(row: &libsql::Row, idx: i32) -> Result<Option<i64>> {
        match row.get_value(idx)? {
            libsql::Value::Integer(n) => Ok(Some(n)),
            libsql::Value::Text(s) => Ok(s.parse().ok()),
            libsql::Value::Null => Ok(None),
            _ => Ok(None),
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

    pub async fn create(conn: &Connection, req: CreateImageRequest) -> Result<Self> {
        let id = Uuid::new_v4().to_string();
        let now = Self::to_db_datetime(Utc::now());
        let position = req.position.unwrap_or(0);

        conn.execute(
            "INSERT INTO notebook_images (id, note_id, src, storage_path, alt_text, caption, width, height, position, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                id.clone(),
                req.note_id,
                req.src,
                req.storage_path.as_deref(),
                req.alt_text.as_deref(),
                req.caption.as_deref(),
                req.width,
                req.height,
                position,
                now.clone(),
                now
            ],
        )
        .await?;

        Self::find_by_id(conn, &id).await
    }

    pub async fn find_by_id(conn: &Connection, id: &str) -> Result<Self> {
        let stmt = conn
            .prepare("SELECT id, note_id, src, storage_path, alt_text, caption, width, height, position, created_at, updated_at FROM notebook_images WHERE id = ?")
            .await?;
        let mut rows = stmt.query(params![id]).await?;

        if let Some(row) = rows.next().await? {
            Self::from_row(&row)
        } else {
            anyhow::bail!("Image not found: {}", id)
        }
    }

    pub async fn find_by_note(conn: &Connection, note_id: &str) -> Result<Vec<Self>> {
        let stmt = conn
            .prepare("SELECT id, note_id, src, storage_path, alt_text, caption, width, height, position, created_at, updated_at FROM notebook_images WHERE note_id = ? ORDER BY position")
            .await?;
        let mut rows = stmt.query(params![note_id]).await?;

        let mut images = Vec::new();
        while let Some(row) = rows.next().await? {
            images.push(Self::from_row(&row)?);
        }
        Ok(images)
    }

    pub async fn update(conn: &Connection, id: &str, req: UpdateImageRequest) -> Result<Self> {
        let existing = Self::find_by_id(conn, id).await?;
        let src = req.src.unwrap_or(existing.src);
        let alt_text = req.alt_text.or(existing.alt_text);
        let caption = req.caption.or(existing.caption);
        let width = req.width.or(existing.width);
        let height = req.height.or(existing.height);
        let position = req.position.unwrap_or(existing.position);
        let updated_at = Self::to_db_datetime(Utc::now());

        conn.execute(
            "UPDATE notebook_images SET src = ?, alt_text = ?, caption = ?, width = ?, height = ?, position = ?, updated_at = ? WHERE id = ?",
            params![src, alt_text.as_deref(), caption.as_deref(), width, height, position, updated_at, id],
        )
        .await?;

        Self::find_by_id(conn, id).await
    }

    pub async fn delete(conn: &Connection, id: &str) -> Result<bool> {
        let result = conn
            .execute("DELETE FROM notebook_images WHERE id = ?", params![id])
            .await?;
        Ok(result > 0)
    }

    pub async fn delete_by_note(conn: &Connection, note_id: &str) -> Result<u64> {
        let result = conn
            .execute("DELETE FROM notebook_images WHERE note_id = ?", params![note_id])
            .await?;
        Ok(result)
    }

    pub async fn reorder(conn: &Connection, id: &str, new_position: i64) -> Result<Self> {
        let updated_at = Self::to_db_datetime(Utc::now());
        conn.execute(
            "UPDATE notebook_images SET position = ?, updated_at = ? WHERE id = ?",
            params![new_position, updated_at, id],
        )
        .await?;

        Self::find_by_id(conn, id).await
    }

    fn from_row(row: &libsql::Row) -> Result<Self> {
        let id = Self::get_string(row, 0)?;
        let note_id = Self::get_string(row, 1)?;
        let src = Self::get_string(row, 2)?;
        let storage_path = Self::get_opt_string(row, 3)?;
        let alt_text = Self::get_opt_string(row, 4)?;
        let caption = Self::get_opt_string(row, 5)?;
        let width = Self::get_opt_i64(row, 6)?;
        let height = Self::get_opt_i64(row, 7)?;
        let position = Self::get_i64(row, 8)?;
        let created_at_str = Self::get_string(row, 9)?;
        let updated_at_str = Self::get_string(row, 10)?;

        Ok(Self {
            id,
            note_id,
            src,
            storage_path,
            alt_text,
            caption,
            width,
            height,
            position,
            created_at: Self::parse_db_datetime(&created_at_str, "created_at")?,
            updated_at: Self::parse_db_datetime(&updated_at_str, "updated_at")?,
        })
    }
}
