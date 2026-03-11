use anyhow::{Context, Result};
use libsql::{Builder, Connection, Database};
use log::info;
use serde::{Deserialize, Serialize};

use super::config::TursoConfig;
use super::schema::migrate;

/// Turso client for the single shared database
pub struct TursoClient {
    db: Database,
    #[allow(dead_code)]
    config: TursoConfig,
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

/// Collaborator registry entry
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CollaboratorEntry {
    pub id: String,
    pub note_id: String,
    pub owner_id: String,
    pub collaborator_id: String,
    pub role: String,
}

/// Scoped database access for a specific user
pub struct UserDb {
    conn: Connection,
    user_id: String,
}

impl UserDb {
    pub fn new(conn: Connection, user_id: String) -> Self {
        Self { conn, user_id }
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn user_id(&self) -> &str {
        &self.user_id
    }
}

impl TursoClient {
    /// Create a new Turso client and run migrations
    pub async fn new(config: TursoConfig) -> Result<Self> {
        let db = Builder::new_remote(config.db_url.clone(), config.db_token.clone())
            .build()
            .await
            .context("Failed to connect to database")?;

        // Run schema migrations on startup
        let conn = db
            .connect()
            .context("Failed to get database connection for migration")?;

        migrate(&conn).await.context("Schema migration failed")?;

        info!("Database connection established and schema migrated");

        Ok(Self { db, config })
    }

    /// Get a connection to the shared database
    pub fn get_connection(&self) -> Result<Connection> {
        self.db
            .connect()
            .context("Failed to get database connection")
    }

    /// Get a scoped UserDb for a specific user
    pub fn get_user_db(&self, user_id: &str) -> Result<UserDb> {
        let conn = self.get_connection()?;
        Ok(UserDb::new(conn, user_id.to_string()))
    }

    /// Health check
    pub async fn health_check(&self) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute("SELECT 1", libsql::params![]).await?;
        Ok(())
    }

    /// List all user IDs
    pub async fn list_all_user_ids(&self) -> Result<Vec<String>> {
        let conn = self.get_connection()?;
        let mut rows = conn
            .query(
                "SELECT DISTINCT user_id FROM user_profile",
                libsql::params![],
            )
            .await?;

        let mut ids = Vec::new();
        while let Some(row) = rows.next().await? {
            ids.push(row.get::<String>(0)?);
        }
        Ok(ids)
    }

    // --- Public notes registry ---

    pub async fn register_public_note(
        &self,
        slug: &str,
        note_id: &str,
        owner_id: &str,
        title: &str,
    ) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "INSERT OR REPLACE INTO public_notes_registry (slug, note_id, owner_id, title) VALUES (?, ?, ?, ?)",
            libsql::params![slug, note_id, owner_id, title],
        ).await?;
        Ok(())
    }

    pub async fn unregister_public_note(&self, slug: &str) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "DELETE FROM public_notes_registry WHERE slug = ?",
            libsql::params![slug],
        )
        .await?;
        Ok(())
    }

    pub async fn get_public_note_entry(&self, slug: &str) -> Result<Option<PublicNoteEntry>> {
        let conn = self.get_connection()?;
        let mut rows = conn
            .query(
                "SELECT slug, note_id, owner_id, title FROM public_notes_registry WHERE slug = ?",
                libsql::params![slug],
            )
            .await?;

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

    // --- Invitations registry ---

    pub async fn register_invitation(
        &self,
        token: &str,
        inviter_id: &str,
        note_id: &str,
        invitee_email: &str,
    ) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "INSERT OR REPLACE INTO invitations_registry (token, inviter_id, note_id, invitee_email) VALUES (?, ?, ?, ?)",
            libsql::params![token, inviter_id, note_id, invitee_email],
        ).await?;
        Ok(())
    }

    pub async fn unregister_invitation(&self, token: &str) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "DELETE FROM invitations_registry WHERE token = ?",
            libsql::params![token],
        )
        .await?;
        Ok(())
    }

    pub async fn get_invitation_entry(&self, token: &str) -> Result<Option<InvitationEntry>> {
        let conn = self.get_connection()?;
        let mut rows = conn
            .query(
                "SELECT token, inviter_id, note_id, invitee_email FROM invitations_registry WHERE token = ?",
                libsql::params![token],
            )
            .await?;

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

    // --- Collaborators registry ---

    pub async fn register_collaborator(
        &self,
        id: &str,
        note_id: &str,
        owner_id: &str,
        collaborator_id: &str,
        role: &str,
    ) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "INSERT OR REPLACE INTO collaborators_registry (id, note_id, owner_id, collaborator_id, role) VALUES (?, ?, ?, ?, ?)",
            libsql::params![id, note_id, owner_id, collaborator_id, role],
        ).await?;
        Ok(())
    }

    pub async fn unregister_collaborator(
        &self,
        note_id: &str,
        collaborator_id: &str,
    ) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "DELETE FROM collaborators_registry WHERE note_id = ? AND collaborator_id = ?",
            libsql::params![note_id, collaborator_id],
        )
        .await?;
        Ok(())
    }

    pub async fn get_collaborator_entry(
        &self,
        note_id: &str,
        user_id: &str,
    ) -> Result<Option<CollaboratorEntry>> {
        let conn = self.get_connection()?;
        let mut rows = conn
            .query(
                "SELECT id, note_id, owner_id, collaborator_id, role FROM collaborators_registry WHERE note_id = ? AND collaborator_id = ?",
                libsql::params![note_id, user_id],
            )
            .await?;

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

    pub async fn get_shared_notes_for_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<CollaboratorEntry>> {
        let conn = self.get_connection()?;
        let mut rows = conn
            .query(
                "SELECT id, note_id, owner_id, collaborator_id, role FROM collaborators_registry WHERE collaborator_id = ?",
                libsql::params![user_id],
            )
            .await?;

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
}
