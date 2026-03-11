use anyhow::Result;
use chrono::{DateTime, Utc};
use libsql::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Note visibility level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NoteVisibility {
    Private,   // Only owner can access
    Shared,    // Owner + collaborators
    Public,    // Anyone with link (read-only)
}

impl NoteVisibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            NoteVisibility::Private => "private",
            NoteVisibility::Shared => "shared",
            NoteVisibility::Public => "public",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "private" => Ok(NoteVisibility::Private),
            "shared" => Ok(NoteVisibility::Shared),
            "public" => Ok(NoteVisibility::Public),
            _ => anyhow::bail!("Invalid visibility: {}", s),
        }
    }
}

/// Note sharing settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteShareSettings {
    pub id: String,
    pub note_id: String,
    pub owner_id: String,
    pub visibility: NoteVisibility,
    pub public_slug: Option<String>,
    pub allow_comments: bool,
    pub allow_copy: bool,
    pub password_hash: Option<String>,
    pub view_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateShareSettingsRequest {
    pub note_id: String,
    pub owner_id: String,
    pub visibility: Option<NoteVisibility>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateShareSettingsRequest {
    pub visibility: Option<NoteVisibility>,
    pub allow_comments: Option<bool>,
    pub allow_copy: Option<bool>,
    pub password: Option<Option<String>>,
}

impl NoteShareSettings {
    fn get_string(row: &libsql::Row, idx: i32) -> Result<String> {
        match row.get_value(idx)? {
            libsql::Value::Text(s) => Ok(s),
            libsql::Value::Null => anyhow::bail!("Unexpected NULL at index {}", idx),
            libsql::Value::Integer(n) => Ok(n.to_string()),
            _ => anyhow::bail!("Unexpected value type at index {}", idx),
        }
    }

    fn get_opt_string(row: &libsql::Row, idx: i32) -> Result<Option<String>> {
        match row.get_value(idx)? {
            libsql::Value::Text(s) => Ok(Some(s)),
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
                .map_err(|e| anyhow::anyhow!("Failed to parse {}: {}", field_name, e))
                .map(|ndt| ndt.and_utc())
        }
    }

    fn generate_slug() -> String {
        Uuid::new_v4().to_string().replace("-", "").chars().take(12).collect()
    }

    pub async fn create(conn: &Connection, req: CreateShareSettingsRequest) -> Result<Self> {
        let id = Uuid::new_v4().to_string();
        let now = Self::to_db_datetime(Utc::now());
        let visibility = req.visibility.unwrap_or(NoteVisibility::Private);
        let public_slug = if visibility == NoteVisibility::Public {
            Some(Self::generate_slug())
        } else {
            None
        };

        conn.execute(
            "INSERT INTO note_share_settings (id, note_id, owner_id, visibility, public_slug, allow_comments, allow_copy, view_count, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                id.clone(),
                req.note_id,
                req.owner_id,
                visibility.as_str(),
                public_slug.as_deref(),
                false,
                true,
                0i64,
                now.clone(),
                now
            ],
        )
        .await?;

        Self::find_by_id(conn, &id).await
    }

    pub async fn find_by_id(conn: &Connection, id: &str) -> Result<Self> {
        let stmt = conn
            .prepare("SELECT id, note_id, owner_id, visibility, public_slug, allow_comments, allow_copy, password_hash, view_count, created_at, updated_at FROM note_share_settings WHERE id = ?")
            .await?;
        let mut rows = stmt.query(params![id]).await?;

        if let Some(row) = rows.next().await? {
            Self::from_row(&row)
        } else {
            anyhow::bail!("Share settings not found: {}", id)
        }
    }

    pub async fn find_by_note(conn: &Connection, note_id: &str) -> Result<Option<Self>> {
        let stmt = conn
            .prepare("SELECT id, note_id, owner_id, visibility, public_slug, allow_comments, allow_copy, password_hash, view_count, created_at, updated_at FROM note_share_settings WHERE note_id = ?")
            .await?;
        let mut rows = stmt.query(params![note_id]).await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(Self::from_row(&row)?))
        } else {
            Ok(None)
        }
    }

    pub async fn find_by_slug(conn: &Connection, slug: &str) -> Result<Option<Self>> {
        let stmt = conn
            .prepare("SELECT id, note_id, owner_id, visibility, public_slug, allow_comments, allow_copy, password_hash, view_count, created_at, updated_at FROM note_share_settings WHERE public_slug = ?")
            .await?;
        let mut rows = stmt.query(params![slug]).await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(Self::from_row(&row)?))
        } else {
            Ok(None)
        }
    }

    pub async fn update(conn: &Connection, id: &str, req: UpdateShareSettingsRequest) -> Result<Self> {
        let existing = Self::find_by_id(conn, id).await?;
        let visibility = req.visibility.unwrap_or(existing.visibility);
        let allow_comments = req.allow_comments.unwrap_or(existing.allow_comments);
        let allow_copy = req.allow_copy.unwrap_or(existing.allow_copy);
        let updated_at = Self::to_db_datetime(Utc::now());

        // Generate slug if becoming public
        let public_slug = if visibility == NoteVisibility::Public && existing.public_slug.is_none() {
            Some(Self::generate_slug())
        } else if visibility != NoteVisibility::Public {
            None
        } else {
            existing.public_slug
        };

        conn.execute(
            "UPDATE note_share_settings SET visibility = ?, public_slug = ?, allow_comments = ?, allow_copy = ?, updated_at = ? WHERE id = ?",
            params![visibility.as_str(), public_slug.as_deref(), allow_comments, allow_copy, updated_at, id],
        )
        .await?;

        Self::find_by_id(conn, id).await
    }

    pub async fn increment_view_count(conn: &Connection, id: &str) -> Result<bool> {
        let result = conn
            .execute(
                "UPDATE note_share_settings SET view_count = view_count + 1 WHERE id = ?",
                params![id],
            )
            .await?;
        Ok(result > 0)
    }

    pub async fn delete(conn: &Connection, id: &str) -> Result<bool> {
        let result = conn
            .execute("DELETE FROM note_share_settings WHERE id = ?", params![id])
            .await?;
        Ok(result > 0)
    }

    fn from_row(row: &libsql::Row) -> Result<Self> {
        let id = Self::get_string(row, 0)?;
        let note_id = Self::get_string(row, 1)?;
        let owner_id = Self::get_string(row, 2)?;
        let visibility_str = Self::get_string(row, 3)?;
        let visibility = NoteVisibility::from_str(&visibility_str)?;
        let public_slug = Self::get_opt_string(row, 4)?;
        let allow_comments = Self::get_bool(row, 5)?;
        let allow_copy = Self::get_bool(row, 6)?;
        let password_hash = Self::get_opt_string(row, 7)?;
        let view_count = Self::get_i64(row, 8)?;
        let created_at_str = Self::get_string(row, 9)?;
        let updated_at_str = Self::get_string(row, 10)?;

        Ok(Self {
            id,
            note_id,
            owner_id,
            visibility,
            public_slug,
            allow_comments,
            allow_copy,
            password_hash,
            view_count,
            created_at: Self::parse_db_datetime(&created_at_str, "created_at")?,
            updated_at: Self::parse_db_datetime(&updated_at_str, "updated_at")?,
        })
    }
}
