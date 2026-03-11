use anyhow::Result;
use chrono::{DateTime, Utc};
use log::{debug, error, info};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::models::notebook::{CreateSyncedEventRequest, EventSource, EventStatus};

const GOOGLE_CALENDAR_API: &str = "https://www.googleapis.com/calendar/v3";

/// External calendar info from Google
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleCalendarInfo {
    pub id: String,
    pub summary: String,
    pub primary: bool,
}

/// Google Calendar event
#[derive(Debug, Clone, Deserialize)]
pub struct GoogleEvent {
    pub id: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start: GoogleEventTime,
    pub end: Option<GoogleEventTime>,
    pub status: Option<String>,
    pub recurrence: Option<Vec<String>>,
    #[serde(rename = "colorId")]
    pub color_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GoogleEventTime {
    #[serde(rename = "dateTime")]
    pub date_time: Option<String>,
    pub date: Option<String>,
    #[serde(rename = "timeZone")]
    pub time_zone: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CalendarListResponse {
    items: Vec<GoogleCalendarListItem>,
}

#[derive(Debug, Deserialize)]
struct GoogleCalendarListItem {
    id: String,
    summary: Option<String>,
    primary: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct EventsListResponse {
    items: Option<Vec<GoogleEvent>>,
    #[serde(rename = "nextSyncToken")]
    next_sync_token: Option<String>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

/// Google Calendar API client
pub struct GoogleCalendarClient {
    client: Client,
}

impl GoogleCalendarClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// List all calendars for the user
    pub async fn list_calendars(&self, access_token: &str) -> Result<Vec<GoogleCalendarInfo>> {
        info!("Fetching Google calendars");

        let response = self.client
            .get(format!("{}/users/me/calendarList", GOOGLE_CALENDAR_API))
            .bearer_auth(access_token)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            error!("Failed to list Google calendars: {}", error_text);
            anyhow::bail!("Failed to list Google calendars: {}", error_text);
        }

        let list_response: CalendarListResponse = response.json().await?;
        
        Ok(list_response.items.into_iter().map(|item| GoogleCalendarInfo {
            id: item.id,
            summary: item.summary.unwrap_or_else(|| "Unnamed Calendar".to_string()),
            primary: item.primary.unwrap_or(false),
        }).collect())
    }

    /// Fetch events from a calendar (full sync or incremental)
    pub async fn fetch_events(
        &self,
        access_token: &str,
        calendar_id: &str,
        sync_token: Option<&str>,
        time_min: Option<DateTime<Utc>>,
        time_max: Option<DateTime<Utc>>,
    ) -> Result<(Vec<GoogleEvent>, Option<String>)> {
        let mut all_events = Vec::new();
        let mut page_token: Option<String> = None;
        let mut final_sync_token: Option<String> = None;

        loop {
            let mut url = format!(
                "{}/calendars/{}/events?maxResults=250&singleEvents=true",
                GOOGLE_CALENDAR_API,
                urlencoding::encode(calendar_id)
            );

            if let Some(token) = &sync_token {
                // Incremental sync
                url.push_str(&format!("&syncToken={}", urlencoding::encode(token)));
            } else {
                // Full sync - use time bounds
                if let Some(min) = time_min {
                    url.push_str(&format!("&timeMin={}", urlencoding::encode(&min.to_rfc3339())));
                }
                if let Some(max) = time_max {
                    url.push_str(&format!("&timeMax={}", urlencoding::encode(&max.to_rfc3339())));
                }
            }

            if let Some(token) = &page_token {
                url.push_str(&format!("&pageToken={}", urlencoding::encode(token)));
            }

            debug!("Fetching Google events: {}", url);

            let response = self.client
                .get(&url)
                .bearer_auth(access_token)
                .send()
                .await?;

            if response.status().as_u16() == 410 {
                // Sync token expired, need full sync
                info!("Google sync token expired, full sync required");
                anyhow::bail!("SYNC_TOKEN_EXPIRED");
            }

            if !response.status().is_success() {
                let error_text = response.text().await.unwrap_or_default();
                error!("Failed to fetch Google events: {}", error_text);
                anyhow::bail!("Failed to fetch Google events: {}", error_text);
            }

            let events_response: EventsListResponse = response.json().await?;

            if let Some(items) = events_response.items {
                all_events.extend(items);
            }

            if let Some(token) = events_response.next_sync_token {
                final_sync_token = Some(token);
            }

            if let Some(token) = events_response.next_page_token {
                page_token = Some(token);
            } else {
                break;
            }
        }

        info!("Fetched {} Google events", all_events.len());
        Ok((all_events, final_sync_token))
    }

    /// Convert Google event to our internal format
    pub fn convert_event(
        event: &GoogleEvent,
        calendar_id: &str,
        connection_id: &str,
    ) -> Result<CreateSyncedEventRequest> {
        let title = event.summary.clone().unwrap_or_else(|| "(No title)".to_string());
        
        let (start_time, all_day) = Self::parse_event_time(&event.start)?;
        let end_time = event.end.as_ref()
            .and_then(|e| Self::parse_event_time(e).ok())
            .map(|(dt, _)| dt);

        let status = match event.status.as_deref() {
            Some("confirmed") => EventStatus::Confirmed,
            Some("tentative") => EventStatus::Tentative,
            Some("cancelled") => EventStatus::Cancelled,
            _ => EventStatus::Confirmed,
        };

        let recurrence_rule = event.recurrence.as_ref()
            .and_then(|rules| rules.first())
            .cloned();

        Ok(CreateSyncedEventRequest {
            title,
            description: event.description.clone(),
            start_time,
            end_time,
            all_day: Some(all_day),
            recurrence_rule,
            source: EventSource::Google,
            external_id: event.id.clone(),
            external_calendar_id: calendar_id.to_string(),
            connection_id: connection_id.to_string(),
            color: event.color_id.clone(),
            location: event.location.clone(),
            status: Some(status),
            metadata: None,
        })
    }

    fn parse_event_time(time: &GoogleEventTime) -> Result<(DateTime<Utc>, bool)> {
        if let Some(dt_str) = &time.date_time {
            // DateTime format
            let dt = DateTime::parse_from_rfc3339(dt_str)
                .map_err(|e| anyhow::anyhow!("Failed to parse datetime: {}", e))?
                .with_timezone(&Utc);
            Ok((dt, false))
        } else if let Some(date_str) = &time.date {
            // All-day event (date only)
            let naive_date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                .map_err(|e| anyhow::anyhow!("Failed to parse date: {}", e))?;
            let dt = naive_date.and_hms_opt(0, 0, 0)
                .ok_or_else(|| anyhow::anyhow!("Invalid time"))?
                .and_utc();
            Ok((dt, true))
        } else {
            anyhow::bail!("Event has no start time")
        }
    }
}

impl Default for GoogleCalendarClient {
    fn default() -> Self {
        Self::new()
    }
}
