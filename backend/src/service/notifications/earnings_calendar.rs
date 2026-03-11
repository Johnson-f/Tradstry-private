use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result, anyhow};
use chrono::{Datelike, Timelike, Utc};
use chrono_tz::America::New_York;
use futures::stream::{self, StreamExt};
use log::{error, info, warn};
use tokio::time::{MissedTickBehavior, interval};
use uuid::Uuid;

use crate::{
    service::{
        market_engine::{
            calendar::{EarningsDay, fetch_earnings_calendar},
            quotes::{SimpleQuotesResponse, get_simple_quotes},
        },
        notifications::push::{PushPayload, PushService},
    },
    turso::AppState,
};

const DISPATCH_HOUR: u32 = 15;
const DISPATCH_MINUTE: u32 = 30;
const MIN_MARKET_CAP: i64 = 1_000_000_000; // $1B
const MIN_AVG_VOLUME: i64 = 10_000_000; // 10M
const NOTIFICATION_TYPE: &str = "earnings_calendar";

#[derive(Debug, Clone)]
pub struct DispatchSummary {
    pub sent: usize,
    pub failed: usize,
    #[allow(dead_code)]
    pub after_hours: Vec<String>,
    #[allow(dead_code)]
    pub premarket: Vec<String>,
}

pub fn start_earnings_calendar_scheduler(app_state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(60));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            ticker.tick().await;
            if let Err(err) = run_cycle(&app_state).await {
                error!("Earnings calendar notification cycle failed: {err}");
            }
        }
    });
}

/// Manually run the earnings calendar flow once.
/// `from_date`/`to_date` should be YYYY-MM-DD; if None, defaults to today/tomorrow ET.
/// `dry_run` skips push sends and dispatch logging (used for testing).
#[allow(dead_code)]
pub async fn run_earnings_calendar_once(
    app_state: Arc<AppState>,
    from_date: Option<&str>,
    to_date: Option<&str>,
    dry_run: bool,
) -> Result<DispatchSummary> {
    execute_earnings_flow(&app_state, from_date, to_date, dry_run).await
}

async fn run_cycle(app_state: &Arc<AppState>) -> Result<()> {
    let now_et = Utc::now().with_timezone(&New_York);

    if !is_weekday(&now_et) {
        return Ok(());
    }

    if !is_dispatch_minute(&now_et) {
        return Ok(());
    }

    let today = now_et.date_naive();
    let slot_key = format!("earnings-{}", today);

    let registry_conn = app_state.turso_client.get_registry_connection().await?;

    if slot_already_sent(&registry_conn, &slot_key).await? {
        return Ok(());
    }

    let summary = execute_earnings_flow(app_state, None, None, false).await?;

    record_dispatch(&registry_conn, &slot_key, summary.sent, summary.failed).await?;

    info!(
        "Earnings calendar notification sent. Slot={}, sent={}, failed={}",
        slot_key, summary.sent, summary.failed
    );

    Ok(())
}

async fn execute_earnings_flow(
    app_state: &Arc<AppState>,
    from_date: Option<&str>,
    to_date: Option<&str>,
    dry_run: bool,
) -> Result<DispatchSummary> {
    let now_et = Utc::now().with_timezone(&New_York);
    let today = from_date
        .map(|d| d.to_string())
        .unwrap_or_else(|| now_et.date_naive().to_string());
    let tomorrow = if let Some(d) = to_date {
        d.to_string()
    } else {
        let next = now_et
            .date_naive()
            .succ_opt()
            .ok_or_else(|| anyhow!("Failed to compute tomorrow date"))?;
        next.to_string()
    };

    let calendar = fetch_earnings_calendar(Some(&today), Some(&tomorrow), false)
        .await
        .context("Failed to fetch earnings calendar")?;

    let after_hours_today = select_symbols_for_day(&calendar.earnings, &today, is_after_hours);
    let premarket_tomorrow = select_symbols_for_day(&calendar.earnings, &tomorrow, is_pre_market);

    if after_hours_today.is_empty() && premarket_tomorrow.is_empty() {
        warn!("No earnings symbols found for AH today / BMO tomorrow");
        return Ok(DispatchSummary {
            sent: 0,
            failed: 0,
            after_hours: Vec::new(),
            premarket: Vec::new(),
        });
    }

    let filtered_after_hours = filter_by_cap_and_volume(&after_hours_today).await?;
    let filtered_premarket = filter_by_cap_and_volume(&premarket_tomorrow).await?;

    if filtered_after_hours.is_empty() && filtered_premarket.is_empty() {
        warn!("All earnings symbols filtered out by cap/volume thresholds");
        return Ok(DispatchSummary {
            sent: 0,
            failed: 0,
            after_hours: Vec::new(),
            premarket: Vec::new(),
        });
    }

    let user_ids = app_state
        .turso_client
        .list_all_user_ids()
        .await
        .unwrap_or_default();
    if user_ids.is_empty() {
        warn!("No users found for earnings calendar dispatch");
        return Ok(DispatchSummary {
            sent: 0,
            failed: 0,
            after_hours: filtered_after_hours,
            premarket: filtered_premarket,
        });
    }

    let payload = build_payload(&filtered_after_hours, &filtered_premarket);
    let web_push = app_state.config.web_push.clone();

    let (sent_count, failed_count) = if dry_run {
        (0usize, 0usize)
    } else {
        stream::iter(user_ids)
            .map(|user_id| {
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
                                warn!(
                                    "Failed to send earnings calendar notification to {user_id}: {err}"
                                );
                                (0usize, 1usize)
                            } else {
                                (1usize, 0usize)
                            }
                        }
                        Ok(None) => {
                            warn!("No database for user {user_id}, skipping earnings calendar notification");
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
            })
            .buffer_unordered(8)
            .fold((0usize, 0usize), |(sent, failed), (s, f)| async move {
                (sent + s, failed + f)
            })
            .await
    };

    Ok(DispatchSummary {
        sent: sent_count,
        failed: failed_count,
        after_hours: filtered_after_hours,
        premarket: filtered_premarket,
    })
}

fn is_weekday(now_et: &chrono::DateTime<chrono_tz::Tz>) -> bool {
    !matches!(
        now_et.weekday(),
        chrono::Weekday::Sat | chrono::Weekday::Sun
    )
}

fn is_dispatch_minute(now_et: &chrono::DateTime<chrono_tz::Tz>) -> bool {
    now_et.hour() == DISPATCH_HOUR && now_et.minute() == DISPATCH_MINUTE
}

async fn slot_already_sent(conn: &libsql::Connection, slot_key: &str) -> Result<bool> {
    let mut rows = conn
        .prepare("SELECT 1 FROM notification_dispatch_log WHERE slot_key = ? LIMIT 1")
        .await?
        .query(libsql::params![slot_key])
        .await?;
    Ok(rows.next().await?.is_some())
}

async fn record_dispatch(
    conn: &libsql::Connection,
    slot_key: &str,
    sent_count: usize,
    failed_count: usize,
) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO notification_dispatch_log (id, notification_id, notification_type, slot_key, sent_count, failed_count, created_at)
         VALUES (?, ?, ?, ?, ?, ?, datetime('now'))",
        libsql::params![
            Uuid::new_v4().to_string(),
            Option::<String>::None,
            NOTIFICATION_TYPE,
            slot_key,
            sent_count as i64,
            failed_count as i64
        ],
    )
    .await
    .context("Failed to record earnings calendar dispatch")?;
    Ok(())
}

fn select_symbols_for_day(
    earnings: &HashMap<String, EarningsDay>,
    date: &str,
    predicate: fn(&str) -> bool,
) -> Vec<String> {
    earnings
        .get(date)
        .map(|day| {
            day.stocks
                .iter()
                .filter(|s| predicate(&s.time))
                .map(|s| s.symbol.to_uppercase())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

async fn filter_by_cap_and_volume(symbols: &[String]) -> Result<Vec<String>> {
    let mut unique = HashSet::new();
    for s in symbols {
        unique.insert(s.to_uppercase());
    }

    if unique.is_empty() {
        return Ok(Vec::new());
    }

    let symbol_vec: Vec<String> = unique.into_iter().collect();
    let symbol_refs: Vec<&str> = symbol_vec.iter().map(|s| s.as_str()).collect();
    let quotes_value = get_simple_quotes(&symbol_refs)
        .await
        .context("Failed to fetch quotes for earnings symbols")?;
    let parsed: SimpleQuotesResponse =
        serde_json::from_value(quotes_value).context("Failed to parse simple quotes response")?;

    let results = parsed.quote_response.result.unwrap_or_default();

    let filtered = results
        .into_iter()
        .filter(|q| {
            let market_cap = q.market_cap.unwrap_or(0);
            let adv = q
                .average_daily_volume_10_day
                .or(q.average_daily_volume_3_month)
                .unwrap_or(0);
            market_cap >= MIN_MARKET_CAP && adv >= MIN_AVG_VOLUME
        })
        .map(|q| q.symbol.to_uppercase())
        .collect();

    Ok(filtered)
}

fn is_after_hours(time: &str) -> bool {
    let lower = time.to_lowercase();
    lower.contains("after") || lower.contains("post") || lower.contains("pm")
}

fn is_pre_market(time: &str) -> bool {
    let lower = time.to_lowercase();
    lower.contains("before") || lower.contains("pre") || lower.contains("am")
}

fn build_payload(after_hours: &[String], premarket: &[String]) -> PushPayload {
    let ah_preview = preview_list(after_hours);
    let pre_preview = preview_list(premarket);

    let body = match (ah_preview.as_deref(), pre_preview.as_deref()) {
        (Some(ah), Some(pre)) => format!("AH today: {ah} | BMO tomorrow: {pre}"),
        (Some(ah), None) => format!("After-hours today: {ah}"),
        (None, Some(pre)) => format!("Pre-market tomorrow: {pre}"),
        _ => "Earnings watchlist update".to_string(),
    };

    PushPayload {
        title: "Earnings watch".to_string(),
        body: Some(body),
        icon: None,
        url: None,
        tag: Some(NOTIFICATION_TYPE.to_string()),
        data: None,
    }
}

fn preview_list(symbols: &[String]) -> Option<String> {
    if symbols.is_empty() {
        return None;
    }
    let preview: Vec<String> = symbols.iter().take(8).cloned().collect();
    let mut text = preview.join(", ");
    if symbols.len() > preview.len() {
        text.push_str(", …");
    }
    Some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration-ish sanity check. Requires real env + VAPID if you set dry_run=false.
    // By default, this test is ignored. To run:
    // RUN_EARNINGS_CALENDAR_TEST=1 cargo test earnings_calendar_once -- --ignored
    #[tokio::test]
    #[ignore]
    async fn earnings_calendar_once_dry_run() -> Result<()> {
        if std::env::var("RUN_EARNINGS_CALENDAR_TEST")
            .unwrap_or_default()
            .to_lowercase()
            != "1"
        {
            return Ok(());
        }

        // Load .env for local runs; if SUPABASE_URL is still missing, skip gracefully.
        let _ = dotenvy::dotenv();
        if std::env::var("SUPABASE_URL").is_err() {
            println!("SUPABASE_URL not set; skipping earnings calendar test");
            return Ok(());
        }

        let app_state = AppState::new()
            .await
            .map_err(|e| anyhow!("init app state failed: {e}"))?;
        let summary = run_earnings_calendar_once(Arc::new(app_state), None, None, true).await?;

        // Dry run must not attempt to send (sent/failed == 0), but we allow empty symbol sets.
        assert_eq!(summary.sent, 0);
        assert_eq!(summary.failed, 0);
        Ok(())
    }
}
