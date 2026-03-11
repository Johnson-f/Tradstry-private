use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AlertType {
    Price,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AlertChannel {
    Push,
    InApp,
    Email,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub id: String,
    pub user_id: String,
    pub alert_type: AlertType,
    pub symbol: Option<String>,
    pub operator: Option<String>,
    pub threshold: Option<f64>,
    pub channels: Vec<AlertChannel>,
    pub cooldown_seconds: Option<i64>,
    pub last_fired_at: Option<String>,
    pub note: Option<String>,
    pub max_triggers: Option<i64>,
    pub fired_count: i64,
    pub is_active: bool,
    pub metadata: Option<serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAlertRuleRequest {
    pub alert_type: AlertType,
    pub symbol: Option<String>,
    pub operator: Option<String>,
    pub threshold: Option<f64>,
    pub channels: Option<Vec<AlertChannel>>,
    pub cooldown_seconds: Option<i64>,
    pub note: Option<String>,
    pub max_triggers: Option<i64>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateAlertRuleRequest {
    pub symbol: Option<String>,
    pub operator: Option<String>,
    pub threshold: Option<f64>,
    pub channels: Option<Vec<AlertChannel>>,
    pub cooldown_seconds: Option<i64>,
    pub last_fired_at: Option<String>,
    pub note: Option<String>,
    pub max_triggers: Option<i64>,
    pub fired_count: Option<i64>,
    pub is_active: Option<bool>,
    pub metadata: Option<serde_json::Value>,
}

impl AlertRule {
    pub fn new(user_id: &str, req: CreateAlertRuleRequest) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            alert_type: req.alert_type,
            symbol: req.symbol,
            operator: req.operator,
            threshold: req.threshold,
            channels: req.channels.unwrap_or_else(|| vec![AlertChannel::Push]),
            cooldown_seconds: req.cooldown_seconds,
            last_fired_at: None,
            note: req.note,
            max_triggers: req.max_triggers,
            fired_count: 0,
            is_active: true,
            metadata: req.metadata,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}
