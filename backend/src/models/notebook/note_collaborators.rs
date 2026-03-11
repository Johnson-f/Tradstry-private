use anyhow::Result;
use chrono::{DateTime, Utc};
use libsql::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Collaborator role for a note
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CollaboratorRole {
    Owner,
    Editor,
    Viewer,
}

impl CollaboratorRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            CollaboratorRole::Owner => "owner",
            CollaboratorRole::Editor => "editor",
            CollaboratorRole::Viewer => "viewer",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "owner" => Ok(CollaboratorRole::Owner),
            "editor" => Ok(CollaboratorRole::Editor),
            "viewer" => Ok(CollaboratorRole::Viewer),
            _ => anyhow::bail!("Invalid collaborator role: {}", s),
        }
    }

    pub fn can_edit(&self) -> bool {
        matches!(self, CollaboratorRole::Owner | CollaboratorRole::Editor)
    }
}

/// Note collaborator record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteCollaborator {
    pub id: String,
    pub note_id: String,
    pub user_id: String,
    pub user_email: String,
    pub user_name: Option<String>,
    pub role: CollaboratorRole,
    pub invited_by: String,
    pub joined_at: DateTime<Utc>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCollaboratorRequest {
    pub note_id: String,
    pub user_id: String,
    pub user_email: String,
    pub user_name: Option<String>,
    pub role: CollaboratorRole,
    pub invited_by: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCollaboratorRequest {
    pub role: Option<CollaboratorRole>,
    pub user_name: Option<String>,
}

impl NoteCollaborator {
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

    pub async fn create(conn: &Connection, req: CreateCollaboratorRequest) -> Result<Self> {
        let id = Uuid::new_v4().to_string();
        let now = Self::to_db_datetime(Utc::now());
        let role_str = req.role.as_str();

        conn.execute(
            "INSERT INTO note_collaborators (id, note_id, user_id, user_email, user_name, role, invited_by, joined_at, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                id.clone(),
                req.note_id,
                req.user_id,
                req.user_email,
                req.user_name.as_deref(),
                role_str,
                req.invited_by,
                now.clone(),
                now.clone(),
                now
            ],
        )
        .await?;

        Self::find_by_id(conn, &id).await
    }

    pub async fn find_by_id(conn: &Connection, id: &str) -> Result<Self> {
        let stmt = conn
            .prepare("SELECT id, note_id, user_id, user_email, user_name, role, invited_by, joined_at, last_seen_at, created_at, updated_at FROM note_collaborators WHERE id = ?")
            .await?;
        let mut rows = stmt.query(params![id]).await?;

        if let Some(row) = rows.next().await? {
            Self::from_row(&row)
        } else {
            anyhow::bail!("Collaborator not found: {}", id)
        }
    }

    pub async fn find_by_note(conn: &Connection, note_id: &str) -> Result<Vec<Self>> {
        let stmt = conn
            .prepare("SELECT id, note_id, user_id, user_email, user_name, role, invited_by, joined_at, last_seen_at, created_at, updated_at FROM note_collaborators WHERE note_id = ? ORDER BY role, joined_at")
            .await?;
        let mut rows = stmt.query(params![note_id]).await?;

        let mut collaborators = Vec::new();
        while let Some(row) = rows.next().await? {
            collaborators.push(Self::from_row(&row)?);
        }
        Ok(collaborators)
    }

    pub async fn find_by_user(conn: &Connection, user_id: &str) -> Result<Vec<Self>> {
        let stmt = conn
            .prepare("SELECT id, note_id, user_id, user_email, user_name, role, invited_by, joined_at, last_seen_at, created_at, updated_at FROM note_collaborators WHERE user_id = ? ORDER BY joined_at DESC")
            .await?;
        let mut rows = stmt.query(params![user_id]).await?;

        let mut collaborators = Vec::new();
        while let Some(row) = rows.next().await? {
            collaborators.push(Self::from_row(&row)?);
        }
        Ok(collaborators)
    }

    pub async fn find_by_note_and_user(conn: &Connection, note_id: &str, user_id: &str) -> Result<Option<Self>> {
        let stmt = conn
            .prepare("SELECT id, note_id, user_id, user_email, user_name, role, invited_by, joined_at, last_seen_at, created_at, updated_at FROM note_collaborators WHERE note_id = ? AND user_id = ?")
            .await?;
        let mut rows = stmt.query(params![note_id, user_id]).await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(Self::from_row(&row)?))
        } else {
            Ok(None)
        }
    }

    pub async fn update(conn: &Connection, id: &str, req: UpdateCollaboratorRequest) -> Result<Self> {
        let existing = Self::find_by_id(conn, id).await?;
        let role = req.role.unwrap_or(existing.role);
        let user_name = req.user_name.or(existing.user_name);
        let updated_at = Self::to_db_datetime(Utc::now());

        conn.execute(
            "UPDATE note_collaborators SET role = ?, user_name = ?, updated_at = ? WHERE id = ?",
            params![role.as_str(), user_name.as_deref(), updated_at, id],
        )
        .await?;

        Self::find_by_id(conn, id).await
    }

    pub async fn update_last_seen(conn: &Connection, id: &str) -> Result<bool> {
        let now = Self::to_db_datetime(Utc::now());
        let result = conn
            .execute(
                "UPDATE note_collaborators SET last_seen_at = ?, updated_at = ? WHERE id = ?",
                params![now.clone(), now, id],
            )
            .await?;
        Ok(result > 0)
    }

    pub async fn delete(conn: &Connection, id: &str) -> Result<bool> {
        let result = conn
            .execute("DELETE FROM note_collaborators WHERE id = ?", params![id])
            .await?;
        Ok(result > 0)
    }

    pub async fn delete_by_note(conn: &Connection, note_id: &str) -> Result<u64> {
        let result = conn
            .execute("DELETE FROM note_collaborators WHERE note_id = ?", params![note_id])
            .await?;
        Ok(result)
    }

    pub async fn remove_from_note(conn: &Connection, note_id: &str, user_id: &str) -> Result<bool> {
        let result = conn
            .execute(
                "DELETE FROM note_collaborators WHERE note_id = ? AND user_id = ?",
                params![note_id, user_id],
            )
            .await?;
        Ok(result > 0)
    }

    fn from_row(row: &libsql::Row) -> Result<Self> {
        let id = Self::get_string(row, 0)?;
        let note_id = Self::get_string(row, 1)?;
        let user_id = Self::get_string(row, 2)?;
        let user_email = Self::get_string(row, 3)?;
        let user_name = Self::get_opt_string(row, 4)?;
        let role_str = Self::get_string(row, 5)?;
        let role = CollaboratorRole::from_str(&role_str)?;
        let invited_by = Self::get_string(row, 6)?;
        let joined_at_str = Self::get_string(row, 7)?;
        let last_seen_at_str = Self::get_opt_string(row, 8)?;
        let created_at_str = Self::get_string(row, 9)?;
        let updated_at_str = Self::get_string(row, 10)?;

        Ok(Self {
            id,
            note_id,
            user_id,
            user_email,
            user_name,
            role,
            invited_by,
            joined_at: Self::parse_db_datetime(&joined_at_str, "joined_at")?,
            last_seen_at: last_seen_at_str.and_then(|s| Self::parse_db_datetime(&s, "last_seen_at").ok()),
            created_at: Self::parse_db_datetime(&created_at_str, "created_at")?,
            updated_at: Self::parse_db_datetime(&updated_at_str, "updated_at")?,
        })
    }
}
