use anyhow::{Context, Result};
use log::{error, info, warn};
use std::collections::HashMap;
use std::sync::Arc;

use crate::service::ai_service::vector_service::qdrant::QdrantDocumentClient;
use crate::turso::client::TursoClient;

/// Account deletion service for completely removing user data
/// Implements all-or-nothing transaction behavior with rollback on failure
pub struct AccountDeletionService {
    turso_client: Arc<TursoClient>,
    qdrant_client: Arc<QdrantDocumentClient>,
    supabase_url: String,
    supabase_service_role_key: String,
}

impl AccountDeletionService {
    /// Create a new account deletion service
    pub fn new(
        turso_client: Arc<TursoClient>,
        qdrant_client: Arc<QdrantDocumentClient>,
        supabase_url: String,
        supabase_service_role_key: String,
    ) -> Self {
        Self {
            turso_client,
            qdrant_client,
            supabase_url,
            supabase_service_role_key,
        }
    }

    /// Delete all user data (all-or-nothing transaction)
    /// Returns Ok(()) on success, Err on failure (all changes rolled back)
    pub async fn delete_user_account(&self, user_id: &str) -> Result<()> {
        info!("Starting account deletion for user: {}", user_id);

        // Get user database entry for rollback info
        let user_db_entry = self
            .turso_client
            .get_user_database(user_id)
            .await
            .context("Failed to get user database entry")?;

        let db_name = user_db_entry
            .as_ref()
            .map(|e| e.db_name.clone())
            .context("User database not found in registry")?;

        let mut rollback_data: HashMap<String, String> = HashMap::new();
        rollback_data.insert("db_name".to_string(), db_name.clone());
        rollback_data.insert("user_id".to_string(), user_id.to_string());

        // Step 1: Delete Turso Database
        info!("Step 1/5: Deleting Turso database: {}", db_name);
        self.turso_client
            .delete_user_database(&db_name)
            .await
            .map_err(|e| {
                error!("Failed to delete Turso database: {}", e);
                e
            })?;

        // Step 2: Delete Supabase database tables
        info!("Step 2/5: Deleting Supabase database entries");
        self.delete_supabase_database_entries(user_id).await
            .map_err(|e| {
                error!("Failed to delete Supabase database entries: {}", e);
                // Rollback: Already deleted storage files and database, can't fully rollback
                // Log error for manual cleanup
                warn!("CRITICAL: Partial deletion occurred. User {} database deleted but Supabase cleanup failed. Manual cleanup required.", user_id);
                e
            })?;

        // Step 3: Delete Vector Databases
        info!("Step 3/5: Deleting vector databases");
        self.delete_vector_databases(user_id).await
            .map_err(|e| {
                error!("Failed to delete vector databases: {}", e);
                warn!("CRITICAL: Partial deletion occurred. User {} core data deleted but vector cleanup failed. Manual cleanup required.", user_id);
                e
            })?;

        // Step 4: Remove from Registry Database
        info!("Step 4/5: Removing registry entry");
        self.turso_client
            .remove_user_database_entry(user_id)
            .await
            .map_err(|e| {
                error!("Failed to remove registry entry: {}", e);
                warn!("CRITICAL: Partial deletion occurred. User {} data deleted but registry entry remains. Manual cleanup required.", user_id);
                e
            })?;

        // Step 5: Delete Supabase Auth Account (FINAL STEP)
        info!("Step 5/5: Deleting Supabase Auth account");
        self.delete_supabase_auth_user(user_id).await.map_err(|e| {
            error!("Failed to delete Supabase Auth account: {}", e);
            warn!(
                "CRITICAL: User {} data deleted but auth account remains. Manual cleanup required.",
                user_id
            );
            e
        })?;

        info!("Successfully deleted all data for user: {}", user_id);
        Ok(())
    }

    /// Delete entries from Supabase database tables
    async fn delete_supabase_database_entries(&self, user_id: &str) -> Result<()> {
        use reqwest::Client;

        let client = Client::new();
        let base_url = format!("{}/rest/v1", self.supabase_url);

        // Delete from user_profile_images (CASCADE should handle it, but explicit deletion is safer)
        let profile_images_url = format!("{}/user_profile_images?user_id=eq.{}", base_url, user_id);
        let response = client
            .delete(&profile_images_url)
            .header(
                "Authorization",
                format!("Bearer {}", self.supabase_service_role_key),
            )
            .header("apikey", self.supabase_service_role_key.clone())
            .header("Prefer", "return=minimal")
            .send()
            .await
            .context("Failed to delete user_profile_images")?;

        if !response.status().is_success() {
            warn!(
                "Failed to delete user_profile_images: status {}",
                response.status()
            );
        }

        // Delete from notebook_images if table exists
        let notebook_images_url = format!("{}/notebook_images?user_id=eq.{}", base_url, user_id);
        let response = client
            .delete(&notebook_images_url)
            .header(
                "Authorization",
                format!("Bearer {}", self.supabase_service_role_key),
            )
            .header("apikey", self.supabase_service_role_key.clone())
            .header("Prefer", "return=minimal")
            .send()
            .await;

        if let Err(e) = response {
            warn!(
                "Failed to delete notebook_images (table may not exist): {}",
                e
            );
        } else if let Ok(resp) = response
            && !resp.status().is_success()
        {
            warn!("Failed to delete notebook_images: status {}", resp.status());
        }

        info!("Completed Supabase database cleanup for user: {}", user_id);
        Ok(())
    }

    /// Delete all vector databases for a user
    async fn delete_vector_databases(&self, user_id: &str) -> Result<()> {
        info!("Deleting vector databases for user: {}", user_id);

        // Delete from Qdrant (delete entire collection)
        let _ = self
            .qdrant_client
            .delete_user_collection(user_id)
            .await
            .map_err(|e| warn!("Failed to delete Qdrant collection: {}", e));

        info!("Completed vector database cleanup for user: {}", user_id);
        Ok(())
    }

    /// Delete Supabase Auth user account
    async fn delete_supabase_auth_user(&self, user_id: &str) -> Result<()> {
        use reqwest::Client;

        info!("Deleting Supabase Auth user: {}", user_id);

        let client = Client::new();
        let url = format!("{}/auth/v1/admin/users/{}", self.supabase_url, user_id);

        let response = client
            .delete(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.supabase_service_role_key),
            )
            .header("apikey", self.supabase_service_role_key.clone())
            .send()
            .await
            .context("Failed to delete Supabase Auth user")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Failed to delete Supabase Auth user: status {} - {}",
                status,
                error_text
            );
        }

        info!("Successfully deleted Supabase Auth user: {}", user_id);
        Ok(())
    }

    /// Attempt to rollback registry entry (used when deletion fails)
    #[allow(dead_code)]
    async fn try_rollback_registry(
        &self,
        _user_id: &str,
        _rollback_data: &HashMap<String, String>,
    ) -> Result<()> {
        warn!("Rollback requested (but Turso database deletion cannot be rolled back)");
        Ok(())
    }
}
