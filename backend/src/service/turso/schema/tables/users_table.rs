use anyhow::{Context, Result};
use async_graphql::SimpleObject;
use libsql::Connection;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
pub struct User {
    pub id: String,
    pub clerk_uuid: String,
    pub full_name: String,
    pub email: String,
    pub created_at: String,
    pub updated_at: String,
}

fn row_to_user(row: &libsql::Row) -> Result<User> {
    Ok(User {
        id: row.get::<String>(0)?,
        clerk_uuid: row.get::<String>(1)?,
        full_name: row.get::<String>(2)?,
        email: row.get::<String>(3)?,
        created_at: row.get::<String>(4)?,
        updated_at: row.get::<String>(5)?,
    })
}

pub async fn find_by_clerk_uuid(conn: &Connection, clerk_uuid: &str) -> Result<Option<User>> {
    let mut rows = conn
        .query(
            "SELECT id, clerk_uuid, full_name, email, created_at, updated_at FROM users WHERE clerk_uuid = ?1",
            libsql::params![clerk_uuid],
        )
        .await
        .context("Failed to query user by clerk_uuid")?;

    match rows.next().await? {
        Some(row) => Ok(Some(row_to_user(&row)?)),
        None => Ok(None),
    }
}

pub async fn create_user(
    conn: &Connection,
    clerk_uuid: &str,
    full_name: &str,
    email: &str,
) -> Result<User> {
    let id = Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO users (id, clerk_uuid, full_name, email) VALUES (?1, ?2, ?3, ?4)",
        libsql::params![id.as_str(), clerk_uuid, full_name, email],
    )
    .await
    .context("Failed to insert user")?;

    find_by_clerk_uuid(conn, clerk_uuid)
        .await?
        .context("User not found after insert")
}

pub async fn find_or_create_user(
    conn: &Connection,
    clerk_uuid: &str,
    full_name: &str,
    email: &str,
) -> Result<(User, bool)> {
    if let Some(user) = find_by_clerk_uuid(conn, clerk_uuid).await? {
        return Ok((user, false));
    }

    match create_user(conn, clerk_uuid, full_name, email).await {
        Ok(user) => Ok((user, true)),
        Err(_) => {
            // Race condition: another request created the user first. Fetch it.
            find_by_clerk_uuid(conn, clerk_uuid)
                .await?
                .context("User not found after concurrent insert")
                .map(|user| (user, false))
        }
    }
}
