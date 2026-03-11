use crate::service::ai_service::model_connection::openrouter::MessageRole as OpenRouterMessageRole;
use crate::service::ai_service::model_connection::ModelSelector;
use crate::websocket::{ConnectionManager, EventType, WsMessage};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

/// AI action types for notebook
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NotebookAIAction {
    CorrectSpelling,
    ImproveWriting,
    Summarize,
    Autocomplete,
    MakeShorter,
    MakeLonger,
    SimplifyLanguage,
    FixGrammar,
}

/// Request for notebook AI operations
#[derive(Debug, Clone, Deserialize)]
pub struct NotebookAIRequest {
    pub action: NotebookAIAction,
    pub content: String,
    pub context: Option<String>,
    pub note_id: Option<String>,
    pub request_id: String,
}

/// Response for notebook AI operations
#[derive(Debug, Clone, Serialize)]
pub struct NotebookAIResponse {
    pub request_id: String,
    pub action: NotebookAIAction,
    pub result: String,
    pub original_content: String,
}

/// Streaming chunk for WebSocket
#[derive(Debug, Clone, Serialize)]
pub struct NotebookAIChunk {
    pub request_id: String,
    pub chunk: String,
    pub is_complete: bool,
}

/// Notebook AI Service
pub struct NotebookAIService {
    model_selector: Arc<Mutex<ModelSelector>>,
}

impl NotebookAIService {
    pub fn new(model_selector: Arc<Mutex<ModelSelector>>) -> Self {
        Self { model_selector }
    }

    /// Process an AI request and stream results via WebSocket
    pub async fn process_request(
        &self,
        user_id: &str,
        request: NotebookAIRequest,
        ws_manager: Arc<Mutex<ConnectionManager>>,
    ) -> Result<NotebookAIResponse> {
        let prompt = self.build_prompt(&request);
        
        // Send progress start
        self.send_ws_event(
            user_id,
            &ws_manager,
            EventType::NotebookAiProgress,
            json!({
                "request_id": request.request_id,
                "action": request.action,
                "status": "processing"
            }),
        ).await;

        // Generate AI response with streaming
        let result = self.generate_with_streaming(
            user_id,
            &request.request_id,
            prompt,
            &ws_manager,
        ).await?;

        // Send completion event
        self.send_ws_event(
            user_id,
            &ws_manager,
            EventType::NotebookAiComplete,
            json!({
                "request_id": request.request_id,
                "action": request.action,
                "result": result,
                "original_content": request.content
            }),
        ).await;

        Ok(NotebookAIResponse {
            request_id: request.request_id,
            action: request.action,
            result,
            original_content: request.content,
        })
    }

    /// Build prompt based on action type
    fn build_prompt(&self, request: &NotebookAIRequest) -> String {
        let context_hint = request.context.as_ref()
            .map(|c| format!("\n\nContext from surrounding text:\n{}", c))
            .unwrap_or_default();

        match request.action {
            NotebookAIAction::CorrectSpelling => format!(
                r#"Correct any spelling mistakes in the following text. Only fix spelling errors, do not change the meaning, style, or grammar. Return ONLY the corrected text without any explanation.

Text to correct:
{}
{}"#,
                request.content, context_hint
            ),

            NotebookAIAction::FixGrammar => format!(
                r#"Fix any grammar mistakes in the following text. Correct grammar issues while preserving the original meaning and style. Return ONLY the corrected text without any explanation.

Text to fix:
{}
{}"#,
                request.content, context_hint
            ),

            NotebookAIAction::ImproveWriting => format!(
                r#"Improve the writing quality of the following text. Make it clearer, more engaging, and better structured while preserving the original meaning. Return ONLY the improved text without any explanation.

Text to improve:
{}
{}"#,
                request.content, context_hint
            ),

            NotebookAIAction::Summarize => format!(
                r#"Provide a concise summary of the following text. Capture the key points and main ideas. Return ONLY the summary without any explanation or preamble.

Text to summarize:
{}
{}"#,
                request.content, context_hint
            ),

            NotebookAIAction::Autocomplete => format!(
                r#"Continue writing the following text naturally. Write 1-3 sentences that logically follow from the given text. Match the tone and style. Return ONLY the continuation without any explanation.

Text to continue:
{}
{}"#,
                request.content, context_hint
            ),

            NotebookAIAction::MakeShorter => format!(
                r#"Make the following text more concise while preserving the key information. Remove unnecessary words and redundancy. Return ONLY the shortened text without any explanation.

Text to shorten:
{}
{}"#,
                request.content, context_hint
            ),

            NotebookAIAction::MakeLonger => format!(
                r#"Expand the following text with more detail and elaboration while maintaining the same tone and style. Add relevant information and examples. Return ONLY the expanded text without any explanation.

Text to expand:
{}
{}"#,
                request.content, context_hint
            ),

            NotebookAIAction::SimplifyLanguage => format!(
                r#"Simplify the following text to make it easier to understand. Use simpler words and shorter sentences while preserving the meaning. Return ONLY the simplified text without any explanation.

Text to simplify:
{}
{}"#,
                request.content, context_hint
            ),
        }
    }

    /// Generate AI response with streaming via WebSocket
    async fn generate_with_streaming(
        &self,
        user_id: &str,
        request_id: &str,
        prompt: String,
        ws_manager: &Arc<Mutex<ConnectionManager>>,
    ) -> Result<String> {
        let messages = vec![
            crate::service::ai_service::model_connection::openrouter::ChatMessage {
                role: OpenRouterMessageRole::User,
                content: prompt,
            },
        ];

        let mut selector = self.model_selector.lock().await;
        
        // Try streaming first
        match selector.generate_chat_stream_with_fallback(messages.clone()).await {
            Ok(mut receiver) => {
                let mut full_response = String::new();
                
                while let Some(chunk) = receiver.recv().await {
                    full_response.push_str(&chunk);
                    
                    // Send chunk via WebSocket
                    self.send_ws_event(
                        user_id,
                        ws_manager,
                        EventType::NotebookAiChunk,
                        json!({
                            "request_id": request_id,
                            "chunk": chunk,
                            "is_complete": false
                        }),
                    ).await;
                }

                // Send final chunk marker
                self.send_ws_event(
                    user_id,
                    ws_manager,
                    EventType::NotebookAiChunk,
                    json!({
                        "request_id": request_id,
                        "chunk": "",
                        "is_complete": true
                    }),
                ).await;

                Ok(full_response)
            }
            Err(stream_err) => {
                log::warn!("Streaming failed, falling back to non-streaming: {}", stream_err);
                
                // Fallback to non-streaming
                let response = selector.generate_chat_with_fallback(messages).await?;
                
                // Send complete response as single chunk
                self.send_ws_event(
                    user_id,
                    ws_manager,
                    EventType::NotebookAiChunk,
                    json!({
                        "request_id": request_id,
                        "chunk": response,
                        "is_complete": true
                    }),
                ).await;

                Ok(response)
            }
        }
    }

    /// Send WebSocket event to user
    async fn send_ws_event(
        &self,
        user_id: &str,
        ws_manager: &Arc<Mutex<ConnectionManager>>,
        event: EventType,
        data: serde_json::Value,
    ) {
        let manager = ws_manager.lock().await;
        manager.broadcast_to_user(user_id, WsMessage::new(event, data));
    }
}
