use anyhow::{Context, Result};
use libsql::{Builder, Connection, Database};
use log::info;

use super::schema::migrate;

#[derive(Clone)]
pub struct TursoConfig {
    pub db_url: String,
    pub db_token: String,
}

impl TursoConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            db_url: std::env::var("TURSO_DB_URL")
                .context("TURSO_DB_URL environment variable not set")?,
            db_token: std::env::var("TURSO_DB_TOKEN")
                .context("TURSO_DB_TOKEN environment variable not set")?,
        })
    }
}

pub struct TursoClient {
    db: Database,
    #[allow(dead_code)]
    config: TursoConfig,
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

        let conn = db
            .connect()
            .context("Failed to get database connection for migration")?;

        migrate(&conn).await.context("Schema migration failed")?;

        info!("Database connection established and schema migrated");

        Ok(Self { db, config })
    }

    pub fn get_connection(&self) -> Result<Connection> {
        self.db
            .connect()
            .context("Failed to get database connection")
    }

    pub fn get_user_db(&self, user_id: &str) -> Result<UserDb> {
        let conn = self.get_connection()?;
        Ok(UserDb::new(conn, user_id.to_string()))
    }

    pub async fn health_check(&self) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute("SELECT 1", libsql::params![]).await?;
        Ok(())
    }
}
