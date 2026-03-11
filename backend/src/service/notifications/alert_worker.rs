use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use libsql::Connection;
use uuid::Uuid;

use crate::models::alerts::{AlertRule, AlertType};
use crate::service::notifications::alert_dispatcher::{AlertEvent, dispatch_events};
use crate::service::notifications::alerts::AlertRuleService;
use crate::turso::config::WebPushConfig;

#[derive(Debug, Clone)]
pub struct PricePoint {
    pub symbol: String,
    pub price: f64,
    #[allow(dead_code)]
    pub direction: Option<String>,
    pub percent_change: Option<f64>,
    pub timestamp: DateTime<Utc>,
}

pub async fn evaluate_price_rules_for_user(
    conn: &Connection,
    web_push: &WebPushConfig,
    user_id: &str,
    prices: &[PricePoint],
) -> Result<usize> {
    if prices.is_empty() {
        return Ok(0);
    }

    let service = AlertRuleService::new(conn);
    let rules = service.list_rules(user_id).await?;

    let mut events = Vec::new();
    for rule in rules
        .into_iter()
        .filter(|r| matches!(r.alert_type, AlertType::Price))
    {
        if let Some(event) = evaluate_rule(&rule, prices) {
            events.push(event);
        }
    }

    let dispatched = dispatch_events(conn, web_push, user_id, &events).await?;

    // Update last fired timestamps for successfully dispatched events
    for event in events {
        let _ = service
            .mark_fired(user_id, &event.rule_id, &event.created_at)
            .await;
    }

    Ok(dispatched)
}

fn evaluate_rule(rule: &AlertRule, prices: &[PricePoint]) -> Option<AlertEvent> {
    if !rule.is_active {
        return None;
    }
    if let Some(limit) = rule.max_triggers && rule.fired_count >= limit {
        return None;
    }

    let symbol = rule.symbol.as_ref()?;
    let operator = rule.operator.as_deref()?.to_lowercase();
    let threshold = rule.threshold?;

    let point = prices
        .iter()
        .find(|p| p.symbol.eq_ignore_ascii_case(symbol))?;

    if !is_condition_met(operator.as_str(), point.price, threshold) {
        return None;
    }

    if let Some(cooldown) = rule.cooldown_seconds
        && let Some(last_fired) = rule
            .last_fired_at
            .as_ref()
            .and_then(|ts| ts.parse::<DateTime<Utc>>().ok())
    {
        let elapsed = point.timestamp - last_fired;
        if elapsed < Duration::seconds(cooldown) {
            return None;
        }
    }

    Some(AlertEvent {
        id: Uuid::new_v4().to_string(),
        rule_id: rule.id.clone(),
        user_id: rule.user_id.clone(),
        alert_type: rule.alert_type.clone(),
        symbol: rule.symbol.clone(),
        direction: rule.operator.clone(),
        threshold: rule.threshold,
        price: Some(point.price),
        percent_change: point.percent_change,
        metadata: rule.metadata.clone(),
        created_at: point.timestamp.to_rfc3339(),
        channels: rule.channels.clone(),
    })
}

fn is_condition_met(operator: &str, price: f64, threshold: f64) -> bool {
    match operator {
        "gte" | ">=" | "at_or_above" | "above" | "gt" | ">" => price >= threshold,
        "lte" | "<=" | "at_or_below" | "below" | "lt" | "<" => price <= threshold,
        _ => false,
    }
}
