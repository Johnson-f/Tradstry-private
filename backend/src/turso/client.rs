use anyhow::{Context, Result};
use libsql::{Builder, Connection, Database};
use log::{error, info, warn};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::config::TursoConfig;
use super::schema::{
    ColumnInfo, SchemaVersion, TableSchema, create_table, ensure_indexes, ensure_triggers,
    get_current_schema_version, get_current_tables, get_expected_schema,
    initialize_schema_version_table, initialize_user_database_schema, update_schema_version,
    update_table_schema,
};

/// Turso client for managing user databases
pub struct TursoClient {
    config: TursoConfig,
    registry_db: Database,
    http_client: Client,
}

/// User database registry entry
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserDatabaseEntry {
    pub user_id: String,
    pub email: String,
    pub db_name: String,
    pub db_url: String,
    pub db_token: String,
    pub storage_used_bytes: Option<i64>,
    pub schema_version: Option<String>,
    pub initialized_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Public note registry entry
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PublicNoteEntry {
    pub slug: String,
    pub note_id: String,
    pub owner_id: String,
    pub title: String,
}

/// Invitation registry entry
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InvitationEntry {
    pub token: String,
    pub inviter_id: String,
    pub note_id: String,
    pub invitee_email: String,
}

/// Collaborator registry entry - for cross-database note access
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CollaboratorEntry {
    pub id: String,
    pub note_id: String,
    pub owner_id: String,
    pub collaborator_id: String,
    pub role: String,
}

/// Turso API response for database creation
#[derive(Debug, Serialize, Deserialize)]
pub struct TursoCreateDbResponse {
    pub database: TursoDatabaseInfo,
}

/// Turso database information
#[derive(Debug, Serialize, Deserialize)]
pub struct TursoDatabaseInfo {
    #[serde(rename = "Name")]
    #[allow(dead_code)]
    pub name: String,
    #[serde(rename = "Hostname")]
    pub hostname: String,
    #[serde(rename = "primaryRegion")]
    #[allow(dead_code)]
    pub primary_region: String,
}

/// Turso API response for token creation
#[derive(Debug, Deserialize)]
pub struct TursoTokenResponse {
    pub jwt: String,
}

// Schema types are now defined in super::schema

impl TursoClient {
    /// Create a new Turso client
    pub async fn new(config: TursoConfig) -> Result<Self> {
        // Connect to the central registry database
        let registry_db = Builder::new_remote(
            config.registry_db_url.clone(),
            config.registry_db_token.clone(),
        )
        .build()
        .await
        .context("Failed to connect to registry database")?;

        let http_client = Client::new();

        // Run registry database migrations
        let conn = registry_db
            .connect()
            .context("Failed to get registry database connection for migration")?;

        // Ensure user_databases table exists (registry)
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS user_databases (
                user_id TEXT PRIMARY KEY,
                email TEXT NOT NULL,
                db_name TEXT NOT NULL,
                db_url TEXT NOT NULL,
                db_token TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
            "#,
            libsql::params![],
        )
        .await
        .ok();

        // Add storage_used_bytes column if it doesn't exist
        conn.execute(
            "ALTER TABLE user_databases ADD COLUMN storage_used_bytes INTEGER DEFAULT 0",
            libsql::params![],
        )
        .await
        .ok(); // Ignore error if column already exists

        // Add is_active column if it doesn't exist
        conn.execute(
            "ALTER TABLE user_databases ADD COLUMN is_active INTEGER DEFAULT 1",
            libsql::params![],
        )
        .await
        .ok();

        // Add schema_version column to track user DB schema version in registry
        conn.execute(
            "ALTER TABLE user_databases ADD COLUMN schema_version TEXT",
            libsql::params![],
        )
        .await
        .ok(); // Ignore error if column already exists

        // Add initialized_at column to track when DB was first fully set up
        conn.execute(
            "ALTER TABLE user_databases ADD COLUMN initialized_at TIMESTAMP",
            libsql::params![],
        )
        .await
        .ok(); // Ignore error if column already exists

        // Ensure email index exists
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_user_databases_email ON user_databases(email)",
            libsql::params![],
        )
        .await
        .ok();

        // System-wide notifications (registry scope)
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS notifications (
                id TEXT PRIMARY KEY,
                notification_type TEXT NOT NULL,
                title TEXT NOT NULL,
                body TEXT,
                data TEXT,
                status TEXT NOT NULL DEFAULT 'template',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
            "#,
            libsql::params![],
        )
        .await
        .ok();
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_notifications_type ON notifications(notification_type)",
            libsql::params![],
        )
        .await
        .ok();
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_notifications_status ON notifications(status)",
            libsql::params![],
        )
        .await
        .ok();
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_notifications_created_at ON notifications(created_at)",
            libsql::params![],
        )
        .await
        .ok();
        conn.execute(
            r#"
            CREATE TRIGGER IF NOT EXISTS update_notifications_timestamp
            AFTER UPDATE ON notifications
            FOR EACH ROW
            BEGIN
                UPDATE notifications SET updated_at = datetime('now') WHERE id = NEW.id;
            END
            "#,
            libsql::params![],
        )
        .await
        .ok();

        // Dispatch log for system notifications
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS notification_dispatch_log (
                id TEXT PRIMARY KEY,
                notification_id TEXT,
                notification_type TEXT NOT NULL,
                slot_key TEXT NOT NULL UNIQUE,
                sent_count INTEGER DEFAULT 0,
                failed_count INTEGER DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
            "#,
            libsql::params![],
        )
        .await
        .ok();
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_notification_dispatch_type ON notification_dispatch_log(notification_type)",
            libsql::params![],
        )
        .await
        .ok();

        // Public notes registry - maps public slugs to user databases
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS public_notes_registry (
                slug TEXT PRIMARY KEY,
                note_id TEXT NOT NULL,
                owner_id TEXT NOT NULL,
                title TEXT NOT NULL DEFAULT 'Untitled',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
            "#,
            libsql::params![],
        )
        .await
        .ok();
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_public_notes_owner ON public_notes_registry(owner_id)",
            libsql::params![],
        )
        .await
        .ok();

        // Invitations registry - maps invitation tokens to inviter databases
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS invitations_registry (
                token TEXT PRIMARY KEY,
                inviter_id TEXT NOT NULL,
                note_id TEXT NOT NULL,
                invitee_email TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
            "#,
            libsql::params![],
        )
        .await
        .ok();
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_invitations_inviter ON invitations_registry(inviter_id)",
            libsql::params![],
        )
        .await
        .ok();
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_invitations_invitee ON invitations_registry(invitee_email)",
            libsql::params![],
        )
        .await
        .ok();

        // Collaborators registry - maps collaborator user_id to note owner for cross-database access
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS collaborators_registry (
                id TEXT PRIMARY KEY,
                note_id TEXT NOT NULL,
                owner_id TEXT NOT NULL,
                collaborator_id TEXT NOT NULL,
                role TEXT NOT NULL DEFAULT 'editor',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(note_id, collaborator_id)
            )
            "#,
            libsql::params![],
        )
        .await
        .ok();
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_collaborators_note ON collaborators_registry(note_id)",
            libsql::params![],
        )
        .await
        .ok();
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_collaborators_owner ON collaborators_registry(owner_id)",
            libsql::params![],
        )
        .await
        .ok();
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_collaborators_user ON collaborators_registry(collaborator_id)",
            libsql::params![],
        )
        .await
        .ok();

        info!("Registry database migration completed");

        Ok(Self {
            config,
            registry_db,
            http_client,
        })
    }

    /// Get a connection to the registry database
    pub async fn get_registry_connection(&self) -> Result<Connection> {
        self.registry_db
            .connect()
            .context("Failed to get registry database connection")
    }

    /// Create a new user database in Turso
    pub async fn create_user_database(
        &self,
        user_id: &str,
        email: &str,
    ) -> Result<UserDatabaseEntry> {
        info!("Creating database for user: {}", user_id);

        // Create database name (sanitize user_id for Turso requirements)
        // Turso requires: numbers, lowercase letters, and dashes only
        let sanitized_id = user_id
            .to_lowercase() // Convert to lowercase
            .replace("_", "-"); // Replace underscores with dashes
        let db_name = format!("user-{}", sanitized_id);

        info!("Generated database name: {}", db_name);

        // Create database via Turso API
        let db_info = self.create_database_via_api(&db_name).await?;

        // Create auth token for the database
        let token = self.create_database_token(&db_name).await?;

        // Construct the database URL
        let db_url = format!("libsql://{}", db_info.hostname);

        // Initialize the database schema
        initialize_user_database_schema(&db_url, &token).await?;

        // Get current schema version for new database
        let current_version = get_current_schema_version();
        let now = chrono::Utc::now().to_rfc3339();

        // Create user database entry with schema version
        let user_db_entry = UserDatabaseEntry {
            user_id: user_id.to_string(),
            email: email.to_string(),
            db_name: db_name.clone(),
            db_url: db_url.clone(),
            db_token: token,
            storage_used_bytes: Some(0), // Initialize to 0 for new databases
            schema_version: Some(current_version.version), // Set to current app schema version
            initialized_at: Some(now.clone()), // Mark as initialized now
            created_at: now.clone(),
            updated_at: now,
        };

        // Store in registry
        self.store_user_database_entry(&user_db_entry).await?;

        info!(
            "Successfully created database {} for user {}",
            db_name, user_id
        );
        Ok(user_db_entry)
    }

    /// List all user IDs that have a registry entry
    pub async fn list_all_user_ids(&self) -> Result<Vec<String>> {
        let conn = self.get_registry_connection().await?;
        let stmt = conn
            .prepare("SELECT user_id FROM user_databases")
            .await
            .context("Failed to prepare user list query")?;

        let mut rows = stmt
            .query(libsql::params![])
            .await
            .context("Failed to query user list")?;

        let mut ids = Vec::new();
        while let Some(row) = rows.next().await? {
            ids.push(row.get::<String>(0)?);
        }

        Ok(ids)
    }

    /// Create database via Turso API
    async fn create_database_via_api(&self, db_name: &str) -> Result<TursoDatabaseInfo> {
        let url = format!(
            "https://api.turso.tech/v1/organizations/{}/databases",
            self.config.turso_org
        );

        let mut payload = HashMap::new();
        payload.insert("name", db_name);
        payload.insert("group", "users-group"); // Use users-group for user databases

        let response = self
            .http_client
            .post(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.turso_api_token),
            )
            .json(&payload)
            .send()
            .await
            .context("Failed to send database creation request")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();

            // Check if the error is because database already exists
            if error_text.contains("already exists") {
                info!(
                    "Database {} already exists, fetching existing info",
                    db_name
                );
                return self.get_existing_database_info(db_name).await;
            }

            anyhow::bail!("Failed to create database: {}", error_text);
        }

        let create_response: TursoCreateDbResponse = response
            .json()
            .await
            .context("Failed to parse database creation response")?;

        Ok(create_response.database)
    }

    /// Get existing database info from Turso API
    async fn get_existing_database_info(&self, db_name: &str) -> Result<TursoDatabaseInfo> {
        let url = format!(
            "https://api.turso.tech/v1/organizations/{}/databases/{}",
            self.config.turso_org, db_name
        );

        let response = self
            .http_client
            .get(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.turso_api_token),
            )
            .send()
            .await
            .context("Failed to get existing database info")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get existing database info: {}", error_text);
        }

        #[derive(Deserialize)]
        struct GetDbResponse {
            database: TursoDatabaseInfo,
        }

        let db_response: GetDbResponse = response
            .json()
            .await
            .context("Failed to parse existing database response")?;

        Ok(db_response.database)
    }

    /// Create a database token for the given database
    pub async fn create_database_token(&self, db_name: &str) -> Result<String> {
        let url = format!(
            "https://api.turso.tech/v1/organizations/{}/databases/{}/auth/tokens",
            self.config.turso_org, db_name
        );

        let mut payload = HashMap::new();
        payload.insert("expiration", "never");
        payload.insert("authorization", "full-access");

        let response = self
            .http_client
            .post(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.turso_api_token),
            )
            .json(&payload)
            .send()
            .await
            .context("Failed to create database token")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create database token: {}", error_text);
        }

        let token_response: TursoTokenResponse = response
            .json()
            .await
            .context("Failed to parse token response")?;

        Ok(token_response.jwt)
    }

    /// Initialize user database with trading schema
    /// Creates the stocks table and other necessary tables for the trading journal
    #[allow(dead_code)]
    async fn initialize_user_database_schema(&self, db_url: &str, token: &str) -> Result<()> {
        initialize_user_database_schema(db_url, token).await
    }

    /// Store user database entry in registry
    async fn store_user_database_entry(&self, entry: &UserDatabaseEntry) -> Result<()> {
        let conn = self.get_registry_connection().await?;

        conn.execute(
            "INSERT OR REPLACE INTO user_databases
             (user_id, email, db_name, db_url, db_token, storage_used_bytes, schema_version, initialized_at, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            libsql::params![
                entry.user_id.as_str(),
                entry.email.as_str(),
                entry.db_name.as_str(),
                entry.db_url.as_str(),
                entry.db_token.as_str(),
                entry.storage_used_bytes.unwrap_or(0),
                entry.schema_version.as_deref(),
                entry.initialized_at.as_deref(),
                entry.created_at.as_str(),
                entry.updated_at.as_str(),
            ],
        ).await.context("Failed to store user database entry")?;

        Ok(())
    }

    /// Get user database entry by user ID
    pub async fn get_user_database(&self, user_id: &str) -> Result<Option<UserDatabaseEntry>> {
        let conn = self.get_registry_connection().await?;

        let mut rows = conn
            .prepare("SELECT user_id, email, db_name, db_url, db_token, storage_used_bytes, schema_version, initialized_at, created_at, updated_at FROM user_databases WHERE user_id = ?")
            .await
            .context("Failed to prepare query")?
            .query(libsql::params![user_id.to_string()])
            .await
            .context("Failed to execute query")?;

        if let Some(row) = rows.next().await? {
            let entry = UserDatabaseEntry {
                user_id: row.get(0)?,
                email: row.get(1)?,
                db_name: row.get(2)?,
                db_url: row.get(3)?,
                db_token: row.get(4)?,
                storage_used_bytes: row.get::<Option<i64>>(5)?,
                schema_version: row.get::<Option<String>>(6)?,
                initialized_at: row.get::<Option<String>>(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            };
            Ok(Some(entry))
        } else {
            Ok(None)
        }
    }

    /// Get all user database entries from registry (for background schema sync)
    pub async fn get_all_user_database_entries(&self) -> Result<Vec<UserDatabaseEntry>> {
        let conn = self.get_registry_connection().await?;

        let mut rows = conn
            .prepare("SELECT user_id, email, db_name, db_url, db_token, storage_used_bytes, schema_version, initialized_at, created_at, updated_at FROM user_databases")
            .await
            .context("Failed to prepare query")?
            .query(libsql::params![])
            .await
            .context("Failed to execute query")?;

        let mut entries = Vec::new();
        while let Some(row) = rows.next().await? {
            entries.push(UserDatabaseEntry {
                user_id: row.get(0)?,
                email: row.get(1)?,
                db_name: row.get(2)?,
                db_url: row.get(3)?,
                db_token: row.get(4)?,
                storage_used_bytes: row.get::<Option<i64>>(5)?,
                schema_version: row.get::<Option<String>>(6)?,
                initialized_at: row.get::<Option<String>>(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            });
        }

        Ok(entries)
    }

    /// Update schema version in registry for a user (called after successful sync)
    pub async fn update_registry_schema_version(&self, user_id: &str, version: &str) -> Result<()> {
        let conn = self.get_registry_connection().await?;

        conn.execute(
            "UPDATE user_databases SET schema_version = ?, updated_at = CURRENT_TIMESTAMP WHERE user_id = ?",
            libsql::params![version, user_id],
        ).await.context("Failed to update schema version in registry")?;

        Ok(())
    }

    /// Get user database connection
    pub async fn get_user_database_connection(&self, user_id: &str) -> Result<Option<Connection>> {
        if let Some(entry) = self.get_user_database(user_id).await? {
            let user_db = Builder::new_remote(entry.db_url, entry.db_token)
                .build()
                .await
                .context("Failed to connect to user database")?;

            let conn = user_db
                .connect()
                .context("Failed to get user database connection")?;
            Ok(Some(conn))
        } else {
            Ok(None)
        }
    }

    /// Delete a user database via Turso API
    pub async fn delete_user_database(&self, db_name: &str) -> Result<()> {
        info!("Deleting Turso database: {}", db_name);

        let url = format!(
            "https://api.turso.tech/v1/organizations/{}/databases/{}",
            self.config.turso_org, db_name
        );

        let response = self
            .http_client
            .delete(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.turso_api_token),
            )
            .send()
            .await
            .context("Failed to send database deletion request")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();

            // If database doesn't exist, consider it a success (idempotent)
            if error_text.contains("not found") || status == 404 {
                info!("Database {} does not exist (already deleted?)", db_name);
                return Ok(());
            }

            anyhow::bail!("Failed to delete database: {} - {}", status, error_text);
        }

        info!("Successfully deleted Turso database: {}", db_name);
        Ok(())
    }

    /// Remove user database entry from registry
    pub async fn remove_user_database_entry(&self, user_id: &str) -> Result<()> {
        info!("Removing user database entry from registry: {}", user_id);

        let conn = self.get_registry_connection().await?;

        conn.execute(
            "DELETE FROM user_databases WHERE user_id = ?",
            libsql::params![user_id],
        )
        .await
        .context("Failed to remove user database entry from registry")?;

        info!(
            "Successfully removed user database entry from registry: {}",
            user_id
        );
        Ok(())
    }

    /// Register a public note in the central registry
    pub async fn register_public_note(
        &self,
        slug: &str,
        note_id: &str,
        owner_id: &str,
        title: &str,
    ) -> Result<()> {
        let conn = self.get_registry_connection().await?;

        conn.execute(
            "INSERT OR REPLACE INTO public_notes_registry (slug, note_id, owner_id, title, updated_at)
             VALUES (?, ?, ?, ?, datetime('now'))",
            libsql::params![slug, note_id, owner_id, title],
        )
        .await
        .context("Failed to register public note")?;

        info!("Registered public note: slug={}, note_id={}", slug, note_id);
        Ok(())
    }

    /// Unregister a public note from the central registry
    pub async fn unregister_public_note(&self, slug: &str) -> Result<()> {
        let conn = self.get_registry_connection().await?;

        conn.execute(
            "DELETE FROM public_notes_registry WHERE slug = ?",
            libsql::params![slug],
        )
        .await
        .context("Failed to unregister public note")?;

        info!("Unregistered public note: slug={}", slug);
        Ok(())
    }

    /// Get public note registry entry by slug
    pub async fn get_public_note_entry(&self, slug: &str) -> Result<Option<PublicNoteEntry>> {
        let conn = self.get_registry_connection().await?;

        let mut rows = conn
            .prepare("SELECT slug, note_id, owner_id, title FROM public_notes_registry WHERE slug = ?")
            .await
            .context("Failed to prepare query")?
            .query(libsql::params![slug])
            .await
            .context("Failed to execute query")?;

        if let Some(row) = rows.next().await? {
            Ok(Some(PublicNoteEntry {
                slug: row.get(0)?,
                note_id: row.get(1)?,
                owner_id: row.get(2)?,
                title: row.get(3)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Register an invitation in the central registry
    pub async fn register_invitation(
        &self,
        token: &str,
        inviter_id: &str,
        note_id: &str,
        invitee_email: &str,
    ) -> Result<()> {
        let conn = self.get_registry_connection().await?;

        conn.execute(
            "INSERT OR REPLACE INTO invitations_registry (token, inviter_id, note_id, invitee_email)
             VALUES (?, ?, ?, ?)",
            libsql::params![token, inviter_id, note_id, invitee_email],
        )
        .await
        .context("Failed to register invitation")?;

        info!("Registered invitation: token={}, inviter={}", token, inviter_id);
        Ok(())
    }

    /// Unregister an invitation from the central registry
    pub async fn unregister_invitation(&self, token: &str) -> Result<()> {
        let conn = self.get_registry_connection().await?;

        conn.execute(
            "DELETE FROM invitations_registry WHERE token = ?",
            libsql::params![token],
        )
        .await
        .context("Failed to unregister invitation")?;

        Ok(())
    }

    /// Get invitation registry entry by token
    pub async fn get_invitation_entry(&self, token: &str) -> Result<Option<InvitationEntry>> {
        let conn = self.get_registry_connection().await?;

        let mut rows = conn
            .prepare("SELECT token, inviter_id, note_id, invitee_email FROM invitations_registry WHERE token = ?")
            .await
            .context("Failed to prepare query")?
            .query(libsql::params![token])
            .await
            .context("Failed to execute query")?;

        if let Some(row) = rows.next().await? {
            Ok(Some(InvitationEntry {
                token: row.get(0)?,
                inviter_id: row.get(1)?,
                note_id: row.get(2)?,
                invitee_email: row.get(3)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Register a collaborator in the central registry for cross-database access
    pub async fn register_collaborator(
        &self,
        id: &str,
        note_id: &str,
        owner_id: &str,
        collaborator_id: &str,
        role: &str,
    ) -> Result<()> {
        let conn = self.get_registry_connection().await?;

        conn.execute(
            "INSERT OR REPLACE INTO collaborators_registry (id, note_id, owner_id, collaborator_id, role)
             VALUES (?, ?, ?, ?, ?)",
            libsql::params![id, note_id, owner_id, collaborator_id, role],
        )
        .await
        .context("Failed to register collaborator")?;

        info!("Registered collaborator: user={} for note={} owned by {}", collaborator_id, note_id, owner_id);
        Ok(())
    }

    /// Unregister a collaborator from the central registry
    pub async fn unregister_collaborator(&self, note_id: &str, collaborator_id: &str) -> Result<()> {
        let conn = self.get_registry_connection().await?;

        conn.execute(
            "DELETE FROM collaborators_registry WHERE note_id = ? AND collaborator_id = ?",
            libsql::params![note_id, collaborator_id],
        )
        .await
        .context("Failed to unregister collaborator")?;

        info!("Unregistered collaborator: user={} from note={}", collaborator_id, note_id);
        Ok(())
    }

    /// Get collaborator entry for a specific note and user
    pub async fn get_collaborator_entry(&self, note_id: &str, user_id: &str) -> Result<Option<CollaboratorEntry>> {
        let conn = self.get_registry_connection().await?;

        let mut rows = conn
            .prepare("SELECT id, note_id, owner_id, collaborator_id, role FROM collaborators_registry WHERE note_id = ? AND collaborator_id = ?")
            .await
            .context("Failed to prepare query")?
            .query(libsql::params![note_id, user_id])
            .await
            .context("Failed to execute query")?;

        if let Some(row) = rows.next().await? {
            Ok(Some(CollaboratorEntry {
                id: row.get(0)?,
                note_id: row.get(1)?,
                owner_id: row.get(2)?,
                collaborator_id: row.get(3)?,
                role: row.get(4)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get all notes shared with a user (where they are a collaborator)
    pub async fn get_shared_notes_for_user(&self, user_id: &str) -> Result<Vec<CollaboratorEntry>> {
        let conn = self.get_registry_connection().await?;

        let mut rows = conn
            .prepare("SELECT id, note_id, owner_id, collaborator_id, role FROM collaborators_registry WHERE collaborator_id = ?")
            .await
            .context("Failed to prepare query")?
            .query(libsql::params![user_id])
            .await
            .context("Failed to execute query")?;

        let mut entries = Vec::new();
        while let Some(row) = rows.next().await? {
            entries.push(CollaboratorEntry {
                id: row.get(0)?,
                note_id: row.get(1)?,
                owner_id: row.get(2)?,
                collaborator_id: row.get(3)?,
                role: row.get(4)?,
            });
        }

        Ok(entries)
    }

    /// Health check for registry database
    pub async fn health_check(&self) -> Result<()> {
        let conn = self.get_registry_connection().await?;
        conn.execute("SELECT 1", libsql::params![]).await?;
        Ok(())
    }

    /// Get current schema version from user database
    pub async fn get_user_schema_version(&self, user_id: &str) -> Result<Option<SchemaVersion>> {
        if let Some(conn) = self.get_user_database_connection(user_id).await? {
            // Check if schema_version table exists
            let mut rows = conn
                .prepare(
                    "SELECT name FROM sqlite_master WHERE type='table' AND name='schema_version'",
                )
                .await
                .context("Failed to check schema_version table")?
                .query(libsql::params![])
                .await
                .context("Failed to execute schema_version check")?;

            if rows.next().await?.is_none() {
                return Ok(None); // No schema version table, means old schema
            }

            // Get the latest schema version
            let mut rows = conn
                .prepare("SELECT version, description, created_at FROM schema_version ORDER BY created_at DESC LIMIT 1")
                .await
                .context("Failed to prepare schema version query")?
                .query(libsql::params![])
                .await
                .context("Failed to execute schema version query")?;

            if let Some(row) = rows.next().await? {
                Ok(Some(SchemaVersion {
                    version: row.get(0)?,
                    description: row.get(1)?,
                    created_at: row.get(2)?,
                }))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Synchronize user database schema with current application schema
    pub async fn sync_user_database_schema(&self, user_id: &str) -> Result<()> {
        info!("Starting schema synchronization for user: {}", user_id);

        if let Some(conn) = self.get_user_database_connection(user_id).await? {
            let current_version = self.get_user_schema_version(user_id).await?;
            let expected_version = get_current_schema_version();
            let expected_schema = get_expected_schema();

            // If no version exists, this is a new database or very old one
            if current_version.is_none() {
                info!("No schema version found, initializing with current schema");
                initialize_schema_version_table(&conn).await?;
                self.apply_schema_migrations(&conn, &expected_schema)
                    .await?;
                update_schema_version(&conn, &expected_version).await?;
                return Ok(());
            }

            let current_version = current_version.unwrap();

            // Compare versions
            if current_version.version != expected_version.version {
                info!(
                    "Schema version mismatch: current={}, expected={}",
                    current_version.version, expected_version.version
                );

                // Apply schema migrations
                self.apply_schema_migrations(&conn, &expected_schema)
                    .await?;
                update_schema_version(&conn, &expected_version).await?;

                info!("Schema synchronized successfully for user: {}", user_id);
            } else {
                info!("Schema is up to date for user: {}", user_id);
            }
        } else {
            anyhow::bail!("Could not get database connection for user: {}", user_id);
        }

        Ok(())
    }

    /// Initialize the schema version table
    #[allow(dead_code)]
    async fn initialize_schema_version_table(&self, conn: &Connection) -> Result<()> {
        initialize_schema_version_table(conn).await
    }

    /// Update schema version in the database
    #[allow(dead_code)]
    async fn update_schema_version(
        &self,
        conn: &Connection,
        version: &SchemaVersion,
    ) -> Result<()> {
        update_schema_version(conn, version).await
    }

    /// Apply schema migrations to bring database up to current schema
    /// This function makes schema.rs the source of truth - it will drop any tables
    /// that exist in the database but are not in the expected schema
    #[allow(dead_code)]
    async fn apply_schema_migrations(
        &self,
        conn: &Connection,
        expected_schema: &[TableSchema],
    ) -> Result<()> {
        info!("Applying schema migrations");

        // Get list of expected table names (source of truth)
        let expected_table_names: std::collections::HashSet<String> =
            expected_schema.iter().map(|s| s.name.clone()).collect();

        // Also include system tables that should never be dropped
        let protected_tables: std::collections::HashSet<String> = [
            "schema_version".to_string(),
            "sqlite_sequence".to_string(), // SQLite internal table
        ]
        .iter()
        .cloned()
        .collect();

        // Get current tables in database
        let current_tables = get_current_tables(conn).await?;

        // Drop tables that exist in database but are not in expected schema
        // Temporarily disable foreign key constraints to allow dropping tables with dependencies
        conn.execute("PRAGMA foreign_keys = OFF", libsql::params![])
            .await?;

        for table_name in &current_tables {
            if !expected_table_names.contains(table_name) && !protected_tables.contains(table_name)
            {
                info!(
                    "Dropping table '{}' - not in expected schema (schema.rs is source of truth)",
                    table_name
                );

                // Drop all indexes for this table first
                let mut index_rows = conn
                    .prepare("SELECT name FROM sqlite_master WHERE type='index' AND tbl_name=? AND name NOT LIKE 'sqlite_autoindex_%'")
                    .await?
                    .query(libsql::params![table_name.clone()])
                    .await?;

                while let Some(index_row) = index_rows.next().await? {
                    let index_name: String = index_row.get(0)?;
                    info!("Dropping index '{}' for table '{}'", index_name, table_name);
                    if let Err(e) = conn
                        .execute(
                            &format!("DROP INDEX IF EXISTS {}", index_name),
                            libsql::params![],
                        )
                        .await
                    {
                        warn!("Failed to drop index '{}': {}", index_name, e);
                    }
                }

                // Drop all triggers for this table
                let mut trigger_rows = conn
                    .prepare("SELECT name FROM sqlite_master WHERE type='trigger' AND tbl_name=?")
                    .await?
                    .query(libsql::params![table_name.clone()])
                    .await?;

                while let Some(trigger_row) = trigger_rows.next().await? {
                    let trigger_name: String = trigger_row.get(0)?;
                    info!(
                        "Dropping trigger '{}' for table '{}'",
                        trigger_name, table_name
                    );
                    if let Err(e) = conn
                        .execute(
                            &format!("DROP TRIGGER IF EXISTS {}", trigger_name),
                            libsql::params![],
                        )
                        .await
                    {
                        warn!("Failed to drop trigger '{}': {}", trigger_name, e);
                    }
                }

                // Finally, drop the table
                if let Err(e) = conn
                    .execute(
                        &format!("DROP TABLE IF EXISTS {}", table_name),
                        libsql::params![],
                    )
                    .await
                {
                    error!("Failed to drop table '{}': {}", table_name, e);
                } else {
                    info!("Successfully dropped table '{}'", table_name);
                }
            }
        }

        // Re-enable foreign key constraints
        conn.execute("PRAGMA foreign_keys = ON", libsql::params![])
            .await?;

        // Create or update expected tables
        for table_schema in expected_schema {
            if !current_tables.contains(&table_schema.name) {
                info!("Creating missing table: {}", table_schema.name);
                create_table(conn, table_schema).await?;
            } else {
                info!("Checking table schema for: {}", table_schema.name);
                update_table_schema(conn, table_schema).await?;
            }
        }

        // Ensure indexes and triggers for all expected tables
        for table_schema in expected_schema {
            ensure_indexes(conn, table_schema).await?;
            ensure_triggers(conn, table_schema).await?;
        }

        info!("Schema migrations completed - database now matches schema.rs (source of truth)");
        Ok(())
    }

    /// Get list of current tables in the database
    #[allow(dead_code)]
    async fn get_current_tables(&self, conn: &Connection) -> Result<Vec<String>> {
        get_current_tables(conn).await
    }

    /// Create a table based on schema definition
    #[allow(dead_code)]
    async fn create_table(&self, conn: &Connection, table_schema: &TableSchema) -> Result<()> {
        create_table(conn, table_schema).await
    }

    /// Update table schema if needed
    #[allow(dead_code)]
    async fn update_table_schema(
        &self,
        conn: &Connection,
        table_schema: &TableSchema,
    ) -> Result<()> {
        update_table_schema(conn, table_schema).await
    }

    /// Get current columns for a table
    #[allow(dead_code)]
    async fn get_table_columns(
        &self,
        conn: &Connection,
        table_name: &str,
    ) -> Result<Vec<ColumnInfo>> {
        super::schema::get_table_columns(conn, table_name).await
    }

    /// Ensure indexes exist for a table
    #[allow(dead_code)]
    async fn ensure_indexes(&self, conn: &Connection, table_schema: &TableSchema) -> Result<()> {
        ensure_indexes(conn, table_schema).await
    }

    /// Ensure triggers exist for a table
    #[allow(dead_code)]
    async fn ensure_triggers(&self, conn: &Connection, table_schema: &TableSchema) -> Result<()> {
        ensure_triggers(conn, table_schema).await
    }
}

impl TursoClient {
    /// Public helper to be called by the auth/init flow: ensures the user's DB schema
    /// is up-to-date with the application schema definition. This will add missing
    /// columns like `stocks.version` or attempt to remove deprecated columns.
    #[allow(dead_code)]
    pub async fn ensure_user_schema_on_login(&self, user_id: &str) -> Result<()> {
        info!("Ensuring user schema is up to date for {}", user_id);
        self.sync_user_database_schema(user_id).await
    }
}
