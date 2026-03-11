use anyhow::Result;
use chrono::{DateTime, Utc};
use libsql::Connection;
use serde::Serialize;

use crate::models::alerts::{AlertChannel, AlertType};
use crate::service::notifications::push::{PushPayload, PushService};
use crate::turso::config::WebPushConfig;

#[derive(Debug, Clone, Serialize)]
pub struct AlertEvent {
    pub id: String,
    pub rule_id: String,
    pub user_id: String,
    pub alert_type: AlertType,
    pub symbol: Option<String>,
    pub direction: Option<String>,
    pub threshold: Option<f64>,
    pub price: Option<f64>,
    pub percent_change: Option<f64>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: String,
    pub channels: Vec<AlertChannel>,
}

pub async fn dispatch_events(
    conn: &Connection,
    web_push: &WebPushConfig,
    user_id: &str,
    events: &[AlertEvent],
) -> Result<usize> {
    if events.is_empty() {
        return Ok(0);
    }

    let push_service = PushService::new(conn, web_push);
    let mut sent = 0usize;

    for event in events {
        if !event.channels.contains(&AlertChannel::Push) {
            continue;
        }

        let payload = build_push_payload(event);
        // send_to_user already fans out to all subscriptions for this user
        if push_service.send_to_user(user_id, &payload).await.is_ok() {
            sent += 1;
        }
    }

    Ok(sent)
}

fn build_push_payload(event: &AlertEvent) -> PushPayload {
    let ts_dt = DateTime::parse_from_rfc3339(&event.created_at)
        .unwrap_or_else(|_| Utc::now().into())
        .with_timezone(&Utc);
    let ts = ts_dt.timestamp();
    let date_display = ts_dt.format("%m/%d").to_string();
    let time_rfc = ts_dt.to_rfc3339();
    let title = match event.alert_type {
        AlertType::Price => format!(
            "Price alert: {}",
            event.symbol.as_deref().unwrap_or("symbol")
        ),
        AlertType::System => "System alert".to_string(),
    };

    let body = match event.alert_type {
        AlertType::Price => {
            let sym = event.symbol.as_deref().unwrap_or_default();
            let pct = event
                .percent_change
                .map(|p| format!("{:+.2}% today", p))
                .unwrap_or_else(|| "Alert triggered".to_string());
            format!("{sym} {pct} • {date_display} • {time_rfc}")
        }
        AlertType::System => "System notification".to_string(),
    };

    PushPayload {
        title,
        body: Some(body),
        icon: Some("/app/icon.png".to_string()),
        url: Some("/app/alerts".to_string()),
        tag: Some(event.rule_id.clone()),
        data: Some(serde_json::json!({
            "t": match event.alert_type { AlertType::Price => "price", AlertType::System => "system" },
            "sid": event.symbol,
            "p": event.price,
            "pct": event.percent_change,
            "dir": event.direction,
            "th": event.threshold,
            "rid": event.rule_id,
            "eid": event.id,
            "ts": ts,
        })),
    }
}
