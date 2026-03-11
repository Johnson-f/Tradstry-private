use anyhow::Result;
use chrono::{DateTime, Utc};
use libsql::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarSyncCursor {
    pub id: String,
    pub connection_id: String,
    pub external_calendar_id: String,
    pub external_calendar_name: Option<String>,
    pub sync_token: Option<String>,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSyncCursorRequest {
    pub connection_id: String,
    pub external_calendar_id: String,
    pub external_calendar_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSyncCursorRequest {
    pub external_calendar_name: Option<String>,
    pub sync_token: Option<Option<String>>,
    pub last_sync_at: Option<DateTime<Utc>>,
}

impl CalendarSyncCursor {
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

    pub async fn create(conn: &Connection, req: CreateSyncCursorRequest) -> Result<Self> {
        let id = Uuid::new_v4().to_string();
        let now = Self::to_db_datetime(Utc::now());

        conn.execute(
            "INSERT INTO calendar_sync_cursors (id, connection_id, external_calendar_id, external_calendar_name, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)",
            params![
                id.clone(),
                req.connection_id,
                req.external_calendar_id,
                req.external_calendar_name.as_deref(),
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
                "SELECT id, connection_id, external_calendar_id, external_calendar_name, sync_token, last_sync_at, created_at, updated_at 
                 FROM calendar_sync_cursors WHERE id = ?"
            )
            .await?;
        let mut rows = stmt.query(params![id]).await?;

        if let Some(row) = rows.next().await? {
            Self::from_row(&row)
        } else {
            anyhow::bail!("Calendar sync cursor not found: {}", id)
        }
    }

    pub async fn find_by_connection(conn: &Connection, connection_id: &str) -> Result<Vec<Self>> {
        let stmt = conn
            .prepare(
                "SELECT id, connection_id, external_calendar_id, external_calendar_name, sync_token, last_sync_at, created_at, updated_at 
                 FROM calendar_sync_cursors WHERE connection_id = ? ORDER BY external_calendar_name"
            )
            .await?;
        let mut rows = stmt.query(params![connection_id]).await?;

        let mut cursors = Vec::new();
        while let Some(row) = rows.next().await? {
            cursors.push(Self::from_row(&row)?);
        }
        Ok(cursors)
    }

    pub async fn find_by_external_calendar(conn: &Connection, connection_id: &str, external_calendar_id: &str) -> Result<Option<Self>> {
        let stmt = conn
            .prepare(
                "SELECT id, connection_id, external_calendar_id, external_calendar_name, sync_token, last_sync_at, created_at, updated_at 
                 FROM calendar_sync_cursors WHERE connection_id = ? AND external_calendar_id = ?"
            )
            .await?;
        let mut rows = stmt.query(params![connection_id, external_calendar_id]).await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(Self::from_row(&row)?))
        } else {
            Ok(None)
        }
    }


    pub async fn upsert(conn: &Connection, connection_id: &str, external_calendar_id: &str, external_calendar_name: Option<&str>) -> Result<Self> {
        if let Some(existing) = Self::find_by_external_calendar(conn, connection_id, external_calendar_id).await? {
            if external_calendar_name.is_some() {
                Self::update(conn, &existing.id, UpdateSyncCursorRequest {
                    external_calendar_name: external_calendar_name.map(String::from),
                    sync_token: None,
                    last_sync_at: None,
                }).await
            } else {
                Ok(existing)
            }
        } else {
            Self::create(conn, CreateSyncCursorRequest {
                connection_id: connection_id.to_string(),
                external_calendar_id: external_calendar_id.to_string(),
                external_calendar_name: external_calendar_name.map(String::from),
            }).await
        }
    }

    pub async fn update(conn: &Connection, id: &str, req: UpdateSyncCursorRequest) -> Result<Self> {
        let existing = Self::find_by_id(conn, id).await?;
        let external_calendar_name = req.external_calendar_name.or(existing.external_calendar_name);
        let sync_token = match req.sync_token {
            Some(token) => token,
            None => existing.sync_token,
        };
        let last_sync_at = req.last_sync_at.or(existing.last_sync_at);
        let updated_at = Self::to_db_datetime(Utc::now());

        conn.execute(
            "UPDATE calendar_sync_cursors SET external_calendar_name = ?, sync_token = ?, last_sync_at = ?, updated_at = ? WHERE id = ?",
            params![
                external_calendar_name.as_deref(),
                sync_token.as_deref(),
                last_sync_at.map(Self::to_db_datetime).as_deref(),
                updated_at,
                id
            ],
        )
        .await?;

        Self::find_by_id(conn, id).await
    }

    pub async fn update_sync_token(conn: &Connection, id: &str, sync_token: Option<&str>) -> Result<Self> {
        let now = Self::to_db_datetime(Utc::now());

        conn.execute(
            "UPDATE calendar_sync_cursors SET sync_token = ?, last_sync_at = ?, updated_at = ? WHERE id = ?",
            params![sync_token, now.clone(), now, id],
        )
        .await?;

        Self::find_by_id(conn, id).await
    }

    pub async fn delete(conn: &Connection, id: &str) -> Result<bool> {
        let result = conn
            .execute("DELETE FROM calendar_sync_cursors WHERE id = ?", params![id])
            .await?;
        Ok(result > 0)
    }

    pub async fn delete_by_connection(conn: &Connection, connection_id: &str) -> Result<u64> {
        let result = conn
            .execute("DELETE FROM calendar_sync_cursors WHERE connection_id = ?", params![connection_id])
            .await?;
        Ok(result)
    }

    fn from_row(row: &libsql::Row) -> Result<Self> {
        let id = Self::get_string(row, 0)?;
        let connection_id = Self::get_string(row, 1)?;
        let external_calendar_id = Self::get_string(row, 2)?;
        let external_calendar_name = Self::get_opt_string(row, 3)?;
        let sync_token = Self::get_opt_string(row, 4)?;
        let last_sync_at_str = Self::get_opt_string(row, 5)?;
        let created_at_str = Self::get_string(row, 6)?;
        let updated_at_str = Self::get_string(row, 7)?;

        Ok(Self {
            id,
            connection_id,
            external_calendar_id,
            external_calendar_name,
            sync_token,
            last_sync_at: Self::parse_opt_db_datetime(last_sync_at_str.as_deref(), "last_sync_at")?,
            created_at: Self::parse_db_datetime(&created_at_str, "created_at")?,
            updated_at: Self::parse_db_datetime(&updated_at_str, "updated_at")?,
        })
    }
}
