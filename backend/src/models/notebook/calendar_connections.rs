use anyhow::Result;
use chrono::{DateTime, Utc};
use libsql::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarConnection {
    pub id: String,
    pub provider: CalendarProvider,
    pub access_token: String,
    pub refresh_token: String,
    pub token_expires_at: DateTime<Utc>,
    pub account_email: String,
    pub account_name: Option<String>,
    pub enabled: bool,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub sync_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CalendarProvider {
    Google,
    Microsoft,
}

impl CalendarProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            CalendarProvider::Google => "google",
            CalendarProvider::Microsoft => "microsoft",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "google" => Ok(CalendarProvider::Google),
            "microsoft" => Ok(CalendarProvider::Microsoft),
            _ => anyhow::bail!("Invalid calendar provider: {}", s),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateConnectionRequest {
    pub provider: CalendarProvider,
    pub access_token: String,
    pub refresh_token: String,
    pub token_expires_at: DateTime<Utc>,
    pub account_email: String,
    pub account_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateConnectionRequest {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub account_name: Option<String>,
    pub enabled: Option<bool>,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub sync_error: Option<Option<String>>,
}


impl CalendarConnection {
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

    fn parse_opt_db_datetime(datetime_str: Option<&str>, field_name: &str) -> Result<Option<DateTime<Utc>>> {
        match datetime_str {
            Some(s) if !s.is_empty() => Ok(Some(Self::parse_db_datetime(s, field_name)?)),
            _ => Ok(None),
        }
    }

    pub async fn create(conn: &Connection, req: CreateConnectionRequest) -> Result<Self> {
        let id = Uuid::new_v4().to_string();
        let now = Self::to_db_datetime(Utc::now());

        conn.execute(
            "INSERT INTO calendar_connections (id, provider, access_token, refresh_token, token_expires_at, account_email, account_name, enabled, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                id.clone(),
                req.provider.as_str(),
                req.access_token,
                req.refresh_token,
                Self::to_db_datetime(req.token_expires_at),
                req.account_email,
                req.account_name.as_deref(),
                true,
                now.clone(),
                now
            ],
        )
        .await?;

        Self::find_by_id(conn, &id).await
    }

    pub async fn find_by_id(conn: &Connection, id: &str) -> Result<Self> {
        let stmt = conn
            .prepare(
                "SELECT id, provider, access_token, refresh_token, token_expires_at, account_email, account_name, enabled, last_sync_at, sync_error, created_at, updated_at 
                 FROM calendar_connections WHERE id = ?"
            )
            .await?;
        let mut rows = stmt.query(params![id]).await?;

        if let Some(row) = rows.next().await? {
            Self::from_row(&row)
        } else {
            anyhow::bail!("Calendar connection not found: {}", id)
        }
    }

    pub async fn find_by_provider_email(conn: &Connection, provider: &CalendarProvider, email: &str) -> Result<Option<Self>> {
        let stmt = conn
            .prepare(
                "SELECT id, provider, access_token, refresh_token, token_expires_at, account_email, account_name, enabled, last_sync_at, sync_error, created_at, updated_at 
                 FROM calendar_connections WHERE provider = ? AND account_email = ?"
            )
            .await?;
        let mut rows = stmt.query(params![provider.as_str(), email]).await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(Self::from_row(&row)?))
        } else {
            Ok(None)
        }
    }

    pub async fn find_all(conn: &Connection) -> Result<Vec<Self>> {
        let stmt = conn
            .prepare(
                "SELECT id, provider, access_token, refresh_token, token_expires_at, account_email, account_name, enabled, last_sync_at, sync_error, created_at, updated_at 
                 FROM calendar_connections ORDER BY created_at DESC"
            )
            .await?;
        let mut rows = stmt.query(params![]).await?;

        let mut connections = Vec::new();
        while let Some(row) = rows.next().await? {
            connections.push(Self::from_row(&row)?);
        }
        Ok(connections)
    }

    pub async fn find_enabled(conn: &Connection) -> Result<Vec<Self>> {
        let stmt = conn
            .prepare(
                "SELECT id, provider, access_token, refresh_token, token_expires_at, account_email, account_name, enabled, last_sync_at, sync_error, created_at, updated_at 
                 FROM calendar_connections WHERE enabled = 1 ORDER BY created_at DESC"
            )
            .await?;
        let mut rows = stmt.query(params![]).await?;

        let mut connections = Vec::new();
        while let Some(row) = rows.next().await? {
            connections.push(Self::from_row(&row)?);
        }
        Ok(connections)
    }


    pub async fn update(conn: &Connection, id: &str, req: UpdateConnectionRequest) -> Result<Self> {
        let existing = Self::find_by_id(conn, id).await?;
        let access_token = req.access_token.unwrap_or(existing.access_token);
        let refresh_token = req.refresh_token.unwrap_or(existing.refresh_token);
        let token_expires_at = req.token_expires_at.unwrap_or(existing.token_expires_at);
        let account_name = req.account_name.or(existing.account_name);
        let enabled = req.enabled.unwrap_or(existing.enabled);
        let last_sync_at = req.last_sync_at.or(existing.last_sync_at);
        let sync_error = match req.sync_error {
            Some(err) => err,
            None => existing.sync_error,
        };
        let updated_at = Self::to_db_datetime(Utc::now());

        conn.execute(
            "UPDATE calendar_connections SET access_token = ?, refresh_token = ?, token_expires_at = ?, account_name = ?, enabled = ?, last_sync_at = ?, sync_error = ?, updated_at = ? WHERE id = ?",
            params![
                access_token,
                refresh_token,
                Self::to_db_datetime(token_expires_at),
                account_name.as_deref(),
                enabled,
                last_sync_at.map(Self::to_db_datetime).as_deref(),
                sync_error.as_deref(),
                updated_at,
                id
            ],
        )
        .await?;

        Self::find_by_id(conn, id).await
    }

    pub async fn update_tokens(conn: &Connection, id: &str, access_token: &str, refresh_token: &str, expires_at: DateTime<Utc>) -> Result<Self> {
        let updated_at = Self::to_db_datetime(Utc::now());

        conn.execute(
            "UPDATE calendar_connections SET access_token = ?, refresh_token = ?, token_expires_at = ?, updated_at = ? WHERE id = ?",
            params![access_token, refresh_token, Self::to_db_datetime(expires_at), updated_at, id],
        )
        .await?;

        Self::find_by_id(conn, id).await
    }

    pub async fn update_sync_status(conn: &Connection, id: &str, error: Option<&str>) -> Result<Self> {
        let now = Self::to_db_datetime(Utc::now());

        conn.execute(
            "UPDATE calendar_connections SET last_sync_at = ?, sync_error = ?, updated_at = ? WHERE id = ?",
            params![now.clone(), error, now, id],
        )
        .await?;

        Self::find_by_id(conn, id).await
    }

    pub async fn delete(conn: &Connection, id: &str) -> Result<bool> {
        let result = conn
            .execute("DELETE FROM calendar_connections WHERE id = ?", params![id])
            .await?;
        Ok(result > 0)
    }

    fn from_row(row: &libsql::Row) -> Result<Self> {
        let id = Self::get_string(row, 0)?;
        let provider_str = Self::get_string(row, 1)?;
        let access_token = Self::get_string(row, 2)?;
        let refresh_token = Self::get_string(row, 3)?;
        let token_expires_at_str = Self::get_string(row, 4)?;
        let account_email = Self::get_string(row, 5)?;
        let account_name = Self::get_opt_string(row, 6)?;
        let enabled = Self::get_bool(row, 7)?;
        let last_sync_at_str = Self::get_opt_string(row, 8)?;
        let sync_error = Self::get_opt_string(row, 9)?;
        let created_at_str = Self::get_string(row, 10)?;
        let updated_at_str = Self::get_string(row, 11)?;

        Ok(Self {
            id,
            provider: CalendarProvider::from_str(&provider_str)?,
            access_token,
            refresh_token,
            token_expires_at: Self::parse_db_datetime(&token_expires_at_str, "token_expires_at")?,
            account_email,
            account_name,
            enabled,
            last_sync_at: Self::parse_opt_db_datetime(last_sync_at_str.as_deref(), "last_sync_at")?,
            sync_error,
            created_at: Self::parse_db_datetime(&created_at_str, "created_at")?,
            updated_at: Self::parse_db_datetime(&updated_at_str, "updated_at")?,
        })
    }
}
