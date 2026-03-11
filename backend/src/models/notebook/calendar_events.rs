use anyhow::Result;
use chrono::{DateTime, Utc};
use libsql::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub all_day: bool,
    pub reminder_minutes: Option<i64>,
    pub recurrence_rule: Option<String>,
    pub source: EventSource,
    pub external_id: Option<String>,
    pub external_calendar_id: Option<String>,
    pub connection_id: Option<String>,
    pub color: Option<String>,
    pub location: Option<String>,
    pub status: EventStatus,
    pub is_reminder: bool,
    pub reminder_sent: bool,
    pub metadata: Option<Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub synced_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EventSource {
    Local,
    Google,
    Microsoft,
}

impl EventSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventSource::Local => "local",
            EventSource::Google => "google",
            EventSource::Microsoft => "microsoft",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "local" => Ok(EventSource::Local),
            "google" => Ok(EventSource::Google),
            "microsoft" => Ok(EventSource::Microsoft),
            _ => anyhow::bail!("Invalid event source: {}", s),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EventStatus {
    Confirmed,
    Tentative,
    Cancelled,
}

impl EventStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventStatus::Confirmed => "confirmed",
            EventStatus::Tentative => "tentative",
            EventStatus::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "confirmed" => Ok(EventStatus::Confirmed),
            "tentative" => Ok(EventStatus::Tentative),
            "cancelled" => Ok(EventStatus::Cancelled),
            _ => anyhow::bail!("Invalid event status: {}", s),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateEventRequest {
    pub title: String,
    pub description: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub all_day: Option<bool>,
    pub reminder_minutes: Option<i64>,
    pub recurrence_rule: Option<String>,
    pub color: Option<String>,
    pub location: Option<String>,
    pub is_reminder: Option<bool>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSyncedEventRequest {
    pub title: String,
    pub description: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub all_day: Option<bool>,
    pub recurrence_rule: Option<String>,
    pub source: EventSource,
    pub external_id: String,
    pub external_calendar_id: String,
    pub connection_id: String,
    pub color: Option<String>,
    pub location: Option<String>,
    pub status: Option<EventStatus>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEventRequest {
    pub title: Option<String>,
    pub description: Option<Option<String>>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<Option<DateTime<Utc>>>,
    pub all_day: Option<bool>,
    pub reminder_minutes: Option<Option<i64>>,
    pub recurrence_rule: Option<Option<String>>,
    pub color: Option<Option<String>>,
    pub location: Option<Option<String>>,
    pub status: Option<EventStatus>,
    pub is_reminder: Option<bool>,
    pub reminder_sent: Option<bool>,
    pub metadata: Option<Option<Value>>,
}

#[derive(Debug, Deserialize, Default)]
pub struct EventQuery {
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub source: Option<EventSource>,
    pub connection_id: Option<String>,
    pub is_reminder: Option<bool>,
    pub status: Option<EventStatus>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}


impl CalendarEvent {
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

    fn get_opt_i64(row: &libsql::Row, idx: i32) -> Result<Option<i64>> {
        match row.get_value(idx)? {
            libsql::Value::Integer(n) => Ok(Some(n)),
            libsql::Value::Null => Ok(None),
            libsql::Value::Text(s) => Ok(s.parse().ok()),
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

    /// Create a local event/reminder
    pub async fn create(conn: &Connection, req: CreateEventRequest) -> Result<Self> {
        let id = Uuid::new_v4().to_string();
        let now = Self::to_db_datetime(Utc::now());
        let all_day = req.all_day.unwrap_or(false);
        let is_reminder = req.is_reminder.unwrap_or(false);

        conn.execute(
            "INSERT INTO calendar_events (id, title, description, start_time, end_time, all_day, reminder_minutes, recurrence_rule, source, color, location, status, is_reminder, reminder_sent, metadata, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                id.clone(),
                req.title,
                req.description.as_deref(),
                Self::to_db_datetime(req.start_time),
                req.end_time.map(Self::to_db_datetime).as_deref(),
                all_day,
                req.reminder_minutes,
                req.recurrence_rule.as_deref(),
                EventSource::Local.as_str(),
                req.color.as_deref(),
                req.location.as_deref(),
                EventStatus::Confirmed.as_str(),
                is_reminder,
                false,
                req.metadata.as_ref().map(|m| m.to_string()).as_deref(),
                now.clone(),
                now
            ],
        )
        .await?;

        Self::find_by_id(conn, &id).await
    }

    /// Create or update a synced event from external calendar
    pub async fn upsert_synced(conn: &Connection, req: CreateSyncedEventRequest) -> Result<Self> {
        if let Some(existing) = Self::find_by_external_id(conn, &req.source, &req.external_id).await? {
            // Update existing
            let existing_id = existing.id.clone();
            let now = Self::to_db_datetime(Utc::now());
            conn.execute(
                "UPDATE calendar_events SET title = ?, description = ?, start_time = ?, end_time = ?, all_day = ?, recurrence_rule = ?, color = ?, location = ?, status = ?, metadata = ?, updated_at = ?, synced_at = ? WHERE id = ?",
                params![
                    req.title,
                    req.description.as_deref(),
                    Self::to_db_datetime(req.start_time),
                    req.end_time.map(Self::to_db_datetime).as_deref(),
                    req.all_day.unwrap_or(false),
                    req.recurrence_rule.as_deref(),
                    req.color.as_deref(),
                    req.location.as_deref(),
                    req.status.unwrap_or(EventStatus::Confirmed).as_str(),
                    req.metadata.as_ref().map(|m| m.to_string()).as_deref(),
                    now.clone(),
                    now,
                    existing_id.clone()
                ],
            )
            .await?;
            Self::find_by_id(conn, &existing_id).await
        } else {
            // Create new
            let id = Uuid::new_v4().to_string();
            let now = Self::to_db_datetime(Utc::now());

            conn.execute(
                "INSERT INTO calendar_events (id, title, description, start_time, end_time, all_day, recurrence_rule, source, external_id, external_calendar_id, connection_id, color, location, status, is_reminder, reminder_sent, metadata, created_at, updated_at, synced_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    id.clone(),
                    req.title,
                    req.description.as_deref(),
                    Self::to_db_datetime(req.start_time),
                    req.end_time.map(Self::to_db_datetime).as_deref(),
                    req.all_day.unwrap_or(false),
                    req.recurrence_rule.as_deref(),
                    req.source.as_str(),
                    req.external_id,
                    req.external_calendar_id,
                    req.connection_id,
                    req.color.as_deref(),
                    req.location.as_deref(),
                    req.status.unwrap_or(EventStatus::Confirmed).as_str(),
                    false,
                    false,
                    req.metadata.as_ref().map(|m| m.to_string()).as_deref(),
                    now.clone(),
                    now.clone(),
                    now
                ],
            )
            .await?;

            Self::find_by_id(conn, &id).await
        }
    }

    pub async fn find_by_id(conn: &Connection, id: &str) -> Result<Self> {
        let stmt = conn
            .prepare(
                "SELECT id, title, description, start_time, end_time, all_day, reminder_minutes, recurrence_rule, source, external_id, external_calendar_id, connection_id, color, location, status, is_reminder, reminder_sent, metadata, created_at, updated_at, synced_at 
                 FROM calendar_events WHERE id = ?"
            )
            .await?;
        let mut rows = stmt.query(params![id]).await?;

        if let Some(row) = rows.next().await? {
            Self::from_row(&row)
        } else {
            anyhow::bail!("Calendar event not found: {}", id)
        }
    }

    pub async fn find_by_external_id(conn: &Connection, source: &EventSource, external_id: &str) -> Result<Option<Self>> {
        let stmt = conn
            .prepare(
                "SELECT id, title, description, start_time, end_time, all_day, reminder_minutes, recurrence_rule, source, external_id, external_calendar_id, connection_id, color, location, status, is_reminder, reminder_sent, metadata, created_at, updated_at, synced_at 
                 FROM calendar_events WHERE source = ? AND external_id = ?"
            )
            .await?;
        let mut rows = stmt.query(params![source.as_str(), external_id]).await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(Self::from_row(&row)?))
        } else {
            Ok(None)
        }
    }


    pub async fn find_all(conn: &Connection, query: Option<EventQuery>) -> Result<Vec<Self>> {
        let query = query.unwrap_or_default();
        let mut sql = String::from(
            "SELECT id, title, description, start_time, end_time, all_day, reminder_minutes, recurrence_rule, source, external_id, external_calendar_id, connection_id, color, location, status, is_reminder, reminder_sent, metadata, created_at, updated_at, synced_at 
             FROM calendar_events WHERE 1=1"
        );
        let mut params_vec: Vec<String> = Vec::new();

        if let Some(start) = &query.start_time {
            sql.push_str(" AND start_time >= ?");
            params_vec.push(Self::to_db_datetime(*start));
        }
        if let Some(end) = &query.end_time {
            sql.push_str(" AND start_time <= ?");
            params_vec.push(Self::to_db_datetime(*end));
        }
        if let Some(source) = &query.source {
            sql.push_str(" AND source = ?");
            params_vec.push(source.as_str().to_string());
        }
        if let Some(conn_id) = &query.connection_id {
            sql.push_str(" AND connection_id = ?");
            params_vec.push(conn_id.clone());
        }
        if let Some(is_reminder) = query.is_reminder {
            sql.push_str(" AND is_reminder = ?");
            params_vec.push(if is_reminder { "1".to_string() } else { "0".to_string() });
        }
        if let Some(status) = &query.status {
            sql.push_str(" AND status = ?");
            params_vec.push(status.as_str().to_string());
        }

        sql.push_str(" ORDER BY start_time ASC");

        if let Some(limit) = query.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = query.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        let stmt = conn.prepare(&sql).await?;
        let params_refs: Vec<libsql::Value> = params_vec.iter().map(|s| libsql::Value::Text(s.clone())).collect();
        let mut rows = stmt.query(params_refs).await?;

        let mut events = Vec::new();
        while let Some(row) = rows.next().await? {
            events.push(Self::from_row(&row)?);
        }
        Ok(events)
    }

    pub async fn find_by_date_range(conn: &Connection, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<Self>> {
        Self::find_all(conn, Some(EventQuery {
            start_time: Some(start),
            end_time: Some(end),
            ..Default::default()
        })).await
    }

    pub async fn find_by_connection(conn: &Connection, connection_id: &str) -> Result<Vec<Self>> {
        Self::find_all(conn, Some(EventQuery {
            connection_id: Some(connection_id.to_string()),
            ..Default::default()
        })).await
    }

    pub async fn find_pending_reminders(conn: &Connection, before: DateTime<Utc>) -> Result<Vec<Self>> {
        let stmt = conn
            .prepare(
                "SELECT id, title, description, start_time, end_time, all_day, reminder_minutes, recurrence_rule, source, external_id, external_calendar_id, connection_id, color, location, status, is_reminder, reminder_sent, metadata, created_at, updated_at, synced_at 
                 FROM calendar_events 
                 WHERE is_reminder = 1 AND reminder_sent = 0 AND start_time <= ? 
                 ORDER BY start_time ASC"
            )
            .await?;
        let mut rows = stmt.query(params![Self::to_db_datetime(before)]).await?;

        let mut events = Vec::new();
        while let Some(row) = rows.next().await? {
            events.push(Self::from_row(&row)?);
        }
        Ok(events)
    }

    pub async fn update(conn: &Connection, id: &str, req: UpdateEventRequest) -> Result<Self> {
        let existing = Self::find_by_id(conn, id).await?;
        
        let title = req.title.unwrap_or(existing.title);
        let description = match req.description { Some(d) => d, None => existing.description };
        let start_time = req.start_time.unwrap_or(existing.start_time);
        let end_time = match req.end_time { Some(e) => e, None => existing.end_time };
        let all_day = req.all_day.unwrap_or(existing.all_day);
        let reminder_minutes = match req.reminder_minutes { Some(r) => r, None => existing.reminder_minutes };
        let recurrence_rule = match req.recurrence_rule { Some(r) => r, None => existing.recurrence_rule };
        let color = match req.color { Some(c) => c, None => existing.color };
        let location = match req.location { Some(l) => l, None => existing.location };
        let status = req.status.unwrap_or(existing.status);
        let is_reminder = req.is_reminder.unwrap_or(existing.is_reminder);
        let reminder_sent = req.reminder_sent.unwrap_or(existing.reminder_sent);
        let metadata = match req.metadata { Some(m) => m, None => existing.metadata };
        let updated_at = Self::to_db_datetime(Utc::now());

        conn.execute(
            "UPDATE calendar_events SET title = ?, description = ?, start_time = ?, end_time = ?, all_day = ?, reminder_minutes = ?, recurrence_rule = ?, color = ?, location = ?, status = ?, is_reminder = ?, reminder_sent = ?, metadata = ?, updated_at = ? WHERE id = ?",
            params![
                title,
                description.as_deref(),
                Self::to_db_datetime(start_time),
                end_time.map(Self::to_db_datetime).as_deref(),
                all_day,
                reminder_minutes,
                recurrence_rule.as_deref(),
                color.as_deref(),
                location.as_deref(),
                status.as_str(),
                is_reminder,
                reminder_sent,
                metadata.as_ref().map(|m| m.to_string()).as_deref(),
                updated_at,
                id
            ],
        )
        .await?;

        Self::find_by_id(conn, id).await
    }

    pub async fn mark_reminder_sent(conn: &Connection, id: &str) -> Result<Self> {
        let now = Self::to_db_datetime(Utc::now());
        conn.execute(
            "UPDATE calendar_events SET reminder_sent = 1, updated_at = ? WHERE id = ?",
            params![now, id],
        )
        .await?;
        Self::find_by_id(conn, id).await
    }

    pub async fn delete(conn: &Connection, id: &str) -> Result<bool> {
        let result = conn
            .execute("DELETE FROM calendar_events WHERE id = ?", params![id])
            .await?;
        Ok(result > 0)
    }

    pub async fn delete_by_connection(conn: &Connection, connection_id: &str) -> Result<u64> {
        let result = conn
            .execute("DELETE FROM calendar_events WHERE connection_id = ?", params![connection_id])
            .await?;
        Ok(result)
    }

    pub async fn delete_by_external_calendar(conn: &Connection, connection_id: &str, external_calendar_id: &str) -> Result<u64> {
        let result = conn
            .execute(
                "DELETE FROM calendar_events WHERE connection_id = ? AND external_calendar_id = ?",
                params![connection_id, external_calendar_id],
            )
            .await?;
        Ok(result)
    }

    fn from_row(row: &libsql::Row) -> Result<Self> {
        let id = Self::get_string(row, 0)?;
        let title = Self::get_string(row, 1)?;
        let description = Self::get_opt_string(row, 2)?;
        let start_time_str = Self::get_string(row, 3)?;
        let end_time_str = Self::get_opt_string(row, 4)?;
        let all_day = Self::get_bool(row, 5)?;
        let reminder_minutes = Self::get_opt_i64(row, 6)?;
        let recurrence_rule = Self::get_opt_string(row, 7)?;
        let source_str = Self::get_string(row, 8)?;
        let external_id = Self::get_opt_string(row, 9)?;
        let external_calendar_id = Self::get_opt_string(row, 10)?;
        let connection_id = Self::get_opt_string(row, 11)?;
        let color = Self::get_opt_string(row, 12)?;
        let location = Self::get_opt_string(row, 13)?;
        let status_str = Self::get_string(row, 14)?;
        let is_reminder = Self::get_bool(row, 15)?;
        let reminder_sent = Self::get_bool(row, 16)?;
        let metadata_str = Self::get_opt_string(row, 17)?;
        let created_at_str = Self::get_string(row, 18)?;
        let updated_at_str = Self::get_string(row, 19)?;
        let synced_at_str = Self::get_opt_string(row, 20)?;

        let metadata = match metadata_str {
            Some(s) if !s.is_empty() => serde_json::from_str(&s).ok(),
            _ => None,
        };

        Ok(Self {
            id,
            title,
            description,
            start_time: Self::parse_db_datetime(&start_time_str, "start_time")?,
            end_time: Self::parse_opt_db_datetime(end_time_str.as_deref(), "end_time")?,
            all_day,
            reminder_minutes,
            recurrence_rule,
            source: EventSource::from_str(&source_str)?,
            external_id,
            external_calendar_id,
            connection_id,
            color,
            location,
            status: EventStatus::from_str(&status_str)?,
            is_reminder,
            reminder_sent,
            metadata,
            created_at: Self::parse_db_datetime(&created_at_str, "created_at")?,
            updated_at: Self::parse_db_datetime(&updated_at_str, "updated_at")?,
            synced_at: Self::parse_opt_db_datetime(synced_at_str.as_deref(), "synced_at")?,
        })
    }
}
