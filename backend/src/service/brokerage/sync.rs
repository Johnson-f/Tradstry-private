use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use futures::stream::{self, StreamExt};
use libsql::Connection;
use log::{error, info, warn};
use tokio::time::{MissedTickBehavior, interval};

use crate::service::brokerage::accounts::sync_accounts;
use crate::service::brokerage::client::SnapTradeClient;
use crate::service::notifications::push::{PushPayload, PushService};
use crate::turso::AppState;

/// Starts a background task that periodically syncs brokerage accounts, holdings,
/// and transactions for users that have a pending brokerage connection.
pub fn start_brokerage_sync_scheduler(
    app_state: Arc<AppState>,
    interval_secs: u64,
    max_concurrency: usize,
) {
    let app_state_clone = Arc::clone(&app_state);
    let snaptrade_base_url = app_state.config.snaptrade_service_url.clone();

    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(interval_secs));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        let client = match SnapTradeClient::new(snaptrade_base_url) {
            Ok(c) => c,
            Err(e) => {
                error!("Brokerage sync disabled; failed to create SnapTrade client: {e}");
                return;
            }
        };

        info!(
            "Brokerage sync scheduler started: interval={}s, max_concurrency={}",
            interval_secs, max_concurrency
        );

        loop {
            ticker.tick().await;
            if let Err(e) = run_sync_cycle(&app_state_clone, &client, max_concurrency).await {
                error!("Brokerage sync cycle failed: {}", e);
            }
        }
    });
}

async fn run_sync_cycle(
    app_state: &AppState,
    client: &SnapTradeClient,
    max_concurrency: usize,
) -> Result<()> {
    let user_ids = app_state.turso_client.list_all_user_ids().await?;

    let app_state_clone = app_state.clone();
    stream::iter(user_ids)
        .for_each_concurrent(max_concurrency, move |user_id| {
            let app_state = app_state_clone.clone();
            let client = client.clone();

            async move {
                match app_state
                    .turso_client
                    .get_user_database_connection(&user_id)
                    .await
                {
                    Ok(Some(conn)) => match user_has_pending_brokerage(&conn).await {
                        Ok(true) => {
                            if let Err(e) = sync_accounts(&conn, &user_id, &client).await {
                                warn!("Failed to sync accounts for user {}: {}", user_id, e);
                            } else if let Err(e) =
                                send_merge_ready_notification(&conn, &app_state, &user_id).await
                            {
                                warn!(
                                    "Failed to send brokerage merge-ready notification to {}: {}",
                                    user_id, e
                                );
                            }
                        }
                        Ok(false) => {
                            // No pending brokerage connections; skip
                        }
                        Err(e) => warn!(
                            "Failed to check brokerage connections for user {}: {}",
                            user_id, e
                        ),
                    },
                    Ok(None) => {
                        // User has no database yet - skip
                    }
                    Err(e) => warn!(
                        "Failed to get database connection for user {}: {}",
                        user_id, e
                    ),
                }
            }
        })
        .await;

    Ok(())
}

async fn user_has_pending_brokerage(conn: &Connection) -> Result<bool> {
    let stmt = conn
        .prepare("SELECT 1 FROM brokerage_connections WHERE status = 'pending' LIMIT 1")
        .await?;

    let mut rows = stmt.query(libsql::params![]).await?;
    Ok(rows.next().await?.is_some())
}

async fn send_merge_ready_notification(
    conn: &Connection,
    app_state: &AppState,
    user_id: &str,
) -> Result<()> {
    let push_service = PushService::new(conn, &app_state.config.web_push);
    let payload = PushPayload {
        title: "Brokerage sync complete".to_string(),
        body: Some("Your synced trades are ready for merging.".to_string()),
        icon: None,
        url: Some("/app/brokerage".to_string()),
        tag: Some("brokerage-merge-ready".to_string()),
        data: None,
    };

    push_service.send_to_user(user_id, &payload).await
}
