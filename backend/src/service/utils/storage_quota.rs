use actix_web::{HttpResponse, ResponseError};
use anyhow::{Context, Result};
use libsql::Connection;
use log::{error, info, warn};
use thiserror::Error;

use crate::turso::client::TursoClient;

/// Storage quota limit per user (18 MB)
pub const STORAGE_QUOTA_LIMIT_BYTES: u64 = 18 * 1024 * 1024; // 18,874,368 bytes

/// Storage usage information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StorageUsage {
    pub used_bytes: u64,
    pub limit_bytes: u64,
    pub used_mb: f64,
    pub limit_mb: f64,
    pub remaining_bytes: u64,
    pub remaining_mb: f64,
    pub percentage_used: f64,
}

/// Error type for storage quota operations
#[derive(Debug, Error)]
pub enum StorageQuotaError {
    #[error("Storage quota exceeded: {used_bytes} bytes used of {limit_bytes} bytes limit")]
    QuotaExceeded { used_bytes: u64, limit_bytes: u64 },
    #[error("Database error: {0}")]
    DatabaseError(#[from] anyhow::Error),
}

impl ResponseError for StorageQuotaError {
    fn error_response(&self) -> HttpResponse {
        match self {
            StorageQuotaError::QuotaExceeded {
                used_bytes,
                limit_bytes,
            } => {
                let used_mb = *used_bytes as f64 / (1024.0 * 1024.0);
                let limit_mb = *limit_bytes as f64 / (1024.0 * 1024.0);

                HttpResponse::InsufficientStorage().json(serde_json::json!({
                    "success": false,
                    "error": format!(
                        "Storage quota exceeded. You have used {:.2} MB of your {:.0} MB limit. Please delete some data to free up space.",
                        used_mb, limit_mb
                    ),
                    "used_bytes": used_bytes,
                    "limit_bytes": limit_bytes,
                    "used_mb": used_mb,
                    "limit_mb": limit_mb,
                    "remaining_bytes": 0i64,
                }))
            }
            StorageQuotaError::DatabaseError(e) => {
                error!("Database error in storage quota check: {}", e);
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "success": false,
                    "error": "Failed to check storage quota"
                }))
            }
        }
    }
}

/// Storage quota service for managing user storage limits
#[derive(Clone)]
pub struct StorageQuotaService {
    turso_client: std::sync::Arc<TursoClient>,
}

impl StorageQuotaService {
    /// Create a new storage quota service
    pub fn new(turso_client: std::sync::Arc<TursoClient>) -> Self {
        Self { turso_client }
    }

    /// Calculate the actual size of a user's database using SQLite PRAGMA commands
    pub async fn calculate_database_size(&self, conn: &Connection, user_id: &str) -> Result<u64> {
        // Get page count
        let page_count_stmt = conn
            .prepare("PRAGMA page_count")
            .await
            .context("Failed to prepare PRAGMA page_count query")?;

        let mut page_count_rows = page_count_stmt
            .query(libsql::params![])
            .await
            .context("Failed to execute PRAGMA page_count")?;

        let page_count: i64 = if let Some(row) = page_count_rows.next().await? {
            row.get(0).context("Failed to get page_count value")?
        } else {
            return Err(anyhow::anyhow!("No result from PRAGMA page_count"));
        };

        // Get page size
        let page_size_stmt = conn
            .prepare("PRAGMA page_size")
            .await
            .context("Failed to prepare PRAGMA page_size query")?;

        let mut page_size_rows = page_size_stmt
            .query(libsql::params![])
            .await
            .context("Failed to execute PRAGMA page_size")?;

        let page_size: i64 = if let Some(row) = page_size_rows.next().await? {
            row.get(0).context("Failed to get page_size value")?
        } else {
            return Err(anyhow::anyhow!("No result from PRAGMA page_size"));
        };

        let total_size = (page_count * page_size) as u64;

        info!(
            "Calculated database size for user {}: {} bytes ({} pages × {} bytes)",
            user_id, total_size, page_count, page_size
        );

        Ok(total_size)
    }

    /// Get the cached storage usage from the registry database
    async fn get_cached_storage_usage(&self, user_id: &str) -> Result<Option<u64>> {
        let registry_conn = self.turso_client.get_registry_connection().await?;

        let stmt = registry_conn
            .prepare("SELECT storage_used_bytes FROM user_databases WHERE user_id = ?")
            .await
            .context("Failed to prepare storage query")?;

        let mut rows = stmt
            .query([user_id])
            .await
            .context("Failed to query storage usage")?;

        if let Some(row) = rows.next().await? {
            let storage_used: Option<i64> = row.get(0).ok();
            Ok(storage_used.map(|s| s as u64))
        } else {
            Ok(None)
        }
    }

    /// Update the cached storage usage in the registry database
    pub async fn update_storage_usage(&self, user_id: &str, size_bytes: u64) -> Result<()> {
        let registry_conn = self.turso_client.get_registry_connection().await?;

        registry_conn
            .execute(
                "UPDATE user_databases SET storage_used_bytes = ?, updated_at = CURRENT_TIMESTAMP WHERE user_id = ?",
                libsql::params![size_bytes as i64, user_id],
            )
            .await
            .context("Failed to update storage usage in registry")?;

        info!(
            "Updated cached storage usage for user {}: {} bytes",
            user_id, size_bytes
        );

        Ok(())
    }

    /// Check if user has storage quota available before allowing new data
    /// Returns error if quota is exceeded, Ok(()) if within limit
    pub async fn check_storage_quota(
        &self,
        user_id: &str,
        user_conn: &Connection,
    ) -> Result<(), StorageQuotaError> {
        // First check cached value for quick validation
        let cached_size = self
            .get_cached_storage_usage(user_id)
            .await
            .map_err(StorageQuotaError::DatabaseError)?
            .unwrap_or(0);

        // If cached size is near limit, calculate actual size to verify
        let threshold = STORAGE_QUOTA_LIMIT_BYTES - (1024 * 1024); // 1 MB before limit

        let current_size = if cached_size >= threshold {
            // Recalculate to get accurate size when near limit
            info!(
                "Near quota limit, recalculating actual database size for user {}",
                user_id
            );
            self.calculate_database_size(user_conn, user_id)
                .await
                .map_err(StorageQuotaError::DatabaseError)?
        } else {
            // Use cached value for performance
            cached_size
        };

        // Update cache if we recalculated
        if cached_size < threshold
            && current_size >= threshold
            && let Err(e) = self.update_storage_usage(user_id, current_size).await
        {
            warn!("Failed to update storage cache for user {}: {}", user_id, e);
        }

        // Check against quota limit
        if current_size >= STORAGE_QUOTA_LIMIT_BYTES {
            return Err(StorageQuotaError::QuotaExceeded {
                used_bytes: current_size,
                limit_bytes: STORAGE_QUOTA_LIMIT_BYTES,
            });
        }

        Ok(())
    }

    /// Get current storage usage information for a user
    pub async fn get_storage_usage(
        &self,
        user_id: &str,
        user_conn: &Connection,
    ) -> Result<StorageUsage> {
        // Calculate actual database size
        let used_bytes = self
            .calculate_database_size(user_conn, user_id)
            .await
            .map_err(StorageQuotaError::DatabaseError)?;

        let limit_bytes = STORAGE_QUOTA_LIMIT_BYTES;
        let remaining_bytes = limit_bytes.saturating_sub(used_bytes);

        let used_mb = used_bytes as f64 / (1024.0 * 1024.0);
        let limit_mb = limit_bytes as f64 / (1024.0 * 1024.0);
        let remaining_mb = remaining_bytes as f64 / (1024.0 * 1024.0);
        let percentage_used = (used_bytes as f64 / limit_bytes as f64) * 100.0;

        // Update cached value
        if let Err(e) = self.update_storage_usage(user_id, used_bytes).await {
            warn!("Failed to update storage cache for user {}: {}", user_id, e);
        }

        Ok(StorageUsage {
            used_bytes,
            limit_bytes,
            used_mb,
            limit_mb,
            remaining_bytes,
            remaining_mb,
            percentage_used,
        })
    }
}
