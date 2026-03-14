use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEnvelope {
    #[serde(rename = "type")]
    pub r#type: String,
    pub request_id: String,
    pub session_id: String,
    pub user_id: String,
    #[serde(default)]
    pub payload: Value,
}

impl AgentEnvelope {
    pub fn new(
        message_type: impl Into<String>,
        request_id: impl Into<String>,
        session_id: impl Into<String>,
        user_id: impl Into<String>,
        payload: Value,
    ) -> Self {
        Self {
            r#type: message_type.into(),
            request_id: request_id.into(),
            session_id: session_id.into(),
            user_id: user_id.into(),
            payload,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AgentRequest {
    pub request_id: String,
    pub session_id: String,
    pub user_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallPayload {
    pub tool_call_id: String,
    pub tool_name: String,
    #[serde(default)]
    pub arguments: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResultPayload {
    pub tool_call_id: String,
    pub tool_name: String,
    pub ok: bool,
    #[serde(default)]
    pub result: Value,
    pub error: Option<String>,
}
