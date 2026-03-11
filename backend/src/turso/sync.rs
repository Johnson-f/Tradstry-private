//! Background schema synchronization module
//!
//! This module handles automatic schema synchronization for all user databases
//! when the backend starts up. It runs in the background (non-blocking) and
//! uses parallel processing for efficiency.

use std::sync::Arc;

use futures_util::stream::{self, StreamExt};
use log::{error, info, warn};

use super::client::TursoClient;
use super::schema::get_current_schema_version;

/// Maximum number of concurrent schema sync operations
const MAX_CONCURRENT_SYNCS: usize = 10;

/// Sync result for tracking progress
#[derive(Debug, Default)]
struct SyncStats {
    total: usize,
    synced: usize,
    skipped: usize,
    failed: usize,
}

/// Spawns a background task to sync all user database schemas on startup.
///
/// This function returns immediately - the sync happens in a spawned task.
/// It fetches all user databases from the registry, checks their schema versions
/// (stored in registry, not individual DBs), and syncs any that are out of date.
pub fn sync_all_user_schemas_background(turso_client: Arc<TursoClient>) {
    tokio::spawn(async move {
        info!("🔄 Starting background schema sync for all users...");

        let expected_version = get_current_schema_version();
        info!("Expected schema version: {}", expected_version.version);

        // Fetch all user database entries
        let entries = match turso_client.get_all_user_database_entries().await {
            Ok(entries) => entries,
            Err(e) => {
                error!(
                    "❌ Failed to fetch user databases for background sync: {}",
                    e
                );
                return;
            }
        };

        let total = entries.len();
        if total == 0 {
            info!("✅ No user databases to sync");
            return;
        }

        info!("📊 Found {} user databases to check", total);

        // Use Arc for shared state across concurrent tasks
        let stats = Arc::new(tokio::sync::Mutex::new(SyncStats {
            total,
            ..Default::default()
        }));

        let expected_version_str = expected_version.version.clone();
        let expected_version = Arc::new(expected_version);

        // Process entries concurrently with a limit
        stream::iter(entries)
            .for_each_concurrent(MAX_CONCURRENT_SYNCS, |entry| {
                let turso_client = Arc::clone(&turso_client);
                let stats = Arc::clone(&stats);
                let expected_version = Arc::clone(&expected_version);
                let expected_version_str = expected_version_str.clone();

                async move {
                    let user_id = &entry.user_id;

                    // Check schema version from registry (fast, no DB connection needed)
                    let needs_sync = match &entry.schema_version {
                        Some(version) if version == &expected_version.version => {
                            // Already up to date according to registry
                            let mut s = stats.lock().await;
                            s.skipped += 1;
                            false
                        }
                        Some(version) => {
                            info!(
                                "📝 User {} needs schema update: {} -> {}",
                                user_id, version, expected_version.version
                            );
                            true
                        }
                        None => {
                            // No schema version in registry - needs sync
                            info!("📝 User {} has no schema version in registry, needs sync", user_id);
                            true
                        }
                    };

                    if needs_sync {
                        match turso_client.sync_user_database_schema(user_id).await {
                            Ok(_) => {
                                // Update registry with new schema version
                                if let Err(e) = turso_client
                                    .update_registry_schema_version(user_id, &expected_version_str)
                                    .await
                                {
                                    warn!(
                                        "⚠️ Schema synced but failed to update registry for user {}: {}",
                                        user_id, e
                                    );
                                }
                                let mut s = stats.lock().await;
                                s.synced += 1;
                                info!("✅ Schema synced for user {}", user_id);
                            }
                            Err(e) => {
                                let mut s = stats.lock().await;
                                s.failed += 1;
                                warn!("❌ Failed to sync schema for user {}: {}", user_id, e);
                            }
                        }
                    }
                }
            })
            .await;

        // Log final stats
        let final_stats = stats.lock().await;
        info!(
            "🏁 Background schema sync complete: {} total, {} synced, {} skipped (up-to-date), {} failed",
            final_stats.total, final_stats.synced, final_stats.skipped, final_stats.failed
        );
    });
}
