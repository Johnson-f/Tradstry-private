use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use libsql::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::note_collaborators::CollaboratorRole;

/// Invitation status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InvitationStatus {
    Pending,
    Accepted,
    Declined,
    Expired,
    Revoked,
}

impl InvitationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            InvitationStatus::Pending => "pending",
            InvitationStatus::Accepted => "accepted",
            InvitationStatus::Declined => "declined",
            InvitationStatus::Expired => "expired",
            InvitationStatus::Revoked => "revoked",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(InvitationStatus::Pending),
            "accepted" => Ok(InvitationStatus::Accepted),
            "declined" => Ok(InvitationStatus::Declined),
            "expired" => Ok(InvitationStatus::Expired),
            "revoked" => Ok(InvitationStatus::Revoked),
            _ => anyhow::bail!("Invalid invitation status: {}", s),
        }
    }
}

/// Note invitation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteInvitation {
    pub id: String,
    pub note_id: String,
    pub inviter_id: String,
    pub inviter_email: String,
    pub invitee_email: String,
    pub invitee_user_id: Option<String>,
    pub role: CollaboratorRole,
    pub token: String,
    pub status: InvitationStatus,
    pub message: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateInvitationRequest {
    pub note_id: String,
    pub inviter_id: String,
    pub inviter_email: String,
    pub invitee_email: String,
    pub role: CollaboratorRole,
    pub message: Option<String>,
    pub expires_in_days: Option<i64>,
}

impl NoteInvitation {
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

    fn generate_token() -> String {
        format!("{}-{}", Uuid::new_v4(), Uuid::new_v4())
            .replace("-", "")
            .chars()
            .take(64)
            .collect()
    }

    pub async fn create(conn: &Connection, req: CreateInvitationRequest) -> Result<Self> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let now_str = Self::to_db_datetime(now);
        let token = Self::generate_token();
        let expires_in_days = req.expires_in_days.unwrap_or(7);
        let expires_at = now + Duration::days(expires_in_days);
        let expires_at_str = Self::to_db_datetime(expires_at);

        conn.execute(
            "INSERT INTO note_invitations (id, note_id, inviter_id, inviter_email, invitee_email, role, token, status, message, expires_at, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                id.clone(),
                req.note_id,
                req.inviter_id,
                req.inviter_email,
                req.invitee_email,
                req.role.as_str(),
                token,
                InvitationStatus::Pending.as_str(),
                req.message.as_deref(),
                expires_at_str,
                now_str.clone(),
                now_str
            ],
        )
        .await?;

        Self::find_by_id(conn, &id).await
    }

    pub async fn find_by_id(conn: &Connection, id: &str) -> Result<Self> {
        let stmt = conn
            .prepare("SELECT id, note_id, inviter_id, inviter_email, invitee_email, invitee_user_id, role, token, status, message, expires_at, accepted_at, created_at, updated_at FROM note_invitations WHERE id = ?")
            .await?;
        let mut rows = stmt.query(params![id]).await?;

        if let Some(row) = rows.next().await? {
            Self::from_row(&row)
        } else {
            anyhow::bail!("Invitation not found: {}", id)
        }
    }

    pub async fn find_by_token(conn: &Connection, token: &str) -> Result<Option<Self>> {
        let stmt = conn
            .prepare("SELECT id, note_id, inviter_id, inviter_email, invitee_email, invitee_user_id, role, token, status, message, expires_at, accepted_at, created_at, updated_at FROM note_invitations WHERE token = ?")
            .await?;
        let mut rows = stmt.query(params![token]).await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(Self::from_row(&row)?))
        } else {
            Ok(None)
        }
    }

    pub async fn find_by_note(conn: &Connection, note_id: &str) -> Result<Vec<Self>> {
        let stmt = conn
            .prepare("SELECT id, note_id, inviter_id, inviter_email, invitee_email, invitee_user_id, role, token, status, message, expires_at, accepted_at, created_at, updated_at FROM note_invitations WHERE note_id = ? ORDER BY created_at DESC")
            .await?;
        let mut rows = stmt.query(params![note_id]).await?;

        let mut invitations = Vec::new();
        while let Some(row) = rows.next().await? {
            invitations.push(Self::from_row(&row)?);
        }
        Ok(invitations)
    }

    pub async fn find_pending_by_email(conn: &Connection, email: &str) -> Result<Vec<Self>> {
        let stmt = conn
            .prepare("SELECT id, note_id, inviter_id, inviter_email, invitee_email, invitee_user_id, role, token, status, message, expires_at, accepted_at, created_at, updated_at FROM note_invitations WHERE invitee_email = ? AND status = 'pending' ORDER BY created_at DESC")
            .await?;
        let mut rows = stmt.query(params![email]).await?;

        let mut invitations = Vec::new();
        while let Some(row) = rows.next().await? {
            invitations.push(Self::from_row(&row)?);
        }
        Ok(invitations)
    }

    pub async fn find_pending_by_note_and_email(conn: &Connection, note_id: &str, email: &str) -> Result<Option<Self>> {
        let stmt = conn
            .prepare("SELECT id, note_id, inviter_id, inviter_email, invitee_email, invitee_user_id, role, token, status, message, expires_at, accepted_at, created_at, updated_at FROM note_invitations WHERE note_id = ? AND invitee_email = ? AND status = 'pending'")
            .await?;
        let mut rows = stmt.query(params![note_id, email]).await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(Self::from_row(&row)?))
        } else {
            Ok(None)
        }
    }

    pub async fn accept(conn: &Connection, id: &str, user_id: &str) -> Result<Self> {
        let now = Self::to_db_datetime(Utc::now());
        conn.execute(
            "UPDATE note_invitations SET status = ?, invitee_user_id = ?, accepted_at = ?, updated_at = ? WHERE id = ?",
            params![InvitationStatus::Accepted.as_str(), user_id, now.clone(), now, id],
        )
        .await?;
        Self::find_by_id(conn, id).await
    }

    pub async fn decline(conn: &Connection, id: &str) -> Result<Self> {
        let now = Self::to_db_datetime(Utc::now());
        conn.execute(
            "UPDATE note_invitations SET status = ?, updated_at = ? WHERE id = ?",
            params![InvitationStatus::Declined.as_str(), now, id],
        )
        .await?;
        Self::find_by_id(conn, id).await
    }

    pub async fn revoke(conn: &Connection, id: &str) -> Result<Self> {
        let now = Self::to_db_datetime(Utc::now());
        conn.execute(
            "UPDATE note_invitations SET status = ?, updated_at = ? WHERE id = ?",
            params![InvitationStatus::Revoked.as_str(), now, id],
        )
        .await?;
        Self::find_by_id(conn, id).await
    }

    pub async fn delete(conn: &Connection, id: &str) -> Result<bool> {
        let result = conn
            .execute("DELETE FROM note_invitations WHERE id = ?", params![id])
            .await?;
        Ok(result > 0)
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub fn is_valid(&self) -> bool {
        self.status == InvitationStatus::Pending && !self.is_expired()
    }

    fn from_row(row: &libsql::Row) -> Result<Self> {
        let id = Self::get_string(row, 0)?;
        let note_id = Self::get_string(row, 1)?;
        let inviter_id = Self::get_string(row, 2)?;
        let inviter_email = Self::get_string(row, 3)?;
        let invitee_email = Self::get_string(row, 4)?;
        let invitee_user_id = Self::get_opt_string(row, 5)?;
        let role_str = Self::get_string(row, 6)?;
        let role = CollaboratorRole::from_str(&role_str)?;
        let token = Self::get_string(row, 7)?;
        let status_str = Self::get_string(row, 8)?;
        let status = InvitationStatus::from_str(&status_str)?;
        let message = Self::get_opt_string(row, 9)?;
        let expires_at_str = Self::get_string(row, 10)?;
        let accepted_at_str = Self::get_opt_string(row, 11)?;
        let created_at_str = Self::get_string(row, 12)?;
        let updated_at_str = Self::get_string(row, 13)?;

        Ok(Self {
            id,
            note_id,
            inviter_id,
            inviter_email,
            invitee_email,
            invitee_user_id,
            role,
            token,
            status,
            message,
            expires_at: Self::parse_db_datetime(&expires_at_str, "expires_at")?,
            accepted_at: accepted_at_str.and_then(|s| Self::parse_db_datetime(&s, "accepted_at").ok()),
            created_at: Self::parse_db_datetime(&created_at_str, "created_at")?,
            updated_at: Self::parse_db_datetime(&updated_at_str, "updated_at")?,
        })
    }
}
