use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use serde_json::Value;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::service::agents::protocol::AgentRequest;
use crate::service::agents::AgentsClient;

#[derive(Debug, Clone)]
pub struct AiChatResponse {
    pub request_id: String,
    pub session_id: String,
    pub text: String,
    pub promoted_memory_uris: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "camelCase")]
pub enum AiChatStreamEvent {
    Delta {
        request_id: String,
        session_id: String,
        text: String,
    },
    Completed {
        request_id: String,
        session_id: String,
        text: String,
        promoted_memory_uris: Vec<String>,
    },
    Error {
        request_id: String,
        session_id: String,
        message: String,
    },
}

fn extract_promoted_memory_uris(payload: &Value) -> Vec<String> {
    payload
        .get("promotedMemoryUris")
        .or_else(|| payload.get("promoted_memory_uris"))
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

pub async fn send_chat_message(
    agents_client: &Option<AgentsClient>,
    user_id: &str,
    session_id: Option<String>,
    message: String,
) -> Result<AiChatResponse> {
    let mut receiver = stream_chat_events(agents_client, user_id, session_id, message).await?;

    let mut final_text = String::new();
    let mut request_id = String::new();
    let mut response_session_id = String::new();
    let mut promoted_memory_uris: Vec<String> = Vec::new();

    while let Some(event) = receiver.recv().await {
        match event {
            AiChatStreamEvent::Delta {
                request_id: current_request_id,
                session_id: current_session_id,
                text,
            } => {
                if request_id.is_empty() {
                    request_id = current_request_id;
                    response_session_id = current_session_id;
                }
                final_text.push_str(&text);
            }
            AiChatStreamEvent::Completed {
                request_id: current_request_id,
                session_id: current_session_id,
                text,
                promoted_memory_uris: uris,
            } => {
                request_id = current_request_id;
                response_session_id = current_session_id;
                final_text = text;
                promoted_memory_uris = uris;
                break;
            }
            AiChatStreamEvent::Error {
                request_id: current_request_id,
                session_id: current_session_id,
                message,
            } => {
                request_id = current_request_id;
                response_session_id = current_session_id;
                return Err(anyhow!(message));
            }
        }
    }

    if request_id.is_empty() {
        return Err(anyhow!("agents service disconnected before any response"));
    }

    Ok(AiChatResponse {
        request_id,
        session_id: response_session_id,
        text: final_text,
        promoted_memory_uris,
    })
}

pub async fn stream_chat_events(
    agents_client: &Option<AgentsClient>,
    user_id: &str,
    session_id: Option<String>,
    message: String,
) -> Result<mpsc::UnboundedReceiver<AiChatStreamEvent>> {
    let message = message.trim();
    if message.is_empty() {
        return Err(anyhow!("message cannot be empty"));
    }

    let agents_client = agents_client
        .as_ref()
        .context("Agents service is not configured")?;

    let request_id = Uuid::new_v4().to_string();
    let session_id = session_id.unwrap_or_else(|| Uuid::new_v4().to_string());
    let request = AgentRequest {
        request_id: request_id.clone(),
        session_id: session_id.clone(),
        user_id: user_id.to_string(),
        message: message.to_string(),
    };

    let mut events = agents_client
        .start_request(request)
        .await
        .context("failed to start agents request")?;

    let (sender, receiver) = mpsc::unbounded_channel();

    let request_id_for_task = request_id.clone();
    let session_id_for_task = session_id.clone();
    let sender_for_task = sender.clone();

    tokio::spawn(async move {
        let mut request_id = request_id_for_task;
        let mut session_id = session_id_for_task;
        let mut completed = false;

        while let Some(envelope) = events.recv().await {
            match envelope.r#type.as_str() {
                "response.delta" => {
                    if let Some(chunk) = envelope.payload.get("text").and_then(Value::as_str) {
                        if sender_for_task
                            .send(AiChatStreamEvent::Delta {
                                request_id: request_id.clone(),
                                session_id: session_id.clone(),
                                text: chunk.to_string(),
                            })
                            .is_err()
                        {
                            return;
                        }
                    }
                }
                "response.completed" => {
                    completed = true;
                    let text = envelope
                        .payload
                        .get("text")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let promoted_memory_uris = extract_promoted_memory_uris(&envelope.payload);

                    let _ = sender_for_task.send(AiChatStreamEvent::Completed {
                        request_id: request_id.clone(),
                        session_id: session_id.clone(),
                        text,
                        promoted_memory_uris,
                    });
                    break;
                }
                "response.error" => {
                    let message = envelope
                        .payload
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("agents service error");
                    let _ = sender_for_task.send(AiChatStreamEvent::Error {
                        request_id: request_id.clone(),
                        session_id: session_id.clone(),
                        message: message.to_string(),
                    });
                    return;
                }
                _ => {}
            }
        }

        if !completed {
            let _ = sender_for_task.send(AiChatStreamEvent::Error {
                request_id,
                session_id,
                message: "agents service disconnected before completion".to_string(),
            });
        }
    });

    Ok(receiver)
}
