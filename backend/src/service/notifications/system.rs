use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use chrono::{Timelike, Utc};
use chrono_tz::America::New_York;
use futures::stream::{self, StreamExt};
use libsql::{Connection, params};
use log::{error, info, warn};
use tokio::time::{MissedTickBehavior, interval};
use uuid::Uuid;

use crate::service::notifications::push::{PushPayload, PushService};
use crate::turso::AppState;

#[derive(Clone)]
struct NotificationTemplate {
    id: String,
    title: String,
    body: Option<String>,
    data: Option<String>,
}

const DISCIPLINE_SLOTS: [(u32, u32); 4] = [(9, 30), (11, 30), (13, 30), (15, 30)];

pub fn start_discipline_notification_scheduler(app_state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(60));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            ticker.tick().await;
            if let Err(err) = run_cycle(&app_state).await {
                error!("Discipline notification cycle failed: {err}");
            }
        }
    });
}

async fn run_cycle(app_state: &Arc<AppState>) -> Result<()> {
    let now_et = Utc::now().with_timezone(&New_York);

    let Some(slot_key) = current_slot_key(&now_et) else {
        return Ok(());
    };

    let registry_conn = app_state.turso_client.get_registry_connection().await?;

    if slot_already_sent(&registry_conn, &slot_key).await? {
        return Ok(());
    }

    let template = pick_random_discipline(&registry_conn).await?;

    let user_ids = app_state
        .turso_client
        .list_all_user_ids()
        .await
        .unwrap_or_default();
    if user_ids.is_empty() {
        warn!("No users found for discipline notification dispatch");
        record_dispatch(&registry_conn, &slot_key, &template.id, 0, 0).await?;
        return Ok(());
    }

    let payload = build_payload(&template);
    let web_push = app_state.config.web_push.clone();

    let (sent_count, failed_count) = stream::iter(user_ids.into_iter().map(|user_id| {
        let app_state = Arc::clone(app_state);
        let payload = payload.clone();
        let web_push = web_push.clone();
        async move {
            match app_state
                .turso_client
                .get_user_database_connection(&user_id)
                .await
            {
                Ok(Some(conn)) => {
                    let push_service = PushService::new(&conn, &web_push);
                    if let Err(err) = push_service.send_to_user(&user_id, &payload).await {
                        warn!("Failed to send discipline notification to {user_id}: {err}");
                        (0usize, 1usize)
                    } else {
                        (1usize, 0usize)
                    }
                }
                Ok(None) => {
                    warn!(
                        "No database for user {}, skipping discipline notification",
                        user_id
                    );
                    (0usize, 1usize)
                }
                Err(err) => {
                    warn!(
                        "Failed to get database connection for user {}: {}",
                        user_id, err
                    );
                    (0usize, 1usize)
                }
            }
        }
    }))
    .buffer_unordered(8)
    .fold((0usize, 0usize), |(sent, failed), (s, f)| async move {
        (sent + s, failed + f)
    })
    .await;

    record_dispatch(
        &registry_conn,
        &slot_key,
        &template.id,
        sent_count,
        failed_count,
    )
    .await?;

    info!(
        "Discipline notification dispatched for slot {} (sent={}, failed={})",
        slot_key, sent_count, failed_count
    );

    Ok(())
}

fn current_slot_key(now_et: &chrono::DateTime<chrono_tz::Tz>) -> Option<String> {
    let minutes = now_et.hour() * 60 + now_et.minute();
    let start = 9 * 60 + 30;
    let end = 17 * 60;

    if minutes < start || minutes > end {
        return None;
    }

    let mut candidate: Option<(u32, u32)> = None;
    for (h, m) in DISCIPLINE_SLOTS {
        let slot_min = h * 60 + m;
        if slot_min <= minutes {
            candidate = Some((h, m));
        }
    }

    candidate.map(|(h, m)| format!("{}-{:02}{:02}", now_et.date_naive(), h, m))
}

async fn slot_already_sent(conn: &Connection, slot_key: &str) -> Result<bool> {
    let mut rows = conn
        .prepare("SELECT 1 FROM notification_dispatch_log WHERE slot_key = ? LIMIT 1")
        .await?
        .query(params![slot_key])
        .await?;
    Ok(rows.next().await?.is_some())
}

async fn pick_random_discipline(conn: &Connection) -> Result<NotificationTemplate> {
    let mut rows = conn
        .prepare(
            "SELECT id, title, body, data FROM notifications WHERE notification_type = 'discipline' AND status = 'template' ORDER BY random() LIMIT 1",
        )
        .await?
        .query(params![])
        .await?;

    let Some(row) = rows.next().await? else {
        return Err(anyhow!("No discipline notification templates found"));
    };

    Ok(NotificationTemplate {
        id: row.get(0)?,
        title: row.get(1)?,
        body: row.get::<Option<String>>(2)?,
        data: row.get::<Option<String>>(3)?,
    })
}

async fn record_dispatch(
    conn: &Connection,
    slot_key: &str,
    notification_id: &str,
    sent_count: usize,
    failed_count: usize,
) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO notification_dispatch_log (id, notification_id, notification_type, slot_key, sent_count, failed_count, created_at)
         VALUES (?, ?, ?, ?, ?, ?, datetime('now'))",
        params![
            Uuid::new_v4().to_string(),
            notification_id,
            "discipline",
            slot_key,
            sent_count as i64,
            failed_count as i64
        ],
    )
    .await
    .context("Failed to record discipline notification dispatch")?;
    Ok(())
}

fn build_payload(template: &NotificationTemplate) -> PushPayload {
    let data_json = template
        .data
        .as_ref()
        .and_then(|d| serde_json::from_str::<serde_json::Value>(d).ok());

    let url = data_json
        .as_ref()
        .and_then(|d| d.get("url"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    PushPayload {
        title: template.title.clone(),
        body: template.body.clone(),
        icon: None,
        url,
        tag: Some("discipline".to_string()),
        data: data_json,
    }
}
