use anyhow::Result;
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::{DateTime, Utc};
use libsql::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookNote {
    pub id: String,
    pub title: String,
    pub content: Value,
    pub content_plain_text: Option<String>,
    pub word_count: i64,
    pub is_pinned: bool,
    pub is_archived: bool,
    pub is_deleted: bool,
    pub folder_id: Option<String>,
    pub tags: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "base64_option")]
    pub y_state: Option<Vec<u8>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_synced_at: Option<DateTime<Utc>>,
}

// Custom serialization for Option<Vec<u8>> as base64
mod base64_option {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(data: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match data {
            Some(bytes) => serializer.serialize_str(&STANDARD.encode(bytes)),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt: Option<String> = Option::deserialize(deserializer)?;
        match opt {
            Some(s) => STANDARD
                .decode(&s)
                .map(Some)
                .map_err(serde::de::Error::custom),
            None => Ok(None),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateNoteRequest {
    pub title: Option<String>,
    pub content: Option<Value>,
    pub content_plain_text: Option<String>,
    pub folder_id: Option<String>,
    pub tags: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNoteRequest {
    pub title: Option<String>,
    pub content: Option<Value>,
    pub content_plain_text: Option<String>,
    pub is_pinned: Option<bool>,
    pub is_archived: Option<bool>,
    pub is_deleted: Option<bool>,
    pub folder_id: Option<Option<String>>,
    pub tags: Option<Value>,
    #[serde(default)]
    #[serde(with = "base64_option")]
    pub y_state: Option<Vec<u8>>,
}

#[derive(Debug, Deserialize)]
pub struct NoteQuery {
    pub folder_id: Option<String>,
    pub is_pinned: Option<bool>,
    pub is_archived: Option<bool>,
    pub is_deleted: Option<bool>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl NotebookNote {
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

    fn get_opt_blob(row: &libsql::Row, idx: i32) -> Result<Option<Vec<u8>>> {
        match row.get_value(idx)? {
            libsql::Value::Blob(b) => Ok(Some(b)),
            libsql::Value::Null => Ok(None),
            _ => Ok(None),
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

    fn count_words(text: &str) -> i64 {
        text.split_whitespace().count() as i64
    }

    pub async fn create(conn: &Connection, req: CreateNoteRequest) -> Result<Self> {
        let id = Uuid::new_v4().to_string();
        let now = Self::to_db_datetime(Utc::now());
        let title = req.title.unwrap_or_else(|| "Untitled".to_string());
        let content = req.content.unwrap_or_else(|| serde_json::json!({}));
        let content_str = serde_json::to_string(&content)?;
        let word_count = req.content_plain_text.as_ref().map(|t| Self::count_words(t)).unwrap_or(0);
        let tags_str = req.tags.as_ref().map(|t| serde_json::to_string(t).ok()).flatten();

        conn.execute(
            "INSERT INTO notebook_notes (id, title, content, content_plain_text, word_count, is_pinned, is_archived, is_deleted, folder_id, tags, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                id.clone(),
                title,
                content_str,
                req.content_plain_text.as_deref(),
                word_count,
                false,
                false,
                false,
                req.folder_id.as_deref(),
                tags_str.as_deref(),
                now.clone(),
                now
            ],
        )
        .await?;

        Self::find_by_id(conn, &id).await
    }

    pub async fn find_by_id(conn: &Connection, id: &str) -> Result<Self> {
        let stmt = conn
            .prepare("SELECT id, title, content, content_plain_text, word_count, is_pinned, is_archived, is_deleted, folder_id, tags, y_state, created_at, updated_at, last_synced_at FROM notebook_notes WHERE id = ?")
            .await?;
        let mut rows = stmt.query(params![id]).await?;

        if let Some(row) = rows.next().await? {
            Self::from_row(&row)
        } else {
            anyhow::bail!("Note not found: {}", id)
        }
    }

    pub async fn find_all(conn: &Connection, query: Option<NoteQuery>) -> Result<Vec<Self>> {
        let query = query.unwrap_or_default();
        let limit = query.limit.unwrap_or(100);
        let offset = query.offset.unwrap_or(0);
        let is_deleted = query.is_deleted.unwrap_or(false);

        let mut sql = String::from(
            "SELECT id, title, content, content_plain_text, word_count, is_pinned, is_archived, is_deleted, folder_id, tags, y_state, created_at, updated_at, last_synced_at FROM notebook_notes WHERE is_deleted = ?"
        );
        let mut params_vec: Vec<libsql::Value> = vec![is_deleted.into()];

        if let Some(folder_id) = &query.folder_id {
            sql.push_str(" AND folder_id = ?");
            params_vec.push(folder_id.clone().into());
        }

        if let Some(is_pinned) = query.is_pinned {
            sql.push_str(" AND is_pinned = ?");
            params_vec.push(is_pinned.into());
        }

        if let Some(is_archived) = query.is_archived {
            sql.push_str(" AND is_archived = ?");
            params_vec.push(is_archived.into());
        }

        if let Some(search) = &query.search {
            sql.push_str(" AND (title LIKE ? OR content_plain_text LIKE ?)");
            let search_pattern = format!("%{}%", search);
            params_vec.push(search_pattern.clone().into());
            params_vec.push(search_pattern.into());
        }

        sql.push_str(" ORDER BY is_pinned DESC, updated_at DESC LIMIT ? OFFSET ?");
        params_vec.push(limit.into());
        params_vec.push(offset.into());

        let stmt = conn.prepare(&sql).await?;
        let mut rows = stmt.query(params_vec).await?;

        let mut notes = Vec::new();
        while let Some(row) = rows.next().await? {
            notes.push(Self::from_row(&row)?);
        }
        Ok(notes)
    }

    pub async fn find_by_folder(conn: &Connection, folder_id: Option<&str>) -> Result<Vec<Self>> {
        let stmt = if folder_id.is_some() {
            conn.prepare("SELECT id, title, content, content_plain_text, word_count, is_pinned, is_archived, is_deleted, folder_id, tags, y_state, created_at, updated_at, last_synced_at FROM notebook_notes WHERE folder_id = ? AND is_deleted = 0 ORDER BY is_pinned DESC, updated_at DESC")
                .await?
        } else {
            conn.prepare("SELECT id, title, content, content_plain_text, word_count, is_pinned, is_archived, is_deleted, folder_id, tags, y_state, created_at, updated_at, last_synced_at FROM notebook_notes WHERE folder_id IS NULL AND is_deleted = 0 ORDER BY is_pinned DESC, updated_at DESC")
                .await?
        };

        let mut rows = if let Some(fid) = folder_id {
            stmt.query(params![fid]).await?
        } else {
            stmt.query(params![]).await?
        };

        let mut notes = Vec::new();
        while let Some(row) = rows.next().await? {
            notes.push(Self::from_row(&row)?);
        }
        Ok(notes)
    }

    pub async fn find_deleted(conn: &Connection) -> Result<Vec<Self>> {
        let stmt = conn
            .prepare("SELECT id, title, content, content_plain_text, word_count, is_pinned, is_archived, is_deleted, folder_id, tags, y_state, created_at, updated_at, last_synced_at FROM notebook_notes WHERE is_deleted = 1 ORDER BY updated_at DESC")
            .await?;
        let mut rows = stmt.query(params![]).await?;

        let mut notes = Vec::new();
        while let Some(row) = rows.next().await? {
            notes.push(Self::from_row(&row)?);
        }
        Ok(notes)
    }

    pub async fn update(conn: &Connection, id: &str, req: UpdateNoteRequest) -> Result<Self> {
        let existing = Self::find_by_id(conn, id).await?;
        let title = req.title.unwrap_or(existing.title);
        let content = req.content.unwrap_or(existing.content);
        let content_str = serde_json::to_string(&content)?;
        let content_plain_text = req.content_plain_text.or(existing.content_plain_text);
        let word_count = content_plain_text.as_ref().map(|t| Self::count_words(t)).unwrap_or(existing.word_count);
        let is_pinned = req.is_pinned.unwrap_or(existing.is_pinned);
        let is_archived = req.is_archived.unwrap_or(existing.is_archived);
        let is_deleted = req.is_deleted.unwrap_or(existing.is_deleted);
        let folder_id = req.folder_id.unwrap_or(existing.folder_id);
        let tags = req.tags.or(existing.tags);
        let tags_str = tags.as_ref().map(|t| serde_json::to_string(t).ok()).flatten();
        let updated_at = Self::to_db_datetime(Utc::now());

        conn.execute(
            "UPDATE notebook_notes SET title = ?, content = ?, content_plain_text = ?, word_count = ?, is_pinned = ?, is_archived = ?, is_deleted = ?, folder_id = ?, tags = ?, updated_at = ? WHERE id = ?",
            params![
                title,
                content_str,
                content_plain_text.as_deref(),
                word_count,
                is_pinned,
                is_archived,
                is_deleted,
                folder_id.as_deref(),
                tags_str.as_deref(),
                updated_at,
                id
            ],
        )
        .await?;

        Self::find_by_id(conn, id).await
    }

    pub async fn soft_delete(conn: &Connection, id: &str) -> Result<bool> {
        let updated_at = Self::to_db_datetime(Utc::now());
        let result = conn
            .execute(
                "UPDATE notebook_notes SET is_deleted = 1, updated_at = ? WHERE id = ?",
                params![updated_at, id],
            )
            .await?;
        Ok(result > 0)
    }

    pub async fn restore(conn: &Connection, id: &str) -> Result<bool> {
        let updated_at = Self::to_db_datetime(Utc::now());
        let result = conn
            .execute(
                "UPDATE notebook_notes SET is_deleted = 0, updated_at = ? WHERE id = ?",
                params![updated_at, id],
            )
            .await?;
        Ok(result > 0)
    }

    pub async fn permanent_delete(conn: &Connection, id: &str) -> Result<bool> {
        let result = conn
            .execute("DELETE FROM notebook_notes WHERE id = ?", params![id])
            .await?;
        Ok(result > 0)
    }

    pub async fn update_sync_time(conn: &Connection, id: &str) -> Result<bool> {
        let now = Self::to_db_datetime(Utc::now());
        let result = conn
            .execute(
                "UPDATE notebook_notes SET last_synced_at = ?, updated_at = ? WHERE id = ?",
                params![now.clone(), now, id],
            )
            .await?;
        Ok(result > 0)
    }

    /// Update the Y.js CRDT state for a note (used by collaboration system)
    pub async fn update_y_state(conn: &Connection, id: &str, y_state: &[u8]) -> Result<bool> {
        let updated_at = Self::to_db_datetime(Utc::now());
        let result = conn
            .execute(
                "UPDATE notebook_notes SET y_state = ?, updated_at = ? WHERE id = ?",
                params![y_state, updated_at, id],
            )
            .await?;
        Ok(result > 0)
    }

    /// Update both Y.js state and Lexical content (used when persisting collaboration changes)
    pub async fn update_y_state_and_content(
        conn: &Connection,
        id: &str,
        y_state: &[u8],
        content: &Value,
        content_plain_text: Option<&str>,
    ) -> Result<bool> {
        let updated_at = Self::to_db_datetime(Utc::now());
        let content_str = serde_json::to_string(content)?;
        let word_count = content_plain_text.map(Self::count_words).unwrap_or(0);
        
        let result = conn
            .execute(
                "UPDATE notebook_notes SET y_state = ?, content = ?, content_plain_text = ?, word_count = ?, updated_at = ? WHERE id = ?",
                params![y_state, content_str, content_plain_text, word_count, updated_at, id],
            )
            .await?;
        Ok(result > 0)
    }

    /// Get only the Y.js state for a note (efficient for collaboration loading)
    pub async fn get_y_state(conn: &Connection, id: &str) -> Result<Option<Vec<u8>>> {
        let stmt = conn
            .prepare("SELECT y_state FROM notebook_notes WHERE id = ?")
            .await?;
        let mut rows = stmt.query(params![id]).await?;

        if let Some(row) = rows.next().await? {
            Self::get_opt_blob(&row, 0)
        } else {
            anyhow::bail!("Note not found: {}", id)
        }
    }

    fn from_row(row: &libsql::Row) -> Result<Self> {
        let id = Self::get_string(row, 0)?;
        let title = Self::get_string(row, 1)?;
        let content_str = Self::get_string(row, 2)?;
        let content: Value = serde_json::from_str(&content_str).unwrap_or_else(|_| serde_json::json!({}));
        let content_plain_text = Self::get_opt_string(row, 3)?;
        let word_count = Self::get_i64(row, 4)?;
        let is_pinned = Self::get_bool(row, 5)?;
        let is_archived = Self::get_bool(row, 6)?;
        let is_deleted = Self::get_bool(row, 7)?;
        let folder_id = Self::get_opt_string(row, 8)?;
        let tags_str = Self::get_opt_string(row, 9)?;
        let tags: Option<Value> = tags_str.and_then(|s| serde_json::from_str(&s).ok());
        let y_state = Self::get_opt_blob(row, 10)?;
        let created_at_str = Self::get_string(row, 11)?;
        let updated_at_str = Self::get_string(row, 12)?;
        let last_synced_at_str = Self::get_opt_string(row, 13)?;

        Ok(Self {
            id,
            title,
            content,
            content_plain_text,
            word_count,
            is_pinned,
            is_archived,
            is_deleted,
            folder_id,
            tags,
            y_state,
            created_at: Self::parse_db_datetime(&created_at_str, "created_at")?,
            updated_at: Self::parse_db_datetime(&updated_at_str, "updated_at")?,
            last_synced_at: last_synced_at_str.and_then(|s| Self::parse_db_datetime(&s, "last_synced_at").ok()),
        })
    }
}

impl Default for NoteQuery {
    fn default() -> Self {
        Self {
            folder_id: None,
            is_pinned: None,
            is_archived: None,
            is_deleted: Some(false),
            search: None,
            limit: Some(100),
            offset: Some(0),
        }
    }
}
