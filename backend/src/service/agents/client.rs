use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use log::{error, info, warn};
use serde_json::json;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tokio::time::{Duration, sleep};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::service::agents::protocol::{
    AgentEnvelope, AgentRequest, ToolCallPayload, ToolResultPayload,
};
use crate::service::agents::tools::execute_tool_call;
use crate::service::turso::TursoClient;

type PendingMap = Arc<Mutex<HashMap<String, mpsc::UnboundedSender<AgentEnvelope>>>>;

#[derive(Clone)]
pub struct AgentsClient {
    inner: Arc<AgentsClientInner>,
}

struct AgentsClientInner {
    outbound_tx: mpsc::UnboundedSender<AgentEnvelope>,
    #[allow(dead_code)]
    pending_requests: PendingMap,
}

impl AgentsClient {
    pub async fn connect_from_env(turso: Arc<TursoClient>) -> Result<Option<Self>> {
        let ws_url = match env::var("AGENTS_SERVICE_WS_URL") {
            Ok(value) if !value.trim().is_empty() => value,
            _ => return Ok(None),
        };

        let (outbound_tx, outbound_rx) = mpsc::unbounded_channel();
        let pending_requests = Arc::new(Mutex::new(HashMap::new()));
        let client = Self {
            inner: Arc::new(AgentsClientInner {
                outbound_tx,
                pending_requests: pending_requests.clone(),
            }),
        };

        tokio::spawn(run_connection_loop(
            ws_url,
            outbound_rx,
            pending_requests,
            turso,
            client.inner.outbound_tx.clone(),
        ));

        Ok(Some(client))
    }

    #[allow(dead_code)]
    pub async fn start_request(
        &self,
        request: AgentRequest,
    ) -> Result<mpsc::UnboundedReceiver<AgentEnvelope>> {
        let (events_tx, events_rx) = mpsc::unbounded_channel();
        self.inner
            .pending_requests
            .lock()
            .await
            .insert(request.request_id.clone(), events_tx);

        let envelope = AgentEnvelope::new(
            "request.start",
            request.request_id,
            request.session_id,
            request.user_id,
            json!({ "message": request.message }),
        );

        self.inner
            .outbound_tx
            .send(envelope)
            .context("failed to enqueue agent request")?;

        Ok(events_rx)
    }

    #[allow(dead_code)]
    pub fn cancel_request(
        &self,
        request_id: impl Into<String>,
        session_id: impl Into<String>,
        user_id: impl Into<String>,
    ) -> Result<()> {
        self.inner
            .outbound_tx
            .send(AgentEnvelope::new(
                "request.cancel",
                request_id.into(),
                session_id.into(),
                user_id.into(),
                json!({}),
            ))
            .context("failed to enqueue cancellation")?;
        Ok(())
    }
}

async fn run_connection_loop(
    ws_url: String,
    mut outbound_rx: mpsc::UnboundedReceiver<AgentEnvelope>,
    pending_requests: PendingMap,
    turso: Arc<TursoClient>,
    outbound_tx: mpsc::UnboundedSender<AgentEnvelope>,
) {
    loop {
        match connect_async(ws_url.as_str()).await {
            Ok((stream, _)) => {
                info!("Connected to agents service websocket at {}", ws_url);
                let (mut write, mut read) = stream.split();
                loop {
                    tokio::select! {
                        outbound = outbound_rx.recv() => {
                            match outbound {
                                Some(envelope) => {
                                    match serde_json::to_string(&envelope) {
                                        Ok(payload) => {
                                            if let Err(err) = write.send(Message::Text(payload.into())).await {
                                                warn!("Agents websocket write failed: {}", err);
                                                notify_disconnect(&pending_requests).await;
                                                break;
                                            }
                                        }
                                        Err(err) => {
                                            error!("Failed to serialize outbound agent envelope: {}", err);
                                        }
                                    }
                                }
                                None => {
                                    info!("Agents websocket sender dropped; stopping client loop");
                                    return;
                                }
                            }
                        }
                        inbound = read.next() => {
                            match inbound {
                                Some(Ok(Message::Text(text))) => {
                                    if let Err(err) = handle_inbound_message(
                                        text.to_string(),
                                        pending_requests.clone(),
                                        turso.clone(),
                                        outbound_tx.clone(),
                                    ).await {
                                        error!("Failed to process inbound agent message: {}", err);
                                    }
                                }
                                Some(Ok(Message::Close(_))) | None => {
                                    warn!("Agents websocket connection closed");
                                    notify_disconnect(&pending_requests).await;
                                    break;
                                }
                                Some(Ok(_)) => {}
                                Some(Err(err)) => {
                                    warn!("Agents websocket read failed: {}", err);
                                    notify_disconnect(&pending_requests).await;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            Err(err) => warn!("Failed to connect to agents service websocket: {}", err),
        }

        sleep(Duration::from_secs(1)).await;
    }
}

async fn handle_inbound_message(
    raw_text: String,
    pending_requests: PendingMap,
    turso: Arc<TursoClient>,
    outbound_tx: mpsc::UnboundedSender<AgentEnvelope>,
) -> Result<()> {
    let envelope: AgentEnvelope =
        serde_json::from_str(&raw_text).context("failed to deserialize agent envelope")?;

    if envelope.r#type == "tool.call" {
        let payload: ToolCallPayload = serde_json::from_value(envelope.payload.clone())
            .context("failed to deserialize tool.call payload")?;
        let result_envelope = match execute_tool_call(
            turso,
            &envelope.user_id,
            &payload.tool_name,
            &payload.arguments,
        )
        .await
        {
            Ok(result) => AgentEnvelope::new(
                "tool.result",
                envelope.request_id,
                envelope.session_id,
                envelope.user_id,
                serde_json::to_value(ToolResultPayload {
                    tool_call_id: payload.tool_call_id,
                    tool_name: payload.tool_name,
                    ok: true,
                    result,
                    error: None,
                })?,
            ),
            Err(err) => AgentEnvelope::new(
                "tool.result",
                envelope.request_id,
                envelope.session_id,
                envelope.user_id,
                serde_json::to_value(ToolResultPayload {
                    tool_call_id: payload.tool_call_id,
                    tool_name: payload.tool_name,
                    ok: false,
                    result: json!({}),
                    error: Some(err.to_string()),
                })?,
            ),
        };

        outbound_tx
            .send(result_envelope)
            .context("failed to enqueue tool result")?;
        return Ok(());
    }

    let maybe_sender = pending_requests
        .lock()
        .await
        .get(&envelope.request_id)
        .cloned();

    if let Some(sender) = maybe_sender {
        let should_remove = matches!(envelope.r#type.as_str(), "response.completed" | "response.error");
        let _ = sender.send(envelope.clone());
        if should_remove {
            pending_requests.lock().await.remove(&envelope.request_id);
        }
    }

    Ok(())
}

async fn notify_disconnect(pending_requests: &PendingMap) {
    let mut pending = pending_requests.lock().await;
    let requests: Vec<_> = pending.drain().collect();
    drop(pending);

    for (request_id, sender) in requests {
        let _ = sender.send(AgentEnvelope::new(
            "response.error",
            request_id,
            String::new(),
            String::new(),
            json!({ "message": "agents websocket disconnected" }),
        ));
    }
}
