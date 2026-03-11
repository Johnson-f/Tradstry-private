use anyhow::Result;
use chrono::{Duration, Utc};
use libsql::Connection;
use log::{error, info, warn};

use crate::models::notebook::{
    CalendarConnection, CalendarEvent, CalendarProvider, CalendarSyncCursor,
    CreateConnectionRequest,
};

use super::google_calendar::GoogleCalendarClient;
use super::oauth::{OAuthConfig, OAuthService};

/// Calendar sync service - orchestrates OAuth and sync operations (Google only)
pub struct CalendarSyncService {
    oauth: OAuthService,
    google_client: GoogleCalendarClient,
}

impl CalendarSyncService {
    pub fn new(oauth_config: OAuthConfig) -> Self {
        Self {
            oauth: OAuthService::new(oauth_config),
            google_client: GoogleCalendarClient::new(),
        }
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self {
            oauth: OAuthService::from_env()?,
            google_client: GoogleCalendarClient::new(),
        })
    }

    // ==================== OAuth Methods ====================

    /// Get Google OAuth authorization URL
    pub fn get_auth_url(&self, state: &str) -> Result<String> {
        self.oauth.get_auth_url(state)
    }

    /// Handle OAuth callback - exchange code and create connection
    pub async fn handle_oauth_callback(
        &self,
        conn: &Connection,
        code: &str,
    ) -> Result<CalendarConnection> {
        // Exchange code for tokens
        let (tokens, email, name) = self.oauth.exchange_code(code).await?;

        // Check if connection already exists
        if let Some(existing) = CalendarConnection::find_by_provider_email(conn, &CalendarProvider::Google, &email).await? {
            // Update existing connection with new tokens
            CalendarConnection::update_tokens(
                conn,
                &existing.id,
                &tokens.access_token,
                &tokens.refresh_token,
                tokens.expires_at,
            ).await
        } else {
            // Create new connection
            CalendarConnection::create(conn, CreateConnectionRequest {
                provider: CalendarProvider::Google,
                access_token: tokens.access_token,
                refresh_token: tokens.refresh_token,
                token_expires_at: tokens.expires_at,
                account_email: email,
                account_name: name,
            }).await
        }
    }

    /// Ensure access token is valid, refresh if needed
    async fn ensure_valid_token(&self, conn: &Connection, connection: &CalendarConnection) -> Result<String> {
        if OAuthService::needs_refresh(connection) {
            info!("Refreshing token for connection {}", connection.id);
            let new_tokens = self.oauth.refresh_tokens(&connection.refresh_token).await?;
            
            CalendarConnection::update_tokens(
                conn,
                &connection.id,
                &new_tokens.access_token,
                &new_tokens.refresh_token,
                new_tokens.expires_at,
            ).await?;

            Ok(new_tokens.access_token)
        } else {
            Ok(connection.access_token.clone())
        }
    }

    // ==================== Sync Methods ====================

    /// Sync all enabled connections
    pub async fn sync_all(&self, conn: &Connection) -> Result<SyncResult> {
        let connections = CalendarConnection::find_enabled(conn).await?;
        let mut result = SyncResult::default();

        for connection in connections {
            match self.sync_connection(conn, &connection).await {
                Ok(count) => {
                    result.synced_connections += 1;
                    result.synced_events += count;
                }
                Err(e) => {
                    error!("Failed to sync connection {}: {}", connection.id, e);
                    result.failed_connections += 1;
                    CalendarConnection::update_sync_status(conn, &connection.id, Some(&e.to_string())).await.ok();
                }
            }
        }

        Ok(result)
    }

    /// Sync a specific connection
    pub async fn sync_connection(&self, conn: &Connection, connection: &CalendarConnection) -> Result<usize> {
        info!("Syncing calendar connection: {}", connection.id);

        // Ensure valid token
        let access_token = self.ensure_valid_token(conn, connection).await?;

        // Get or create sync cursors for each calendar
        let calendars = self.google_client.list_calendars(&access_token).await?;
        let mut total_synced = 0;

        for calendar in calendars {
            let cursor = CalendarSyncCursor::upsert(
                conn,
                &connection.id,
                &calendar.id,
                Some(&calendar.summary),
            ).await?;

            match self.sync_calendar(conn, connection, &cursor, &access_token).await {
                Ok(count) => {
                    total_synced += count;
                }
                Err(e) => {
                    warn!("Failed to sync calendar {}: {}", calendar.id, e);
                }
            }
        }

        // Update connection sync status
        CalendarConnection::update_sync_status(conn, &connection.id, None).await?;

        info!("Synced {} events for connection {}", total_synced, connection.id);
        Ok(total_synced)
    }

    /// Sync a specific calendar
    async fn sync_calendar(
        &self,
        conn: &Connection,
        connection: &CalendarConnection,
        cursor: &CalendarSyncCursor,
        access_token: &str,
    ) -> Result<usize> {
        let sync_token = cursor.sync_token.as_deref();
        
        // Default time range for full sync: past 30 days to 1 year ahead
        let time_min = Utc::now() - Duration::days(30);
        let time_max = Utc::now() + Duration::days(365);

        let (events, new_sync_token) = match self.google_client.fetch_events(
            access_token,
            &cursor.external_calendar_id,
            sync_token,
            Some(time_min),
            Some(time_max),
        ).await {
            Ok(result) => result,
            Err(e) if e.to_string().contains("SYNC_TOKEN_EXPIRED") => {
                // Clear sync token and retry full sync
                CalendarSyncCursor::update_sync_token(conn, &cursor.id, None).await?;
                self.google_client.fetch_events(
                    access_token,
                    &cursor.external_calendar_id,
                    None,
                    Some(time_min),
                    Some(time_max),
                ).await?
            }
            Err(e) => return Err(e),
        };

        // Process events
        let mut synced_count = 0;
        for event in events {
            if let Ok(req) = GoogleCalendarClient::convert_event(
                &event,
                &cursor.external_calendar_id,
                &connection.id,
            ) {
                if CalendarEvent::upsert_synced(conn, req).await.is_ok() {
                    synced_count += 1;
                }
            }
        }

        // Update sync token
        if let Some(token) = new_sync_token {
            CalendarSyncCursor::update_sync_token(conn, &cursor.id, Some(&token)).await?;
        }

        Ok(synced_count)
    }

    // ==================== Connection Management ====================

    /// Get all connections
    pub async fn get_connections(&self, conn: &Connection) -> Result<Vec<CalendarConnection>> {
        CalendarConnection::find_all(conn).await
    }

    /// Delete a connection and all its data
    pub async fn delete_connection(&self, conn: &Connection, connection_id: &str) -> Result<bool> {
        // Delete all events from this connection
        CalendarEvent::delete_by_connection(conn, connection_id).await?;
        
        // Delete sync cursors
        CalendarSyncCursor::delete_by_connection(conn, connection_id).await?;
        
        // Delete connection
        CalendarConnection::delete(conn, connection_id).await
    }

    /// Toggle connection enabled status
    pub async fn toggle_connection(&self, conn: &Connection, connection_id: &str, enabled: bool) -> Result<CalendarConnection> {
        use crate::models::notebook::UpdateConnectionRequest;
        
        CalendarConnection::update(conn, connection_id, UpdateConnectionRequest {
            access_token: None,
            refresh_token: None,
            token_expires_at: None,
            account_name: None,
            enabled: Some(enabled),
            last_sync_at: None,
            sync_error: None,
        }).await
    }
}

/// Result of sync operation
#[derive(Debug, Default)]
pub struct SyncResult {
    pub synced_connections: usize,
    pub failed_connections: usize,
    pub synced_events: usize,
}
