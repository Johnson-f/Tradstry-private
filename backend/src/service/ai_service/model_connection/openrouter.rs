#![allow(dead_code)]

use crate::turso::vector_config::OpenRouterConfig;
use anyhow::{Context, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::mpsc;

/// Request structure for OpenRouter chat completion
#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub stream: bool,
    pub temperature: f32,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// Response structure from OpenRouter API (non-streaming)
#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub message: Message,
    pub finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
}

/// Streaming response chunk
#[derive(Debug, Deserialize)]
pub struct StreamChunk {
    pub choices: Vec<StreamChoice>,
}

#[derive(Debug, Deserialize)]
pub struct StreamChoice {
    pub delta: Option<MessageDelta>,
    pub finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct MessageDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallDelta>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ToolCallDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<FunctionCallDelta>,
}

#[derive(Debug, Deserialize)]
pub struct FunctionCallDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}

/// Tool definition for function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub r#type: String,
    pub function: FunctionDefinition,
}

/// Function definition for tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Tool call from the model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: FunctionCall,
}

/// Function call details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// Tool result to send back to the model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub role: String,
    pub content: String,
}

/// OpenRouter error response
#[derive(Debug, Deserialize)]
pub struct OpenRouterError {
    pub error: ErrorDetails,
}

#[derive(Debug, Deserialize)]
pub struct ErrorDetails {
    pub message: String,
    pub code: u16,
}

/// OpenRouter API client with streaming support
pub struct OpenRouterClient {
    config: OpenRouterConfig,
    client: Client,
}

impl OpenRouterClient {
    pub fn new(config: OpenRouterConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { config, client })
    }

    /// Generate a non-streaming chat completion
    pub async fn generate_chat(&self, messages: Vec<ChatMessage>) -> Result<String> {
        self.generate_chat_with_model(messages, None).await
    }

    /// Generate a non-streaming chat completion with optional model override
    pub async fn generate_chat_with_model(
        &self,
        messages: Vec<ChatMessage>,
        model_override: Option<String>,
    ) -> Result<String> {
        let openrouter_messages: Vec<Message> = messages
            .into_iter()
            .map(|msg| Message {
                role: msg.role.to_string(),
                content: Some(msg.content),
                tool_calls: None,
                tool_call_id: None,
            })
            .collect();

        let model = model_override.unwrap_or_else(|| self.config.model.clone());

        log::info!(
            "OpenRouter: Preparing non-streaming request with model: {}",
            model
        );
        let request = ChatRequest {
            model: model.clone(),
            messages: openrouter_messages,
            stream: false,
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            tools: None,
        };

        let mut retries = 0;
        loop {
            match self.make_chat_request(&request).await {
                Ok(response) => {
                    if let Some(choice) = response.choices.first() {
                        log::info!(
                            "OpenRouter: Successfully received response from model: {}",
                            model
                        );
                        if let Some(usage) = &response.usage {
                            log::info!(
                                "OpenRouter: Token usage - prompt: {:?}, completion: {:?}, total: {:?}",
                                usage.prompt_tokens,
                                usage.completion_tokens,
                                usage.total_tokens
                            );
                        }
                        // Handle tool calls - if message has tool_calls, return empty string
                        // The caller should check for tool_calls separately
                        if choice.message.tool_calls.is_some() {
                            return Ok(String::new());
                        }
                        return Ok(choice.message.content.clone().unwrap_or_default());
                    }
                    log::error!("OpenRouter: No content in response from model: {}", model);
                    return Err(anyhow::anyhow!("No content in OpenRouter response"));
                }
                Err(e) => {
                    let error_str = e.to_string();
                    log::warn!(
                        "OpenRouter: Request failed (attempt {}): {}",
                        retries + 1,
                        error_str
                    );

                    // Don't retry on data policy errors or auth errors
                    if error_str.contains("data policy")
                        || error_str.contains("privacy")
                        || error_str.contains("401")
                        || error_str.contains("User not found")
                    {
                        log::error!("OpenRouter: Non-retryable error, aborting");
                        return Err(e);
                    }

                    retries += 1;
                    if retries >= self.config.max_retries {
                        log::error!(
                            "OpenRouter: Max retries ({}) exceeded for model: {}",
                            self.config.max_retries,
                            model
                        );
                        return Err(e).context("Max retries exceeded for OpenRouter API");
                    }

                    // Exponential backoff
                    let delay = Duration::from_millis(1000 * 2_u64.pow(retries - 1));
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    /// Generate a streaming chat completion
    pub async fn generate_chat_stream(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<mpsc::Receiver<String>> {
        self.generate_chat_stream_with_model(messages, None).await
    }

    /// Generate a streaming chat completion with optional model override
    pub async fn generate_chat_stream_with_model(
        &self,
        messages: Vec<ChatMessage>,
        model_override: Option<String>,
    ) -> Result<mpsc::Receiver<String>> {
        let openrouter_messages: Vec<Message> = messages
            .into_iter()
            .map(|msg| Message {
                role: msg.role.to_string(),
                content: Some(msg.content),
                tool_calls: None,
                tool_call_id: None,
            })
            .collect();

        let model = model_override.unwrap_or_else(|| self.config.model.clone());

        log::info!(
            "OpenRouter: Preparing streaming request with model: {}",
            model
        );
        let request = ChatRequest {
            model: model.clone(),
            messages: openrouter_messages,
            stream: true,
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            tools: None,
        };

        let (tx, rx) = mpsc::channel(100);

        // Make HTTP request first to check status before spawning task
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse()?);
        headers.insert(
            "Authorization",
            format!("Bearer {}", self.config.api_key).parse()?,
        );

        // Add optional headers for site tracking
        if let Some(site_url) = &self.config.site_url {
            headers.insert("HTTP-Referer", site_url.parse()?);
        }
        if let Some(site_name) = &self.config.site_name {
            headers.insert("X-Title", site_name.parse()?);
        }

        let url = self.config.get_chat_url();
        let request_json = serde_json::to_value(&request)?;

        log::info!("OpenRouter: Sending streaming request to: {}", url);
        // Check HTTP response status before spawning streaming task
        let response = self
            .client
            .post(&url)
            .headers(headers.clone())
            .json(&request_json)
            .send()
            .await
            .context("Failed to send streaming request to OpenRouter API")?;

        let status = response.status();
        log::info!("OpenRouter response status: {}", status);

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            // Parse error details if possible
            if let Ok(error_response) = serde_json::from_str::<OpenRouterError>(&error_text) {
                // Check for data policy errors
                if error_response.error.message.contains("data policy")
                    || error_response.error.message.contains("privacy")
                {
                    return Err(anyhow::anyhow!(
                        "Data policy error: {}. Please configure your privacy settings at https://openrouter.ai/settings/privacy",
                        error_response.error.message
                    ));
                }
            }

            return Err(anyhow::anyhow!(
                "OpenRouter streaming API error: {} - {}",
                status,
                error_text
            ));
        }

        // If status is OK, spawn task to handle the stream
        log::info!(
            "OpenRouter: HTTP status OK, starting stream processing for model: {}",
            model
        );

        // Convert response into a stream for the spawned task
        let stream = response.bytes_stream();

        tokio::spawn(async move {
            if let Err(e) =
                Self::handle_streaming_response_from_stream(stream, tx, model.clone()).await
            {
                log::error!("OpenRouter streaming error: {}", e);
            }
        });

        Ok(rx)
    }
    /// Handle streaming response from OpenRouter API (legacy - for non-streaming requests)
    async fn handle_streaming_response(
        client: Client,
        url: String,
        config: OpenRouterConfig,
        request: serde_json::Value,
        tx: mpsc::Sender<String>,
    ) -> Result<()> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse()?);
        headers.insert(
            "Authorization",
            format!("Bearer {}", config.api_key).parse()?,
        );

        // Add optional headers for site tracking
        if let Some(site_url) = &config.site_url {
            headers.insert("HTTP-Referer", site_url.parse()?);
        }
        if let Some(site_name) = &config.site_name {
            headers.insert("X-Title", site_name.parse()?);
        }

        log::info!("OpenRouter: Sending request to: {}", url);
        let response = client
            .post(&url)
            .headers(headers)
            .json(&request)
            .send()
            .await
            .context("Failed to send streaming request to OpenRouter API")?;

        let status = response.status();
        log::info!("OpenRouter response status: {}", status);

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            // Parse error details if possible
            if let Ok(error_response) = serde_json::from_str::<OpenRouterError>(&error_text) {
                // Check for data policy errors
                if error_response.error.message.contains("data policy")
                    || error_response.error.message.contains("privacy")
                {
                    return Err(anyhow::anyhow!(
                        "Data policy error: {}. Please configure your privacy settings at https://openrouter.ai/settings/privacy",
                        error_response.error.message
                    ));
                }
            }

            return Err(anyhow::anyhow!(
                "OpenRouter streaming API error: {} - {}",
                status,
                error_text
            ));
        }

        let stream = response.bytes_stream();
        let model = request
            .get("model")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown")
            .to_string();
        Self::handle_streaming_response_from_stream(stream, tx, model).await
    }

    /// Handle streaming response from an existing stream
    async fn handle_streaming_response_from_stream(
        mut stream: impl futures_util::Stream<Item = Result<impl AsRef<[u8]>, reqwest::Error>> + Unpin,
        tx: mpsc::Sender<String>,
        model: String,
    ) -> Result<()> {
        log::info!("OpenRouter: Starting to read stream for model: {}", model);

        // Buffer to accumulate incomplete lines across chunks
        let mut line_buffer = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Failed to read streaming chunk")?;
            let chunk_str = String::from_utf8_lossy(chunk.as_ref());
            // Append chunk to buffer
            line_buffer.push_str(&chunk_str);

            // Split buffer into complete lines (ending with \n) and incomplete remainder
            // Use split_inclusive to keep newlines, then process complete lines
            let mut lines: Vec<String> = Vec::new();
            let parts: Vec<&str> = line_buffer.split_inclusive('\n').collect();

            // All parts except the last are complete lines
            if parts.len() > 1 {
                for part in &parts[..parts.len() - 1] {
                    // Remove the trailing newline
                    let line = part.strip_suffix('\n').unwrap_or(part).to_string();
                    if !line.is_empty() {
                        lines.push(line);
                    }
                }
                // Keep the last part (incomplete line) in buffer
                line_buffer = parts[parts.len() - 1].to_string();
            } else {
                // No newlines found, entire buffer is incomplete
                // Keep it for next chunk
            }

            // If we didn't find any complete lines, continue to next chunk
            if lines.is_empty() {
                continue;
            }

            // Process each complete line - OpenRouter returns SSE format
            for line in lines {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                // Parse SSE format: data: {...}
                if let Some(json_str) = line.strip_prefix("data: ") {
                    if json_str == "[DONE]" {
                        log::info!("Stream completed with [DONE]");
                        break;
                    }

                    // Validate JSON structure before parsing
                    let json_str_trimmed = json_str.trim();
                    if json_str_trimmed.is_empty() {
                        continue;
                    }

                    // Check if JSON appears complete (starts with { and ends with })
                    // This is a heuristic to avoid parsing incomplete JSON
                    let starts_with_brace = json_str_trimmed.starts_with('{');
                    let ends_with_brace = json_str_trimmed.ends_with('}');

                    // Check if JSON appears complete (starts with { and ends with })
                    // This is a heuristic to avoid parsing incomplete JSON
                    if !starts_with_brace || !ends_with_brace {
                        // This shouldn't happen if we're properly splitting by newlines,
                        // but if it does, we'll skip it and it should be completed in next chunk
                        continue;
                    }

                    match serde_json::from_str::<StreamChunk>(json_str_trimmed) {
                        Ok(stream_chunk) => {
                            if let Some(choice) = stream_chunk.choices.first() {
                                if let Some(delta) = &choice.delta
                                    && let Some(content) = &delta.content
                                {
                                    if let Err(e) = tx.send(content.clone()).await {
                                        log::error!(
                                            "Failed to send content through channel: {}",
                                            e
                                        );
                                        break;
                                    }
                                }

                                // Check if stream is finished
                                if choice.finish_reason.is_some() {
                                    log::info!(
                                        "Stream finished with reason: {:?}",
                                        choice.finish_reason
                                    );
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            let error_msg = e.to_string();
                            // Check if it's an incomplete JSON error (EOF, unexpected end, etc.)
                            let is_incomplete = error_msg.contains("EOF")
                                || error_msg.contains("unexpected end")
                                || error_msg.contains("invalid string")
                                || error_msg.contains("trailing characters");

                            if is_incomplete {
                                // This shouldn't happen if we're properly buffering, but log at debug level
                                // Don't add back to buffer - if line is complete (has \n),
                                // the JSON should be complete. If it's not, it's malformed.
                            } else {
                                // Real parsing error - log as warning
                                let json_preview = if json_str_trimmed.len() > 200 {
                                    format!("{}...", &json_str_trimmed[..200])
                                } else {
                                    json_str_trimmed.to_string()
                                };
                                log::warn!(
                                    "Failed to parse OpenRouter streaming chunk: {} - Error: {}",
                                    json_preview,
                                    e
                                );
                            }
                        }
                    }
                } else if !line.starts_with(":") {
                    // Ignore SSE comments
                }
            }
        }
        // Process any remaining buffered data as a final attempt
        if !line_buffer.trim().is_empty() {
            let line = line_buffer.trim();
            if let Some(json_str) = line.strip_prefix("data: ")
                && json_str != "[DONE]"
            {
                let json_str_trimmed = json_str.trim();
                if json_str_trimmed.starts_with('{') && json_str_trimmed.ends_with('}')
                    && let Ok(stream_chunk) =
                        serde_json::from_str::<StreamChunk>(json_str_trimmed)
                    && let Some(choice) = stream_chunk.choices.first()
                    && let Some(delta) = &choice.delta
                    && let Some(content) = &delta.content
                {
                    let _ = tx.send(content.clone()).await;
                }
            }
        }

        log::info!(
            "OpenRouter: Stream processing completed for model: {}",
            model
        );

        Ok(())
    }

    /// Make non-streaming chat request to OpenRouter API
    async fn make_chat_request(&self, request: &ChatRequest) -> Result<ChatResponse> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse()?);
        headers.insert(
            "Authorization",
            format!("Bearer {}", self.config.api_key).parse()?,
        );

        // Add optional headers for site tracking
        if let Some(site_url) = &self.config.site_url {
            headers.insert("HTTP-Referer", site_url.parse()?);
        }
        if let Some(site_name) = &self.config.site_name {
            headers.insert("X-Title", site_name.parse()?);
        }

        let url = self.config.get_chat_url();
        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(request)
            .send()
            .await
            .context("Failed to send request to OpenRouter API")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            // Parse error details if possible
            if let Ok(error_response) = serde_json::from_str::<OpenRouterError>(&error_text) {
                // Check for data policy errors
                if error_response.error.message.contains("data policy")
                    || error_response.error.message.contains("privacy")
                {
                    return Err(anyhow::anyhow!(
                        "Data policy error: {}. Please configure your privacy settings at https://openrouter.ai/settings/privacy",
                        error_response.error.message
                    ));
                }
            }

            return Err(anyhow::anyhow!(
                "OpenRouter API error: {} - {}",
                status,
                error_text
            ));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .context("Failed to parse OpenRouter API response")?;

        Ok(chat_response)
    }

    /// Test connection to OpenRouter API
    pub async fn test_connection(&self) -> Result<()> {
        let test_messages = vec![ChatMessage {
            role: MessageRole::User,
            content: "Hello".to_string(),
        }];

        self.generate_chat(test_messages).await?;
        Ok(())
    }

    /// Generate a non-streaming chat completion with tools
    pub async fn generate_chat_with_tools(
        &self,
        messages: Vec<ChatMessage>,
        tools: Vec<ToolDefinition>,
        model_override: Option<String>,
    ) -> Result<ChatResponseWithTools> {
        // Convert messages, handling tool results specially
        let mut openrouter_messages: Vec<Message> = Vec::new();

        for msg in messages {
            let role_str = msg.role.to_string();

            // Check if this is a tool result message (content starts with "Tool result (call_id:")
            if role_str == "system" && msg.content.starts_with("Tool result (call_id:") {
                // Extract tool_call_id and content from the formatted string
                if let Some(start) = msg.content.find("call_id: ") {
                    let rest = &msg.content[start + 9..];
                    if let Some(end) = rest.find(")") {
                        let tool_call_id = rest[..end].trim().to_string();
                        let content_start = rest.find(": ").map(|i| i + 2).unwrap_or(rest.len());
                        let content = rest[content_start..].trim().to_string();

                        openrouter_messages.push(Message {
                            role: "tool".to_string(),
                            content: Some(content),
                            tool_calls: None,
                            tool_call_id: Some(tool_call_id),
                        });
                        continue;
                    }
                }
            }

            // Regular message conversion
            openrouter_messages.push(Message {
                role: role_str,
                content: Some(msg.content),
                tool_calls: None,
                tool_call_id: None,
            });
        }

        let model = model_override.unwrap_or_else(|| self.config.model.clone());

        log::info!(
            "OpenRouter: Preparing tool-enabled request with model: {}",
            model
        );
        let request = ChatRequest {
            model: model.clone(),
            messages: openrouter_messages,
            stream: false,
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            tools: Some(tools),
        };

        let mut retries = 0;
        loop {
            match self.make_chat_request(&request).await {
                Ok(response) => {
                    log::info!(
                        "OpenRouter: Successfully received tool response from model: {}",
                        model
                    );
                    if let Some(usage) = &response.usage {
                        log::info!(
                            "OpenRouter: Token usage - prompt: {:?}, completion: {:?}, total: {:?}",
                            usage.prompt_tokens,
                            usage.completion_tokens,
                            usage.total_tokens
                        );
                    }
                    return Ok(ChatResponseWithTools {
                        choices: response.choices,
                        usage: response.usage,
                    });
                }
                Err(e) => {
                    let error_str = e.to_string();
                    log::warn!(
                        "OpenRouter: Tool request failed (attempt {}): {}",
                        retries + 1,
                        error_str
                    );

                    if error_str.contains("data policy")
                        || error_str.contains("privacy")
                        || error_str.contains("401")
                        || error_str.contains("User not found")
                    {
                        log::error!("OpenRouter: Non-retryable error, aborting");
                        return Err(e);
                    }

                    retries += 1;
                    if retries >= self.config.max_retries {
                        log::error!(
                            "OpenRouter: Max retries ({}) exceeded for tool request",
                            self.config.max_retries
                        );
                        return Err(e).context("Max retries exceeded for OpenRouter API");
                    }

                    let delay = Duration::from_millis(1000 * 2_u64.pow(retries - 1));
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    /// Get the model being used
    pub fn get_model(&self) -> &str {
        &self.config.model
    }
}

/// Response structure with tool calls support
#[derive(Debug, Deserialize)]
pub struct ChatResponseWithTools {
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

/// Chat message structure
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Debug, Clone)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageRole::User => write!(f, "user"),
            MessageRole::Assistant => write!(f, "assistant"),
            MessageRole::System => write!(f, "system"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_openrouter_client_creation() {
        let config = OpenRouterConfig {
            api_key: "test_key".to_string(),
            model: "meta-llama/llama-3.1-8b-instruct:free".to_string(),
            site_url: None,
            site_name: None,
            max_retries: 3,
            timeout_seconds: 60,
            max_tokens: 4096,
            temperature: 0.7,
        };

        let client = OpenRouterClient::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_message_role_to_string() {
        assert_eq!(MessageRole::User.to_string(), "user");
        assert_eq!(MessageRole::Assistant.to_string(), "assistant");
        assert_eq!(MessageRole::System.to_string(), "system");
    }
}
