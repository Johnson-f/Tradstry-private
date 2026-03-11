#![allow(dead_code)]

use crate::models::ai::chat::{
    ChatMessage, ChatRequest, ChatResponse, ChatSession, ChatSessionDetailsResponse,
    ChatSessionListResponse, ChatSessionSummary, ContextSource, MessageRole,
};
use crate::models::ai::chat_templates::{ChatPromptConfig, ContextFormatter};
use crate::service::ai_service::model_connection::ModelSelector;
use crate::service::ai_service::model_connection::openrouter::{
    ChatMessage as OpenRouterChatMessage, MessageRole as OpenRouterMessageRole, OpenRouterClient,
    ToolResult,
};
use crate::service::ai_service::vector_service::client::VoyagerClient;
use crate::service::ai_service::vector_service::qdrant::QdrantDocumentClient;
use crate::service::ai_service::vector_service::vectors::ChatVectorization;
use crate::service::tool_engine::{ToolEngine, get_tool_definitions};
use crate::turso::client::TursoClient;
use anyhow::{Context, Result};
use chrono::Utc;
use libsql::{Connection, params};
use serde_json;
use std::sync::Arc;
use uuid::Uuid;

/// AI Chat Service for handling chat functionality
#[derive(Clone)]
pub struct AIChatService {
    chat_vector_service: Arc<ChatVectorization>,
    qdrant_client: Arc<QdrantDocumentClient>,
    model_selector: Arc<tokio::sync::Mutex<ModelSelector>>,
    openrouter_client: Arc<OpenRouterClient>,
    turso_client: Arc<TursoClient>,
    voyager_client: Arc<VoyagerClient>,
    tool_engine: Arc<ToolEngine>,
    max_context_vectors: usize,
    prompt_config: ChatPromptConfig,
}

impl AIChatService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        chat_vector_service: Arc<ChatVectorization>,
        qdrant_client: Arc<QdrantDocumentClient>,
        model_selector: Arc<tokio::sync::Mutex<ModelSelector>>,
        openrouter_client: Arc<OpenRouterClient>,
        turso_client: Arc<TursoClient>,
        voyager_client: Arc<VoyagerClient>,
        tool_engine: Arc<ToolEngine>,
        max_context_vectors: usize,
    ) -> Self {
        Self {
            chat_vector_service,
            qdrant_client,
            model_selector,
            openrouter_client,
            turso_client,
            voyager_client,
            tool_engine,
            max_context_vectors,
            prompt_config: ChatPromptConfig::default(),
        }
    }

    /// Configure prompt templates dynamically
    pub fn configure_prompts(&mut self, config: ChatPromptConfig) {
        self.prompt_config = config;
    }

    /// Get current prompt configuration
    pub fn get_prompt_config(&self) -> &ChatPromptConfig {
        &self.prompt_config
    }

    /// Build enhanced system prompt based on query type and context
    fn build_enhanced_system_prompt(
        &self,
        query: &str,
        context_sources: &[ContextSource],
    ) -> String {
        // Detect query type and get appropriate template
        let template = self.prompt_config.detect_query_type(query);

        // Build base system prompt
        let mut system_prompt = ContextFormatter::build_system_prompt(template);

        // Add context if available
        if !context_sources.is_empty() {
            let formatted_context = ContextFormatter::format_context_sources(
                context_sources,
                self.prompt_config.context_max_length,
                self.prompt_config.include_relevance_scores,
            );

            system_prompt.push_str(&format!("\n\n{}", formatted_context));
        }

        system_prompt
    }

    /// Build enhanced messages with system prompt
    fn build_enhanced_messages(
        &self,
        messages: &[ChatMessage],
        query: &str,
        context_sources: &[ContextSource],
    ) -> Vec<crate::service::ai_service::model_connection::openrouter::ChatMessage> {
        let mut openrouter_messages = Vec::new();

        // Add system prompt if this is the first user message or if we have context
        if messages.len() == 1 || !context_sources.is_empty() {
            let system_prompt = self.build_enhanced_system_prompt(query, context_sources);
            openrouter_messages.push(
                crate::service::ai_service::model_connection::openrouter::ChatMessage {
                    role: OpenRouterMessageRole::System,
                    content: system_prompt,
                },
            );
        }

        // Convert existing messages, filtering out any existing system messages to prevent duplicates
        for msg in messages {
            // Skip system messages since we're adding our own enhanced system prompt
            if matches!(msg.role, MessageRole::System) {
                continue;
            }

            openrouter_messages.push(
                crate::service::ai_service::model_connection::openrouter::ChatMessage {
                    role: match msg.role {
                        MessageRole::User => OpenRouterMessageRole::User,
                        MessageRole::Assistant => OpenRouterMessageRole::Assistant,
                        MessageRole::System => OpenRouterMessageRole::System, // This won't be reached due to continue above
                    },
                    content: msg.content.clone(),
                },
            );
        }

        openrouter_messages
    }

    /// Generate a chat response with context retrieval
    pub async fn generate_response(
        &self,
        user_id: &str,
        request: ChatRequest,
        conn: &Connection,
    ) -> Result<ChatResponse> {
        let start_time = std::time::Instant::now();
        let message_preview = request.message.chars().take(100).collect::<String>();

        log::info!(
            "Starting response generation for user={}, session_id={:?}, message_preview='{}'",
            user_id,
            request.session_id,
            message_preview
        );

        // Get or create session
        let session_start = std::time::Instant::now();
        let session = if let Some(session_id) = request.session_id {
            self.get_session(conn, &session_id, user_id).await?
        } else {
            // Create session with a temporary title, will be updated after first message
            self.create_session(conn, user_id, Some("New Chat".to_string()))
                .await?
        };
        let session_time = session_start.elapsed().as_millis();

        log::info!(
            "Session retrieved/created [{}ms] - session_id={}, user={}",
            session_time,
            session.id,
            user_id
        );

        // Retrieve relevant context using vector similarity search with fallback
        let context_start = std::time::Instant::now();
        let context_sources = if request.include_context.unwrap_or(true) {
            match self
                .retrieve_context(
                    user_id,
                    &request.message,
                    request
                        .max_context_vectors
                        .unwrap_or(self.max_context_vectors),
                )
                .await
            {
                Ok(sources) => {
                    let context_time = context_start.elapsed().as_millis();
                    log::info!(
                        "Context retrieved [{}ms] - sources={}, user={}",
                        context_time,
                        sources.len(),
                        user_id
                    );
                    sources
                }
                Err(e) => {
                    let context_time = context_start.elapsed().as_millis();
                    log::warn!(
                        "Context retrieval failed [{}ms] - error={}, user={}. Continuing without context.",
                        context_time,
                        e,
                        user_id
                    );
                    log::debug!("Full context retrieval error details: {:?}", e);
                    Vec::new()
                }
            }
        } else {
            log::info!(
                "Context retrieval skipped - include_context=false, user={}",
                user_id
            );
            Vec::new()
        };

        // Build conversation history
        let history_start = std::time::Instant::now();
        let mut messages = self.get_session_messages(conn, &session.id).await?;
        let history_time = history_start.elapsed().as_millis();

        log::info!(
            "Message history retrieved [{}ms] - messages={}, session={}",
            history_time,
            messages.len(),
            session.id
        );

        // Add user message
        let user_message = ChatMessage::new(
            session.id.clone(),
            MessageRole::User,
            request.message.clone(),
        );
        messages.push(user_message.clone());

        // Convert to OpenRouter format with enhanced prompts
        let prompt_start = std::time::Instant::now();
        let openrouter_messages =
            self.build_enhanced_messages(&messages, &request.message, &context_sources);
        let prompt_time = prompt_start.elapsed().as_millis();

        log::info!(
            "Enhanced messages built [{}ms] - context_sources={}, history_messages={}, user={}",
            prompt_time,
            context_sources.len(),
            messages.len(),
            user_id
        );

        // Generate AI response using ModelSelector with fallback
        let ai_start = std::time::Instant::now();
        let ai_response = {
            let mut selector = self.model_selector.lock().await;
            selector
                .generate_chat_with_fallback(openrouter_messages)
                .await?
        };
        let ai_time = ai_start.elapsed().as_millis();

        log::info!(
            "AI response generated [{}ms] - response_length={}, user={}",
            ai_time,
            ai_response.len(),
            user_id
        );

        // Create assistant message
        let assistant_message = ChatMessage::new(
            session.id.clone(),
            MessageRole::Assistant,
            ai_response.clone(),
        )
        .with_context(
            context_sources
                .iter()
                .map(|s| s.vector_id.clone())
                .collect(),
        );

        // Store messages in database
        let storage_start = std::time::Instant::now();
        self.store_message(conn, &user_message).await?;
        self.store_message(conn, &assistant_message).await?;

        // Vectorize Q&A pair after both messages are stored
        self.vectorize_qa_pair(
            user_id,
            &session.id,
            &user_message.content,
            &assistant_message.content,
        )
        .await
        .ok();
        let storage_time = storage_start.elapsed().as_millis();

        log::info!(
            "Messages stored and vectorized [{}ms] - user_msg={}, ai_msg={}, user={}",
            storage_time,
            user_message.id,
            assistant_message.id,
            user_id
        );

        // Update session
        self.update_session_last_message(conn, &session.id).await?;

        // If this is a new session (title is "New Chat"), update it with a summary of the first message
        if session.title.as_ref().is_some_and(|t| t == "New Chat")
            && let Err(e) = self
                .update_session_title_from_message(conn, &session.id, user_id, &request.message)
                .await
        {
            log::warn!("Failed to update session title: {}", e);
        }

        let processing_time = start_time.elapsed().as_millis() as u64;

        log::info!(
            "Response generation completed [{}ms] - session={}ms, context={}ms, ai={}ms, storage={}ms, user={}",
            processing_time,
            session_time,
            context_start.elapsed().as_millis(),
            ai_time,
            storage_time,
            user_id
        );

        Ok(ChatResponse {
            message: ai_response,
            session_id: session.id,
            message_id: assistant_message.id,
            sources: context_sources,
            token_count: None, // Would be populated from Gemini response
            processing_time_ms: processing_time,
        })
    }

    /// Generate a chat response with tool calling (agentic loop)
    pub async fn generate_response_with_tools(
        &self,
        user_id: &str,
        request: ChatRequest,
        conn: &Connection,
    ) -> Result<ChatResponse> {
        let start_time = std::time::Instant::now();
        let message_preview = request.message.chars().take(100).collect::<String>();

        log::info!(
            "Starting tool-enabled response generation for user={}, session_id={:?}, message_preview='{}'",
            user_id,
            request.session_id,
            message_preview
        );

        // Get or create session
        let session = if let Some(session_id) = request.session_id {
            self.get_session(conn, &session_id, user_id).await?
        } else {
            self.create_session(conn, user_id, Some("New Chat".to_string()))
                .await?
        };

        // Retrieve relevant context
        let context_sources = if request.include_context.unwrap_or(true) {
            self.retrieve_context(
                user_id,
                &request.message,
                request
                    .max_context_vectors
                    .unwrap_or(self.max_context_vectors),
            )
            .await
            .unwrap_or_default()
        } else {
            Vec::new()
        };

        // Build conversation history
        let mut messages = self.get_session_messages(conn, &session.id).await?;
        let user_message = ChatMessage::new(
            session.id.clone(),
            MessageRole::User,
            request.message.clone(),
        );
        messages.push(user_message.clone());

        // Convert to OpenRouter format
        let openrouter_messages =
            self.build_enhanced_messages(&messages, &request.message, &context_sources);

        // Get tool definitions
        let tools = get_tool_definitions();

        // Agentic loop - max 5 iterations
        let max_iterations = 5;
        let mut conversation_messages: Vec<OpenRouterChatMessage> = openrouter_messages;
        let mut final_response = String::new();

        for iteration in 0..max_iterations {
            log::info!("Tool loop iteration {}/{}", iteration + 1, max_iterations);

            // Call LLM with tools
            let response = self
                .openrouter_client
                .generate_chat_with_tools(conversation_messages.clone(), tools.clone(), None)
                .await?;

            if let Some(choice) = response.choices.first() {
                // Check if model wants to call tools
                if let Some(tool_calls) = &choice.message.tool_calls {
                    log::info!("Model requested {} tool calls", tool_calls.len());

                    // Execute all tool calls in parallel
                    let mut tool_results = Vec::new();
                    for tool_call in tool_calls {
                        match self.tool_engine.execute(tool_call).await {
                            Ok(result) => {
                                let result_json = serde_json::to_string(&result)?;
                                tool_results.push(ToolResult {
                                    tool_call_id: tool_call.id.clone(),
                                    role: "tool".to_string(),
                                    content: result_json,
                                });
                                log::info!(
                                    "Tool '{}' executed successfully",
                                    tool_call.function.name
                                );
                            }
                            Err(e) => {
                                log::error!(
                                    "Tool '{}' execution failed: {}",
                                    tool_call.function.name,
                                    e
                                );
                                tool_results.push(ToolResult {
                                    tool_call_id: tool_call.id.clone(),
                                    role: "tool".to_string(),
                                    content: format!("Error: {}", e),
                                });
                            }
                        }
                    }

                    // Add assistant message with tool calls to conversation
                    // Note: We need to include the tool_calls in the assistant message
                    // For now, we'll add it as a regular assistant message
                    // The tool results will be added as separate messages
                    conversation_messages.push(OpenRouterChatMessage {
                        role: OpenRouterMessageRole::Assistant,
                        content: choice.message.content.clone().unwrap_or_default(),
                    });

                    // Add tool results to conversation
                    // Note: OpenRouter expects tool messages with role "tool" and tool_call_id
                    // Since we're using ChatMessage which converts to Message, we'll need to handle this
                    // in the conversion. For now, we'll add them as system messages with the tool result
                    for tool_result in tool_results {
                        // Format tool result as JSON that the model can understand
                        conversation_messages.push(OpenRouterChatMessage {
                            role: OpenRouterMessageRole::System,
                            content: format!(
                                "Tool result (call_id: {}): {}",
                                tool_result.tool_call_id, tool_result.content
                            ),
                        });
                    }

                    // Continue loop to get final response
                    continue;
                } else {
                    // No tool calls - we have the final response
                    final_response = choice.message.content.clone().unwrap_or_default();
                    log::info!("Final response received (no more tool calls)");
                    break;
                }
            } else {
                return Err(anyhow::anyhow!("No response from model"));
            }
        }

        if final_response.is_empty() {
            return Err(anyhow::anyhow!(
                "Max iterations reached without final response"
            ));
        }

        // Create assistant message
        let assistant_message = ChatMessage::new(
            session.id.clone(),
            MessageRole::Assistant,
            final_response.clone(),
        )
        .with_context(
            context_sources
                .iter()
                .map(|s| s.vector_id.clone())
                .collect(),
        );

        // Store messages in database
        self.store_message(conn, &user_message).await?;
        self.store_message(conn, &assistant_message).await?;

        // Vectorize Q&A pair
        self.vectorize_qa_pair(
            user_id,
            &session.id,
            &user_message.content,
            &assistant_message.content,
        )
        .await
        .ok();

        // Update session
        self.update_session_last_message(conn, &session.id).await?;

        // Update session title if needed
        if session.title.as_ref().is_some_and(|t| t == "New Chat") {
            self.update_session_title_from_message(conn, &session.id, user_id, &request.message)
                .await
                .ok();
        }

        let processing_time = start_time.elapsed().as_millis() as u64;
        log::info!(
            "Tool-enabled response generation completed [{}ms] - user={}, session={}",
            processing_time,
            user_id,
            session.id
        );

        Ok(ChatResponse {
            message: final_response,
            session_id: session.id,
            message_id: assistant_message.id,
            sources: context_sources,
            token_count: None,
            processing_time_ms: processing_time,
        })
    }

    /// Generate a streaming chat response with tool support
    /// This method first executes any required tool calls, then streams the final response
    pub async fn generate_streaming_response_with_tools(
        &self,
        user_id: &str,
        request: ChatRequest,
        conn: &Connection,
    ) -> Result<(tokio::sync::mpsc::Receiver<String>, String, String)> {
        let start_time = std::time::Instant::now();
        let message_preview = request.message.chars().take(100).collect::<String>();

        log::info!(
            "Starting streaming response with tools for user={}, session_id={:?}, message_preview='{}'",
            user_id,
            request.session_id,
            message_preview
        );

        // Get or create session
        let session = if let Some(session_id) = &request.session_id {
            self.get_session(conn, session_id, user_id).await?
        } else {
            self.create_session(conn, user_id, Some("New Chat".to_string()))
                .await?
        };

        // Retrieve relevant context
        let context_sources = if request.include_context.unwrap_or(true) {
            self.retrieve_context(
                user_id,
                &request.message,
                request
                    .max_context_vectors
                    .unwrap_or(self.max_context_vectors),
            )
            .await
            .unwrap_or_default()
        } else {
            Vec::new()
        };

        // Build conversation history
        let mut messages = self.get_session_messages(conn, &session.id).await?;
        let user_message = ChatMessage::new(
            session.id.clone(),
            MessageRole::User,
            request.message.clone(),
        );
        messages.push(user_message.clone());

        // Convert to OpenRouter format
        let openrouter_messages =
            self.build_enhanced_messages(&messages, &request.message, &context_sources);

        // Get tool definitions
        let tools = get_tool_definitions();

        // Agentic loop - execute tools first (non-streaming)
        let max_iterations = 5;
        let mut conversation_messages: Vec<OpenRouterChatMessage> = openrouter_messages;

        for iteration in 0..max_iterations {
            log::info!(
                "Tool loop iteration {}/{} for user={}",
                iteration + 1,
                max_iterations,
                user_id
            );

            // Call LLM with tools (non-streaming to check for tool calls)
            let response = self
                .openrouter_client
                .generate_chat_with_tools(conversation_messages.clone(), tools.clone(), None)
                .await?;

            if let Some(choice) = response.choices.first() {
                // Check if model wants to call tools
                if let Some(tool_calls) = &choice.message.tool_calls {
                    if tool_calls.is_empty() {
                        log::info!("No tool calls requested, proceeding to stream final response");
                        break;
                    }

                    log::info!("Model requested {} tool calls", tool_calls.len());

                    // Execute all tool calls
                    let mut tool_results = Vec::new();
                    for tool_call in tool_calls {
                        log::info!(
                            "ToolEngine: Executing tool '{}' with args: {}",
                            tool_call.function.name,
                            tool_call.function.arguments
                        );

                        match self.tool_engine.execute(tool_call).await {
                            Ok(result) => {
                                let result_json = serde_json::to_string(&result)?;
                                log::info!(
                                    "Tool '{}' executed successfully, result_length={}",
                                    tool_call.function.name,
                                    result_json.len()
                                );
                                tool_results.push((
                                    tool_call.id.clone(),
                                    tool_call.function.name.clone(),
                                    result_json,
                                ));
                            }
                            Err(e) => {
                                log::error!(
                                    "Tool '{}' execution failed: {}",
                                    tool_call.function.name,
                                    e
                                );
                                tool_results.push((
                                    tool_call.id.clone(),
                                    tool_call.function.name.clone(),
                                    format!("Error: {}", e),
                                ));
                            }
                        }
                    }

                    // Add assistant message to conversation
                    conversation_messages.push(OpenRouterChatMessage {
                        role: OpenRouterMessageRole::Assistant,
                        content: choice.message.content.clone().unwrap_or_default(),
                    });

                    // Add tool results to conversation as system messages
                    for (call_id, tool_name, result) in tool_results {
                        conversation_messages.push(OpenRouterChatMessage {
                            role: OpenRouterMessageRole::System,
                            content: format!(
                                "Tool '{}' result (call_id: {}): {}",
                                tool_name, call_id, result
                            ),
                        });
                    }

                    // Continue loop to get final response
                    continue;
                } else {
                    // No tool calls - ready to stream final response
                    log::info!("No tool calls in response, proceeding to stream");
                    break;
                }
            }
        }

        log::info!(
            "Tool execution complete, starting streaming response for user={}",
            user_id
        );

        // Now stream the final response using the conversation with tool results
        let stream_receiver = {
            let mut selector = self.model_selector.lock().await;
            selector
                .generate_chat_stream_with_fallback(conversation_messages)
                .await?
        };

        // Store user message
        self.store_message(conn, &user_message).await?;

        // Create assistant message placeholder
        let assistant_message_id = Uuid::new_v4().to_string();
        let assistant_message = ChatMessage {
            id: assistant_message_id.clone(),
            session_id: session.id.clone(),
            role: MessageRole::Assistant,
            content: String::new(),
            timestamp: Utc::now(),
            context_vectors: Some(
                context_sources
                    .iter()
                    .map(|s| s.vector_id.clone())
                    .collect(),
            ),
            token_count: None,
        };

        self.store_message(conn, &assistant_message).await?;
        self.update_session_last_message(conn, &session.id).await?;

        // Update session title if needed
        if session.title.as_ref().is_some_and(|t| t == "New Chat") {
            self.update_session_title_from_message(conn, &session.id, user_id, &request.message)
                .await
                .ok();
        }

        // Create channel for frontend
        let (frontend_tx, frontend_rx) = tokio::sync::mpsc::channel(100);

        // Spawn task to accumulate and save content
        let service = self.clone();
        let msg_id = assistant_message_id.clone();
        let user_id_clone = user_id.to_string();
        let session_id_clone = session.id.clone();
        let user_question = request.message.clone();
        let mut stream_receiver = stream_receiver;

        tokio::spawn(async move {
            let mut accumulated = String::new();
            let mut token_count = 0;

            log::info!(
                "Starting token accumulation for message={}, user={}",
                msg_id,
                user_id_clone
            );

            while let Some(token) = stream_receiver.recv().await {
                accumulated.push_str(&token);
                token_count += 1;
                frontend_tx.send(token).await.ok();
            }

            log::info!(
                "Token accumulation completed - message={}, total_tokens={}, final_length={}, user={}",
                msg_id,
                token_count,
                accumulated.len(),
                user_id_clone
            );

            // Update database with final content
            if let Ok(Some(conn)) = service
                .turso_client
                .get_user_database_connection(&user_id_clone)
                .await
            {
                if let Err(e) = service
                    .update_message_content(&conn, &msg_id, accumulated.clone())
                    .await
                {
                    log::error!(
                        "Failed to update message content for message {}: {}",
                        msg_id,
                        e
                    );
                }

                // Vectorize the Q&A pair
                if let Err(e) = service
                    .chat_vector_service
                    .vectorize_qa_pair(
                        &user_id_clone,
                        &session_id_clone,
                        &user_question,
                        &accumulated,
                    )
                    .await
                {
                    log::error!("Failed to vectorize Q&A pair for message {}: {}", msg_id, e);
                }
            }
        });

        let total_time = start_time.elapsed().as_millis();
        log::info!(
            "Streaming with tools setup completed [{}ms] - user={}, session={}",
            total_time,
            user_id,
            session.id
        );

        Ok((frontend_rx, session.id, assistant_message_id))
    }

    /// Generate a streaming chat response (without tools)
    pub async fn generate_streaming_response(
        &self,
        user_id: &str,
        request: ChatRequest,
        conn: &Connection,
    ) -> Result<(tokio::sync::mpsc::Receiver<String>, String, String)> {
        let start_time = std::time::Instant::now();
        let message_preview = request.message.chars().take(100).collect::<String>();

        log::info!(
            "Starting streaming response generation for user={}, session_id={:?}, message_preview='{}'",
            user_id,
            request.session_id,
            message_preview
        );

        // Get or create session
        let session_start = std::time::Instant::now();
        let session = if let Some(session_id) = request.session_id {
            self.get_session(conn, &session_id, user_id).await?
        } else {
            // Create session with a temporary title, will be updated after first message
            self.create_session(conn, user_id, Some("New Chat".to_string()))
                .await?
        };
        let session_time = session_start.elapsed().as_millis();

        log::info!(
            "Session retrieved/created [{}ms] - session_id={}, user={}",
            session_time,
            session.id,
            user_id
        );

        // Retrieve relevant context with fallback
        let context_start = std::time::Instant::now();
        let context_sources = if request.include_context.unwrap_or(true) {
            match self
                .retrieve_context(
                    user_id,
                    &request.message,
                    request
                        .max_context_vectors
                        .unwrap_or(self.max_context_vectors),
                )
                .await
            {
                Ok(sources) => {
                    let context_time = context_start.elapsed().as_millis();
                    log::info!(
                        "Context retrieved [{}ms] - sources={}, user={}",
                        context_time,
                        sources.len(),
                        user_id
                    );
                    sources
                }
                Err(e) => {
                    let context_time = context_start.elapsed().as_millis();
                    log::warn!(
                        "Context retrieval failed [{}ms] - error={}, user={}. Continuing without context.",
                        context_time,
                        e,
                        user_id
                    );
                    log::debug!("Full context retrieval error details: {:?}", e);
                    Vec::new()
                }
            }
        } else {
            log::info!(
                "Context retrieval skipped - include_context=false, user={}",
                user_id
            );
            Vec::new()
        };

        // Build conversation history
        let history_start = std::time::Instant::now();
        let mut messages = self.get_session_messages(conn, &session.id).await?;
        let history_time = history_start.elapsed().as_millis();

        log::info!(
            "Message history retrieved [{}ms] - messages={}, session={}",
            history_time,
            messages.len(),
            session.id
        );

        // Add user message
        let user_message = ChatMessage::new(
            session.id.clone(),
            MessageRole::User,
            request.message.clone(),
        );
        messages.push(user_message.clone());

        // Convert to OpenRouter format with enhanced prompts
        let prompt_start = std::time::Instant::now();
        let openrouter_messages =
            self.build_enhanced_messages(&messages, &request.message, &context_sources);
        let prompt_time = prompt_start.elapsed().as_millis();

        log::info!(
            "Enhanced messages built [{}ms] - context_sources={}, history_messages={}, user={}",
            prompt_time,
            context_sources.len(),
            messages.len(),
            user_id
        );

        // Generate streaming AI response using ModelSelector with fallback
        let stream_start = std::time::Instant::now();
        let mut stream_receiver = {
            let mut selector = self.model_selector.lock().await;
            selector
                .generate_chat_stream_with_fallback(openrouter_messages)
                .await?
        };
        let stream_init_time = stream_start.elapsed().as_millis();

        log::info!(
            "Streaming initiated [{}ms] - user={}",
            stream_init_time,
            user_id
        );

        // Store user message
        let user_msg_start = std::time::Instant::now();
        self.store_message(conn, &user_message).await?;
        let user_msg_time = user_msg_start.elapsed().as_millis();

        log::info!(
            "User message stored [{}ms] - message_id={}, user={}",
            user_msg_time,
            user_message.id,
            user_id
        );

        // Create assistant message placeholder
        let assistant_message_id = Uuid::new_v4().to_string();
        let assistant_message = ChatMessage {
            id: assistant_message_id.clone(),
            session_id: session.id.clone(),
            role: MessageRole::Assistant,
            content: String::new(), // Will be updated as stream progresses
            timestamp: Utc::now(),
            context_vectors: Some(
                context_sources
                    .iter()
                    .map(|s| s.vector_id.clone())
                    .collect(),
            ),
            token_count: None,
        };

        // Store initial assistant message
        self.store_message(conn, &assistant_message).await?;

        // Update session
        self.update_session_last_message(conn, &session.id).await?;

        // If this is a new session (title is "New Chat"), update it with a summary of the first message
        if session.title.as_ref().is_some_and(|t| t == "New Chat")
            && let Err(e) = self
                .update_session_title_from_message(conn, &session.id, user_id, &request.message)
                .await
        {
            log::warn!("Failed to update session title: {}", e);
        }

        // Create channel for frontend
        let (frontend_tx, frontend_rx) = tokio::sync::mpsc::channel(100);

        log::info!(
            "Streaming setup completed - assistant_message_id={}, user={}",
            assistant_message_id,
            user_id
        );

        // Spawn task to accumulate and save content
        let service = self.clone(); // Make service cloneable
        let msg_id = assistant_message_id.clone();
        let user_id_clone = user_id.to_string();
        let session_id_clone = session.id.clone();
        let user_question = request.message.clone();
        tokio::spawn(async move {
            let mut accumulated = String::new();
            let mut token_count = 0;

            log::info!(
                "Starting token accumulation for message={}, user={}",
                msg_id,
                user_id_clone
            );

            while let Some(token) = stream_receiver.recv().await {
                accumulated.push_str(&token);
                token_count += 1;
                frontend_tx.send(token).await.ok();

                // Log progress every 10 tokens
                if token_count % 10 == 0 {
                    log::debug!(
                        "Token accumulation progress - message={}, tokens={}, length={}, user={}",
                        msg_id,
                        token_count,
                        accumulated.len(),
                        user_id_clone
                    );
                }
            }

            log::info!(
                "Token accumulation completed - message={}, total_tokens={}, final_length={}, user={}",
                msg_id,
                token_count,
                accumulated.len(),
                user_id_clone
            );

            // Update database with final content
            if let Ok(Some(conn)) = service
                .turso_client
                .get_user_database_connection(&user_id_clone)
                .await
            {
                let update_start = std::time::Instant::now();
                if let Err(e) = service
                    .update_message_content(&conn, &msg_id, accumulated.clone())
                    .await
                {
                    log::error!(
                        "Failed to update message content for message {}: {}",
                        msg_id,
                        e
                    );
                } else {
                    let update_time = update_start.elapsed().as_millis();
                    log::info!(
                        "Successfully updated message content [{}ms] - message={}, user={}",
                        update_time,
                        msg_id,
                        user_id_clone
                    );
                }

                // Vectorize the Q&A pair after streaming completes
                let vectorize_start = std::time::Instant::now();
                match service
                    .chat_vector_service
                    .vectorize_qa_pair(
                        &user_id_clone,
                        &session_id_clone,
                        &user_question,
                        &accumulated,
                    )
                    .await
                {
                    Ok(_) => {
                        let vectorize_time = vectorize_start.elapsed().as_millis();
                        log::info!(
                            "Successfully vectorized Q&A pair [{}ms] - message={}, user={}, session={}, question_length={}, answer_length={}",
                            vectorize_time,
                            msg_id,
                            user_id_clone,
                            session_id_clone,
                            user_question.len(),
                            accumulated.len()
                        );
                    }
                    Err(e) => {
                        let vectorize_time = vectorize_start.elapsed().as_millis();
                        log::error!(
                            "Failed to vectorize Q&A pair for message {} [{}ms] - user={}, session={}, question_length={}, answer_length={}, error={}, error_debug={:?}",
                            msg_id,
                            vectorize_time,
                            user_id_clone,
                            session_id_clone,
                            user_question.len(),
                            accumulated.len(),
                            e,
                            e
                        );
                    }
                }
            } else {
                log::error!(
                    "Failed to get database connection for user {} to save message {}",
                    user_id_clone,
                    msg_id
                );
            }
        });

        let total_time = start_time.elapsed().as_millis();
        log::info!(
            "Streaming response setup completed [{}ms] - session={}ms, context={}ms, stream_init={}ms, user={}",
            total_time,
            session_time,
            context_start.elapsed().as_millis(),
            stream_init_time,
            user_id
        );

        Ok((frontend_rx, session.id, assistant_message_id))
    }

    /// Vectorize a Q&A pair and store in Qdrant
    /// Helper method that calls ChatVectorization
    async fn vectorize_qa_pair(
        &self,
        user_id: &str,
        session_id: &str,
        question: &str,
        answer: &str,
    ) -> Result<()> {
        self.chat_vector_service
            .vectorize_qa_pair(user_id, session_id, question, answer)
            .await
    }

    /// Retrieve relevant context using Qdrant semantic search
    /// Searches both trades and chat history
    async fn retrieve_context(
        &self,
        user_id: &str,
        query: &str,
        max_vectors: usize,
    ) -> Result<Vec<ContextSource>> {
        let start_time = std::time::Instant::now();
        let query_preview = query.chars().take(100).collect::<String>();

        log::info!(
            "Starting context retrieval for user={}, query_preview='{}', max_vectors={}",
            user_id,
            query_preview,
            max_vectors
        );

        // Generate query embedding
        let search_start = std::time::Instant::now();
        log::debug!(
            "Generating query embedding - user={}, query='{}'",
            user_id,
            query_preview
        );

        let query_embedding = self
            .voyager_client
            .embed_text(query)
            .await
            .context("Failed to generate query embedding")?;

        log::debug!(
            "Query embedding generated - user={}, embedding_dim={}",
            user_id,
            query_embedding.len()
        );

        // Search Qdrant (both trades and chats, no type filter)
        let search_results = self
            .qdrant_client
            .search_by_embedding(user_id, &query_embedding, max_vectors, None)
            .await
            .context("Failed to perform semantic search in Qdrant")?;

        let search_time = search_start.elapsed().as_millis();

        log::info!(
            "Semantic search completed [{}ms] - found {} matches, user={}",
            search_time,
            search_results.len(),
            user_id
        );

        // Log top similarity scores and data types
        if !search_results.is_empty() {
            let top_scores: Vec<String> = search_results
                .iter()
                .take(5)
                .map(|r| format!("{:.3}", r.score))
                .collect();
            let data_types: Vec<String> = search_results
                .iter()
                .take(5)
                .map(|r| r.r#type.clone().unwrap_or_else(|| "unknown".to_string()))
                .collect();

            log::info!(
                "Top similarity scores: [{}], data_types: [{}], user={}",
                top_scores.join(", "),
                data_types.join(", "),
                user_id
            );
        } else {
            log::warn!(
                "No search results found - user={}, query='{}'",
                user_id,
                query_preview
            );
        }

        // Convert search results to context sources
        let context_sources: Vec<ContextSource> = search_results
            .iter()
            .map(ContextSource::from_search_result)
            .collect();

        let total_time = start_time.elapsed().as_millis();
        log::info!(
            "Context retrieval completed [{}ms] - search={}ms, sources={}, user={}",
            total_time,
            search_time,
            context_sources.len(),
            user_id
        );

        Ok(context_sources)
    }

    /// Create a new chat session
    pub async fn create_session(
        &self,
        conn: &Connection,
        user_id: &str,
        title: Option<String>,
    ) -> Result<ChatSession> {
        let session = ChatSession::new(user_id.to_string(), title);

        conn.execute(
            "INSERT INTO chat_sessions (id, user_id, title, created_at, updated_at, message_count, last_message_at) 
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                session.id.clone(),
                session.user_id.clone(),
                session.title.clone(),
                session.created_at.to_rfc3339(),
                session.updated_at.to_rfc3339(),
                session.message_count,
                session.last_message_at.map(|d| d.to_rfc3339())
            ],
        ).await?;

        Ok(session)
    }

    /// Get a chat session by ID
    pub async fn get_session(
        &self,
        conn: &Connection,
        session_id: &str,
        user_id: &str,
    ) -> Result<ChatSession> {
        let stmt = conn
            .prepare(
                "SELECT id, user_id, title, created_at, updated_at, message_count, last_message_at 
             FROM chat_sessions WHERE id = ? AND user_id = ?",
            )
            .await?;

        let mut rows = stmt.query([session_id, user_id]).await?;

        if let Some(row) = rows.next().await? {
            Ok(ChatSession {
                id: row.get(0)?,
                user_id: row.get(1)?,
                title: row.get(2)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String>(3)?)?
                    .with_timezone(&Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String>(4)?)?
                    .with_timezone(&Utc),
                message_count: row.get(5)?,
                last_message_at: row.get::<Option<String>>(6)?.map(|s| {
                    chrono::DateTime::parse_from_rfc3339(&s)
                        .unwrap()
                        .with_timezone(&Utc)
                }),
            })
        } else {
            Err(anyhow::anyhow!("Session not found"))
        }
    }

    /// Get user's chat sessions
    pub async fn get_user_sessions(
        &self,
        conn: &Connection,
        user_id: &str,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<ChatSessionListResponse> {
        let limit = limit.unwrap_or(20);
        let offset = offset.unwrap_or(0);

        // Get total count
        let mut count_stmt = conn
            .prepare("SELECT COUNT(*) FROM chat_sessions WHERE user_id = ?")
            .await?;
        let row = count_stmt.query_row([user_id]).await?;
        let total_count: u32 = row.get(0)?;

        // Get sessions
        let stmt = conn
            .prepare(
                "SELECT id, user_id, title, created_at, updated_at, message_count, last_message_at 
             FROM chat_sessions WHERE user_id = ? 
             ORDER BY updated_at DESC LIMIT ? OFFSET ?",
            )
            .await?;

        let mut rows = stmt
            .query([user_id, &limit.to_string(), &offset.to_string()])
            .await?;

        let mut sessions = Vec::new();
        while let Some(row) = rows.next().await? {
            let session = ChatSession {
                id: row.get(0)?,
                user_id: row.get(1)?,
                title: row.get(2)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String>(3)?)?
                    .with_timezone(&Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String>(4)?)?
                    .with_timezone(&Utc),
                message_count: row.get(5)?,
                last_message_at: row.get::<Option<String>>(6)?.map(|s| {
                    chrono::DateTime::parse_from_rfc3339(&s)
                        .unwrap()
                        .with_timezone(&Utc)
                }),
            };

            sessions.push(ChatSessionSummary::from(session));
        }

        Ok(ChatSessionListResponse {
            sessions,
            total_count,
        })
    }

    /// Get session details with messages
    pub async fn get_session_details(
        &self,
        conn: &Connection,
        session_id: &str,
        user_id: &str,
    ) -> Result<ChatSessionDetailsResponse> {
        let session = self.get_session(conn, session_id, user_id).await?;
        let messages = self.get_session_messages(conn, session_id).await?;

        Ok(ChatSessionDetailsResponse {
            session,
            messages: messages.clone(),
            total_messages: messages.len() as u32,
        })
    }

    /// Get messages for a session
    async fn get_session_messages(
        &self,
        conn: &Connection,
        session_id: &str,
    ) -> Result<Vec<ChatMessage>> {
        let stmt = conn
            .prepare(
                "SELECT id, session_id, role, content, context_vectors, token_count, created_at 
             FROM chat_messages WHERE session_id = ? ORDER BY created_at ASC",
            )
            .await?;

        let mut rows = stmt.query([session_id]).await?;

        let mut messages = Vec::new();
        while let Some(row) = rows.next().await? {
            let context_vectors: Option<String> = row.get(4)?; // Updated index
            let context_vectors_parsed = if let Some(cv) = context_vectors {
                Some(serde_json::from_str::<Vec<String>>(&cv)?)
            } else {
                None
            };

            messages.push(ChatMessage {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: match row.get::<String>(2)?.as_str() {
                    "user" => MessageRole::User,
                    "assistant" => MessageRole::Assistant,
                    "system" => MessageRole::System,
                    _ => MessageRole::User,
                },
                content: row.get(3)?,
                timestamp: chrono::DateTime::parse_from_rfc3339(&row.get::<String>(6)?)?
                    .with_timezone(&Utc),
                context_vectors: context_vectors_parsed,
                token_count: row.get(5)?,
            });
        }

        Ok(messages)
    }

    /// Update message content after streaming completes
    async fn update_message_content(
        &self,
        conn: &Connection,
        message_id: &str,
        content: String,
    ) -> Result<()> {
        log::info!(
            "Updating message content for message {} with {} characters",
            message_id,
            content.len()
        );

        let result = conn
            .execute(
                "UPDATE chat_messages SET content = ? WHERE id = ?",
                params![content, message_id],
            )
            .await;

        match result {
            Ok(rows_affected) => {
                log::info!(
                    "Successfully updated message {} - {} rows affected",
                    message_id,
                    rows_affected
                );
                Ok(())
            }
            Err(e) => {
                log::error!("Failed to update message {}: {}", message_id, e);
                Err(e.into())
            }
        }
    }

    /// Store a chat message
    async fn store_message(&self, conn: &Connection, message: &ChatMessage) -> Result<()> {
        let context_vectors_json = if let Some(cv) = &message.context_vectors {
            Some(serde_json::to_string(cv)?)
        } else {
            None
        };

        conn.execute(
            "INSERT INTO chat_messages (id, session_id, role, content, context_vectors, token_count, created_at) 
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                message.id.clone(),
                message.session_id.clone(), // FIXED: Use actual session_id instead of timestamp
                message.role.to_string(),
                message.content.clone(),
                context_vectors_json,
                message.token_count,
                message.timestamp.to_rfc3339()
            ],
        ).await?;

        Ok(())
    }

    /// Update session's last message timestamp and increment message count
    async fn update_session_last_message(&self, conn: &Connection, session_id: &str) -> Result<()> {
        conn.execute(
            "UPDATE chat_sessions SET last_message_at = ?, updated_at = ?, message_count = message_count + 2 WHERE id = ?",
            params![Utc::now().to_rfc3339(), Utc::now().to_rfc3339(), session_id],
        ).await?;

        Ok(())
    }

    /// Delete a chat session
    pub async fn delete_session(
        &self,
        conn: &Connection,
        session_id: &str,
        user_id: &str,
    ) -> Result<()> {
        // Verify session belongs to user
        self.get_session(conn, session_id, user_id).await?;

        conn.execute(
            "DELETE FROM chat_sessions WHERE id = ? AND user_id = ?",
            params![session_id, user_id],
        )
        .await?;

        Ok(())
    }

    /// Update session title
    pub async fn update_session_title(
        &self,
        conn: &Connection,
        session_id: &str,
        user_id: &str,
        title: String,
    ) -> Result<()> {
        conn.execute(
            "UPDATE chat_sessions SET title = ?, updated_at = ? WHERE id = ? AND user_id = ?",
            params![title, Utc::now().to_rfc3339(), session_id, user_id],
        )
        .await?;

        Ok(())
    }

    /// Update session title based on the first message
    async fn update_session_title_from_message(
        &self,
        conn: &Connection,
        session_id: &str,
        user_id: &str,
        first_message: &str,
    ) -> Result<()> {
        log::info!(
            "Updating session title from first message for session: {}",
            session_id
        );

        // Generate a title from the first message
        let title = self.generate_title_from_message(first_message).await?;

        // Update the session title
        self.update_session_title(conn, session_id, user_id, title.clone())
            .await?;

        log::info!("Successfully updated session title to: {}", title);
        Ok(())
    }

    /// Generate a concise title from a message
    async fn generate_title_from_message(&self, message: &str) -> Result<String> {
        // Simple title generation - take first 50 characters and clean up
        let mut title = message.trim().to_string();

        // Remove common question words at the beginning
        let question_words = [
            "what", "how", "why", "when", "where", "can", "could", "would", "should", "is", "are",
            "do", "does", "did",
        ];
        let words: Vec<&str> = title.split_whitespace().collect();

        if words.len() > 1 {
            let first_word = words[0].to_lowercase();
            if question_words.contains(&first_word.as_str()) {
                title = words[1..].join(" ");
            }
        }

        // Truncate to 50 characters and add ellipsis if needed
        if title.len() > 50 {
            title = title.chars().take(47).collect::<String>() + "...";
        }

        // Capitalize first letter
        if !title.is_empty() {
            let mut chars = title.chars();
            if let Some(first_char) = chars.next() {
                title = first_char.to_uppercase().collect::<String>() + chars.as_str();
            }
        }

        Ok(title)
    }

    /// Fix message counts for all sessions by recalculating from actual messages
    pub async fn fix_message_counts(&self, conn: &Connection) -> Result<u64> {
        log::info!("Starting message count fix for all sessions");

        let result = conn
            .execute(
                "UPDATE chat_sessions 
             SET message_count = (
                 SELECT COUNT(*) 
                 FROM chat_messages 
                 WHERE chat_messages.session_id = chat_sessions.id
             )
             WHERE EXISTS (
                 SELECT 1 
                 FROM chat_messages 
                 WHERE chat_messages.session_id = chat_sessions.id
             )",
                params![],
            )
            .await?;

        log::info!("Message count fix completed - {} sessions updated", result);
        Ok(result)
    }

    /// Health check for AI chat service
    pub async fn health_check(&self) -> Result<()> {
        // Check Voyager client (used for embeddings)
        self.voyager_client.health_check().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_creation() {
        let message = ChatMessage::new(
            "session123".to_string(),
            MessageRole::User,
            "Hello".to_string(),
        );
        assert_eq!(message.session_id, "session123");
        assert_eq!(message.role, MessageRole::User);
        assert_eq!(message.content, "Hello");
    }

    #[test]
    fn test_chat_session_creation() {
        let session = ChatSession::new("user123".to_string(), Some("Test Session".to_string()));
        assert_eq!(session.user_id, "user123");
        assert_eq!(session.title, Some("Test Session".to_string()));
    }
}
