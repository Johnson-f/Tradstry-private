#![allow(dead_code)]

use crate::service::ai_service::model_connection::openrouter::{ChatMessage, MessageRole};
use crate::turso::vector_config::GeminiConfig;
use anyhow::{Context, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::mpsc;

/// Request structure for Gemini chat completion
#[derive(Debug, Serialize)]
pub struct GeminiChatRequest {
    pub contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<GeminiSystemInstruction>,
    pub generation_config: GeminiGenerationConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiContent {
    pub role: String,
    pub parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiPart {
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct GeminiSystemInstruction {
    pub parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
pub struct GeminiGenerationConfig {
    pub temperature: f32,
    pub max_output_tokens: u32,
}

/// Response structure from Gemini API (non-streaming)
#[derive(Debug, Deserialize)]
pub struct GeminiChatResponse {
    pub candidates: Vec<GeminiCandidate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_metadata: Option<GeminiUsageMetadata>,
}

#[derive(Debug, Deserialize)]
pub struct GeminiCandidate {
    pub content: GeminiContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GeminiUsageMetadata {
    pub prompt_token_count: Option<u32>,
    pub candidates_token_count: Option<u32>,
    pub total_token_count: Option<u32>,
}

/// Streaming response chunk from Gemini API
#[derive(Debug, Deserialize)]
pub struct GeminiStreamChunk {
    pub candidates: Vec<GeminiStreamCandidate>,
}

#[derive(Debug, Deserialize)]
pub struct GeminiStreamCandidate {
    pub content: Option<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// Gemini error response
#[derive(Debug, Deserialize)]
pub struct GeminiError {
    pub error: GeminiErrorDetails,
}

#[derive(Debug, Deserialize)]
pub struct GeminiErrorDetails {
    pub message: String,
    pub code: Option<u16>,
    pub status: Option<String>,
}

/// Google Gemini API client with streaming support
pub struct GeminiClient {
    config: GeminiConfig,
    client: Client,
}

impl GeminiClient {
    pub fn new(config: GeminiConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { config, client })
    }

    /// Generate a non-streaming chat completion
    pub async fn generate_chat(&self, messages: Vec<ChatMessage>) -> Result<String> {
        let (contents, system_instruction) = self.convert_messages(messages);

        let request = GeminiChatRequest {
            contents,
            system_instruction,
            generation_config: GeminiGenerationConfig {
                temperature: self.config.temperature,
                max_output_tokens: self.config.max_tokens,
            },
        };

        let mut retries = 0;
        loop {
            match self.make_chat_request(&request).await {
                Ok(response) => {
                    if let Some(candidate) = response.candidates.first()
                        && let Some(part) = candidate.content.parts.first()
                    {
                        return Ok(part.text.clone());
                    }
                    return Err(anyhow::anyhow!("No content in Gemini response"));
                }
                Err(e) => {
                    // Don't retry on authentication errors
                    if e.to_string().contains("API key")
                        || e.to_string().contains("401")
                        || e.to_string().contains("403")
                    {
                        return Err(e);
                    }

                    retries += 1;
                    if retries >= self.config.max_retries {
                        return Err(e).context("Max retries exceeded for Gemini API");
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
        let (contents, system_instruction) = self.convert_messages(messages);

        let request = GeminiChatRequest {
            contents,
            system_instruction,
            generation_config: GeminiGenerationConfig {
                temperature: self.config.temperature,
                max_output_tokens: self.config.max_tokens,
            },
        };

        let (tx, rx) = mpsc::channel(100);

        // Make HTTP request first to check status before spawning task
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse()?);
        headers.insert("x-goog-api-key", self.config.api_key.parse()?);

        let url_with_stream = format!("{}?alt=sse", self.config.get_chat_url());
        let request_json = serde_json::to_value(&request)?;

        log::info!("Sending streaming request to Gemini: {}", url_with_stream);

        // Check HTTP response status before spawning streaming task
        let response = self
            .client
            .post(&url_with_stream)
            .headers(headers.clone())
            .json(&request_json)
            .send()
            .await
            .context("Failed to send streaming request to Gemini API")?;

        let status = response.status();
        log::info!("Gemini response status: {}", status);

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            log::error!("Gemini API error: {} - {}", status, error_text);

            // Parse error details if possible
            if let Ok(error_response) = serde_json::from_str::<GeminiError>(&error_text) {
                log::error!("Gemini API error details: {}", error_response.error.message);
            }

            return Err(anyhow::anyhow!(
                "Gemini streaming API error: {} - {}",
                status,
                error_text
            ));
        }

        // If status is OK, spawn task to handle the stream
        // Convert response into a stream for the spawned task
        let stream = response.bytes_stream();

        tokio::spawn(async move {
            if let Err(e) = Self::handle_streaming_response_from_stream(stream, tx).await {
                log::error!("Gemini streaming error: {}", e);
            }
        });

        Ok(rx)
    }

    /// Handle streaming response from Gemini API (legacy - for non-streaming requests)
    async fn handle_streaming_response(
        client: Client,
        url: String,
        config: GeminiConfig,
        request: serde_json::Value,
        tx: mpsc::Sender<String>,
    ) -> Result<()> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse()?);
        headers.insert("x-goog-api-key", config.api_key.parse()?);

        // Add stream parameter to URL
        let url_with_stream = format!("{}?alt=sse", url);

        log::info!("Sending streaming request to Gemini: {}", url_with_stream);
        log::debug!(
            "Request payload: {}",
            serde_json::to_string_pretty(&request).unwrap_or_default()
        );

        let response = client
            .post(&url_with_stream)
            .headers(headers)
            .json(&request)
            .send()
            .await
            .context("Failed to send streaming request to Gemini API")?;

        let status = response.status();
        log::info!("Gemini response status: {}", status);

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            log::error!("Gemini API error: {} - {}", status, error_text);

            // Parse error details if possible
            if let Ok(error_response) = serde_json::from_str::<GeminiError>(&error_text) {
                log::error!("Gemini API error details: {}", error_response.error.message);
            }

            return Err(anyhow::anyhow!(
                "Gemini streaming API error: {} - {}",
                status,
                error_text
            ));
        }

        let stream = response.bytes_stream();
        Self::handle_streaming_response_from_stream(stream, tx).await
    }

    /// Handle streaming response from an existing stream
    async fn handle_streaming_response_from_stream(
        mut stream: impl futures_util::Stream<Item = Result<impl AsRef<[u8]>, reqwest::Error>> + Unpin,
        tx: mpsc::Sender<String>,
    ) -> Result<()> {
        log::info!("Starting to read Gemini stream...");

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Failed to read streaming chunk")?;
            let chunk_str = String::from_utf8_lossy(chunk.as_ref());
            log::debug!("Received chunk: {}", chunk_str);

            // Process each line in the chunk - Gemini returns SSE format
            for line in chunk_str.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                log::debug!("Processing line: {}", line);

                // Parse SSE format: data: {...}
                if let Some(json_str) = line.strip_prefix("data: ") {
                    log::debug!("Parsing JSON: {}", json_str);

                    match serde_json::from_str::<GeminiStreamChunk>(json_str) {
                        Ok(stream_chunk) => {
                            if let Some(candidate) = stream_chunk.candidates.first()
                                && let Some(content) = &candidate.content
                                && let Some(part) = content.parts.first()
                            {
                                log::debug!("Sending content: {}", part.text);
                                if let Err(e) = tx.send(part.text.clone()).await {
                                    log::error!(
                                        "Failed to send content through channel: {}",
                                        e
                                    );
                                    break;
                                }

                                if candidate.finish_reason.is_some() {
                                    log::info!(
                                        "Stream finished with reason: {:?}",
                                        candidate.finish_reason
                                    );
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            log::warn!(
                                "Failed to parse Gemini streaming chunk: {} - Error: {}",
                                json_str,
                                e
                            );
                        }
                    }
                } else if !line.starts_with(":") {
                    // Ignore SSE comments
                    log::debug!("Unexpected line format: {}", line);
                }
            }
        }

        log::info!("Gemini stream processing completed");

        Ok(())
    }

    /// Make non-streaming chat request to Gemini API
    async fn make_chat_request(&self, request: &GeminiChatRequest) -> Result<GeminiChatResponse> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse()?);
        headers.insert("x-goog-api-key", self.config.api_key.parse()?);

        let response = self
            .client
            .post(self.config.get_chat_url())
            .headers(headers)
            .json(request)
            .send()
            .await
            .context("Failed to send request to Gemini API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            // Parse error details if possible
            if let Ok(error_response) = serde_json::from_str::<GeminiError>(&error_text) {
                log::error!("Gemini API error details: {}", error_response.error.message);

                // Check for authentication errors
                if error_response.error.message.contains("API key")
                    || status == 401
                    || status == 403
                {
                    return Err(anyhow::anyhow!(
                        "Gemini API authentication error: {}. Please check your GEMINI_API_KEY.",
                        error_response.error.message
                    ));
                }

                // Check for rate limiting
                if status == 429 {
                    return Err(anyhow::anyhow!(
                        "Gemini API rate limit exceeded: {}. Please try again later.",
                        error_response.error.message
                    ));
                }
            }

            return Err(anyhow::anyhow!(
                "Gemini API error: {} - {}",
                status,
                error_text
            ));
        }

        let chat_response: GeminiChatResponse = response
            .json()
            .await
            .context("Failed to parse Gemini API response")?;

        Ok(chat_response)
    }

    /// Convert internal ChatMessage format to Gemini format
    fn convert_messages(
        &self,
        messages: Vec<ChatMessage>,
    ) -> (Vec<GeminiContent>, Option<GeminiSystemInstruction>) {
        let mut contents = Vec::new();
        let mut system_instruction = None;

        for msg in messages {
            match msg.role {
                MessageRole::System => {
                    // Gemini uses system_instruction field for system messages
                    system_instruction = Some(GeminiSystemInstruction {
                        parts: vec![GeminiPart { text: msg.content }],
                    });
                }
                MessageRole::User | MessageRole::Assistant => {
                    contents.push(GeminiContent {
                        role: msg.role.to_string(),
                        parts: vec![GeminiPart { text: msg.content }],
                    });
                }
            }
        }

        (contents, system_instruction)
    }

    /// Test connection to Gemini API
    pub async fn test_connection(&self) -> Result<()> {
        let test_messages = vec![ChatMessage {
            role: MessageRole::User,
            content: "Hello".to_string(),
        }];

        self.generate_chat(test_messages).await?;
        Ok(())
    }

    /// Get the model being used
    pub fn get_model(&self) -> &str {
        &self.config.model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gemini_client_creation() {
        let config = GeminiConfig {
            api_key: "test_key".to_string(),
            model: "gemini-2.0-flash-exp".to_string(),
            api_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
            max_retries: 3,
            timeout_seconds: 60,
            max_tokens: 4096,
            temperature: 0.7,
        };

        let client = GeminiClient::new(config);
        assert!(client.is_ok());
    }
}
