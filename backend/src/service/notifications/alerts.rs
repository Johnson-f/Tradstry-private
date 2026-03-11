use anyhow::{Result, anyhow};
use libsql::{Connection, Value, params};
use serde_json::json;

use crate::models::alerts::{
    AlertChannel, AlertRule, AlertType, CreateAlertRuleRequest, UpdateAlertRuleRequest,
};

pub struct AlertRuleService<'a> {
    conn: &'a Connection,
}

impl<'a> AlertRuleService<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub async fn create_rule(
        &self,
        user_id: &str,
        req: CreateAlertRuleRequest,
    ) -> Result<AlertRule> {
        validate_rule(&req)?;
        let rule = AlertRule::new(user_id, req);
        let channels_json = serde_json::to_string(&rule.channels)?;
        let alert_type_json = serde_json::to_string(&rule.alert_type)?;

        self.conn
            .execute(
                "INSERT INTO alert_rules (
                    id, user_id, alert_type, symbol, operator, threshold, channels,
                    cooldown_seconds, last_fired_at, note, max_triggers, fired_count,
                    is_active, metadata, created_at, updated_at
                 )
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    rule.id.clone(),
                    rule.user_id.clone(),
                    alert_type_json,
                    rule.symbol.clone(),
                    rule.operator.clone(),
                    rule.threshold,
                    channels_json,
                    rule.cooldown_seconds,
                    rule.last_fired_at.clone(),
                    rule.note.clone(),
                    rule.max_triggers,
                    rule.fired_count,
                    rule.is_active,
                    rule.metadata.clone().map(|m| m.to_string()),
                    rule.created_at.clone(),
                    rule.updated_at.clone()
                ],
            )
            .await?;

        Ok(rule)
    }

    pub async fn list_rules(&self, user_id: &str) -> Result<Vec<AlertRule>> {
        let mut rows = self
            .conn
            .prepare(
                "SELECT id, user_id, alert_type, symbol, operator, threshold, channels, cooldown_seconds, last_fired_at, note, max_triggers, fired_count, is_active, metadata, created_at, updated_at
                 FROM alert_rules WHERE user_id = ? ORDER BY created_at DESC",
            )
            .await?
            .query(params![user_id])
            .await?;

        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            results.push(row_to_rule(row)?);
        }
        Ok(results)
    }

    pub async fn get_rule(&self, user_id: &str, id: &str) -> Result<Option<AlertRule>> {
        let mut rows = self
            .conn
            .prepare(
                "SELECT id, user_id, alert_type, symbol, operator, threshold, channels, cooldown_seconds, last_fired_at, note, max_triggers, fired_count, is_active, metadata, created_at, updated_at
                 FROM alert_rules WHERE user_id = ? AND id = ? LIMIT 1",
            )
            .await?
            .query(params![user_id, id])
            .await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(row_to_rule(row)?))
        } else {
            Ok(None)
        }
    }

    pub async fn update_rule(
        &self,
        user_id: &str,
        id: &str,
        req: UpdateAlertRuleRequest,
    ) -> Result<Option<AlertRule>> {
        // Build dynamic SET clauses
        let mut set_clauses: Vec<&str> = Vec::new();
        let mut params: Vec<Value> = Vec::new();

        if let Some(symbol) = req.symbol {
            set_clauses.push("symbol = ?");
            params.push(symbol.into());
        }
        if let Some(operator) = req.operator {
            set_clauses.push("operator = ?");
            params.push(operator.into());
        }
        if let Some(threshold) = req.threshold {
            set_clauses.push("threshold = ?");
            params.push(threshold.into());
        }
        if let Some(channels) = req.channels {
            let channels_json = serde_json::to_string(&channels)?;
            set_clauses.push("channels = ?");
            params.push(channels_json.into());
        }
        if let Some(cooldown) = req.cooldown_seconds {
            set_clauses.push("cooldown_seconds = ?");
            params.push(cooldown.into());
        }
        if let Some(last_fired_at) = req.last_fired_at {
            set_clauses.push("last_fired_at = ?");
            params.push(last_fired_at.into());
        }
        if let Some(note) = req.note {
            set_clauses.push("note = ?");
            params.push(note.into());
        }
        if let Some(max_triggers) = req.max_triggers {
            set_clauses.push("max_triggers = ?");
            params.push(max_triggers.into());
        }
        if let Some(fired_count) = req.fired_count {
            set_clauses.push("fired_count = ?");
            params.push(fired_count.into());
        }
        if let Some(is_active) = req.is_active {
            set_clauses.push("is_active = ?");
            params.push((is_active as i64).into());
        }
        if let Some(metadata) = req.metadata {
            set_clauses.push("metadata = ?");
            params.push(metadata.to_string().into());
        }

        if set_clauses.is_empty() {
            return self.get_rule(user_id, id).await;
        }

        set_clauses.push("updated_at = ?");
        params.push(chrono::Utc::now().to_rfc3339().into());

        params.push(id.into());
        params.push(user_id.into());

        let sql = format!(
            "UPDATE alert_rules SET {} WHERE id = ? AND user_id = ?",
            set_clauses.join(", ")
        );

        self.conn.execute(&sql, params).await?;
        self.get_rule(user_id, id).await
    }

    pub async fn delete_rule(&self, user_id: &str, id: &str) -> Result<bool> {
        let deleted = self
            .conn
            .execute(
                "DELETE FROM alert_rules WHERE id = ? AND user_id = ?",
                params![id, user_id],
            )
            .await?;
        Ok(deleted > 0)
    }

    pub async fn mark_fired(&self, user_id: &str, id: &str, ts: &str) -> Result<()> {
        self.conn
            .execute(
                "UPDATE alert_rules
                 SET last_fired_at = ?,
                     fired_count = fired_count + 1,
                     is_active = CASE
                         WHEN max_triggers IS NOT NULL AND (fired_count + 1) >= max_triggers THEN 0
                         ELSE is_active
                     END,
                     updated_at = ?
                 WHERE id = ? AND user_id = ?",
                params![ts, ts, id, user_id],
            )
            .await?;
        Ok(())
    }
}

fn validate_rule(req: &CreateAlertRuleRequest) -> Result<()> {
    if matches!(req.alert_type, AlertType::Price) {
        if req.symbol.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
            return Err(anyhow!("symbol is required for price alerts"));
        }
        if req.threshold.is_none() {
            return Err(anyhow!("threshold is required for price alerts"));
        }
        if req.operator.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
            return Err(anyhow!("operator is required for price alerts"));
        }
    }
    if let Some(max) = req.max_triggers && max <= 0 {
        return Err(anyhow!("max_triggers must be positive when provided"));
    }
    Ok(())
}

fn row_to_rule(row: libsql::Row) -> Result<AlertRule> {
    let alert_type_raw: String = row.get(2)?;
    let alert_type: AlertType = serde_json::from_str(&alert_type_raw)
        .or_else(|_| serde_json::from_value(json!(alert_type_raw.to_lowercase())))?;

    let channels_raw: String = row.get(6)?;
    let channels: Vec<AlertChannel> =
        serde_json::from_str(&channels_raw).unwrap_or_else(|_| vec![AlertChannel::Push]);

    let note: Option<String> = row.get(9)?;
    let max_triggers: Option<i64> = row.get(10)?;
    let fired_count: i64 = row.get(11)?;
    let is_active_raw: i64 = row.get(12)?;
    let metadata_raw: Option<String> = row.get(13)?;
    let metadata = metadata_raw
        .as_ref()
        .and_then(|m| serde_json::from_str(m).ok());

    Ok(AlertRule {
        id: row.get(0)?,
        user_id: row.get(1)?,
        alert_type,
        symbol: row.get(3)?,
        operator: row.get(4)?,
        threshold: row.get(5)?,
        channels,
        cooldown_seconds: row.get(7)?,
        last_fired_at: row.get(8)?,
        note,
        max_triggers,
        fired_count,
        is_active: is_active_raw != 0,
        metadata,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}
