use anyhow::Result;
use chrono::{DateTime, Utc};
use libsql::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookFolder {
    pub id: String,
    pub name: String,
    pub is_favorite: bool,
    pub parent_folder_id: Option<String>,
    pub position: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateFolderRequest {
    pub name: String,
    pub is_favorite: Option<bool>,
    pub parent_folder_id: Option<String>,
    pub position: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateFolderRequest {
    pub name: Option<String>,
    pub is_favorite: Option<bool>,
    pub parent_folder_id: Option<Option<String>>,
    pub position: Option<i64>,
}

impl NotebookFolder {
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

    fn get_i64(row: &libsql::Row, idx: i32) -> Result<i64> {
        match row.get_value(idx)? {
            libsql::Value::Integer(n) => Ok(n),
            libsql::Value::Text(s) => s.parse().map_err(|e| anyhow::anyhow!("Failed to parse i64: {}", e)),
            libsql::Value::Null => Ok(0),
            _ => Ok(0),
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

    pub async fn create(conn: &Connection, req: CreateFolderRequest) -> Result<Self> {
        let id = Uuid::new_v4().to_string();
        let now = Self::to_db_datetime(Utc::now());
        let is_favorite = req.is_favorite.unwrap_or(false);
        let position = req.position.unwrap_or(0);

        conn.execute(
            "INSERT INTO notebook_folders (id, name, is_favorite, parent_folder_id, position, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                id.clone(),
                req.name,
                is_favorite,
                req.parent_folder_id.as_deref(),
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
            .prepare("SELECT id, name, is_favorite, parent_folder_id, position, created_at, updated_at FROM notebook_folders WHERE id = ?")
            .await?;
        let mut rows = stmt.query(params![id]).await?;

        if let Some(row) = rows.next().await? {
            Self::from_row(&row)
        } else {
            anyhow::bail!("Folder not found: {}", id)
        }
    }

    pub async fn find_all(conn: &Connection) -> Result<Vec<Self>> {
        let stmt = conn
            .prepare("SELECT id, name, is_favorite, parent_folder_id, position, created_at, updated_at FROM notebook_folders ORDER BY position, name")
            .await?;
        let mut rows = stmt.query(params![]).await?;

        let mut folders = Vec::new();
        while let Some(row) = rows.next().await? {
            folders.push(Self::from_row(&row)?);
        }
        Ok(folders)
    }

    pub async fn find_by_parent(conn: &Connection, parent_id: Option<&str>) -> Result<Vec<Self>> {
        let stmt = if parent_id.is_some() {
            conn.prepare("SELECT id, name, is_favorite, parent_folder_id, position, created_at, updated_at FROM notebook_folders WHERE parent_folder_id = ? ORDER BY position, name")
                .await?
        } else {
            conn.prepare("SELECT id, name, is_favorite, parent_folder_id, position, created_at, updated_at FROM notebook_folders WHERE parent_folder_id IS NULL ORDER BY position, name")
                .await?
        };

        let mut rows = if let Some(pid) = parent_id {
            stmt.query(params![pid]).await?
        } else {
            stmt.query(params![]).await?
        };

        let mut folders = Vec::new();
        while let Some(row) = rows.next().await? {
            folders.push(Self::from_row(&row)?);
        }
        Ok(folders)
    }

    pub async fn find_favorites(conn: &Connection) -> Result<Vec<Self>> {
        let stmt = conn
            .prepare("SELECT id, name, is_favorite, parent_folder_id, position, created_at, updated_at FROM notebook_folders WHERE is_favorite = 1 ORDER BY position, name")
            .await?;
        let mut rows = stmt.query(params![]).await?;

        let mut folders = Vec::new();
        while let Some(row) = rows.next().await? {
            folders.push(Self::from_row(&row)?);
        }
        Ok(folders)
    }

    pub async fn update(conn: &Connection, id: &str, req: UpdateFolderRequest) -> Result<Self> {
        let existing = Self::find_by_id(conn, id).await?;
        let name = req.name.unwrap_or(existing.name);
        let is_favorite = req.is_favorite.unwrap_or(existing.is_favorite);
        let parent_folder_id = req.parent_folder_id.unwrap_or(existing.parent_folder_id);
        let position = req.position.unwrap_or(existing.position);
        let updated_at = Self::to_db_datetime(Utc::now());

        conn.execute(
            "UPDATE notebook_folders SET name = ?, is_favorite = ?, parent_folder_id = ?, position = ?, updated_at = ? WHERE id = ?",
            params![name, is_favorite, parent_folder_id.as_deref(), position, updated_at, id],
        )
        .await?;

        Self::find_by_id(conn, id).await
    }

    pub async fn delete(conn: &Connection, id: &str) -> Result<bool> {
        let result = conn
            .execute("DELETE FROM notebook_folders WHERE id = ?", params![id])
            .await?;
        Ok(result > 0)
    }

    fn from_row(row: &libsql::Row) -> Result<Self> {
        let id = Self::get_string(row, 0)?;
        let name = Self::get_string(row, 1)?;
        let is_favorite = Self::get_bool(row, 2)?;
        let parent_folder_id = Self::get_opt_string(row, 3)?;
        let position = Self::get_i64(row, 4)?;
        let created_at_str = Self::get_string(row, 5)?;
        let updated_at_str = Self::get_string(row, 6)?;

        Ok(Self {
            id,
            name,
            is_favorite,
            parent_folder_id,
            position,
            created_at: Self::parse_db_datetime(&created_at_str, "created_at")?,
            updated_at: Self::parse_db_datetime(&updated_at_str, "updated_at")?,
        })
    }
}
