#![allow(dead_code)]

use crate::turso::vector_config::VoyagerConfig;
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Request structure for Voyager embeddings API
#[derive(Debug, Serialize)]
pub struct EmbeddingRequest {
    pub model: String,
    pub input: Vec<String>,
}

/// Response structure from Voyager embeddings API
#[derive(Debug, Deserialize)]
pub struct EmbeddingResponse {
    pub data: Vec<EmbeddingData>,
    pub usage: Usage,
}

#[derive(Debug, Deserialize)]
pub struct EmbeddingData {
    pub embedding: Vec<f32>,
    pub index: usize,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub total_tokens: u32,
}

/// Voyager API client for generating embeddings
pub struct VoyagerClient {
    config: VoyagerConfig,
    client: Client,
}

impl VoyagerClient {
    pub fn new(config: VoyagerConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .context("Failed to create HTTP client")?;

        let instance = Self { config, client };
        instance.validate_config()?;

        Ok(instance)
    }

    /// Validate configuration on initialization
    pub fn validate_config(&self) -> Result<()> {
        if self.config.api_key.is_empty() {
            return Err(anyhow::anyhow!("Voyager API key is empty"));
        }

        if self.config.api_key.len() < 20 {
            log::warn!(
                "Voyager API key seems too short - length={}",
                self.config.api_key.len()
            );
        }

        log::info!(
            "Voyager client configured - model={}, timeout={}s, batch_size={}, max_retries={}",
            self.config.model,
            self.config.timeout_seconds,
            self.config.batch_size,
            self.config.max_retries
        );

        Ok(())
    }

    /// Validate text content before embedding
    fn validate_content(&self, text: &str) -> Result<()> {
        const MAX_CHARS: usize = 16000; // Conservative limit

        if text.is_empty() {
            return Err(anyhow::anyhow!("Cannot embed empty text"));
        }

        if text.len() > MAX_CHARS {
            log::warn!(
                "Text exceeds recommended length - actual={}, max={}, text_preview='{}'",
                text.len(),
                MAX_CHARS,
                text.chars().take(100).collect::<String>()
            );
            return Err(anyhow::anyhow!(
                "Text length {} exceeds maximum {} characters",
                text.len(),
                MAX_CHARS
            ));
        }

        Ok(())
    }

    /// Generate embeddings for a single text
    pub async fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        self.validate_content(text)?;
        let embeddings = self.embed_texts(&[text.to_string()]).await?;
        Ok(embeddings.into_iter().next().unwrap_or_default())
    }

    /// Generate embeddings for multiple texts (batch processing)
    pub async fn embed_texts(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        // Split into batches if needed
        let mut all_embeddings = Vec::new();
        for batch in texts.chunks(self.config.batch_size) {
            let batch_embeddings = self.embed_batch(batch).await?;
            all_embeddings.extend(batch_embeddings);
        }

        Ok(all_embeddings)
    }

    /// Generate embeddings for a single batch
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let total_chars: usize = texts.iter().map(|t| t.len()).sum();

        log::debug!(
            "Embedding batch - texts={}, total_chars={}, model={}",
            texts.len(),
            total_chars,
            self.config.model
        );

        let request = EmbeddingRequest {
            model: self.config.model.clone(),
            input: texts.to_vec(),
        };

        let mut retries = 0;
        loop {
            match self.make_request(&request).await {
                Ok(response) => {
                    log::debug!(
                        "Embedding successful - embeddings={}, tokens={}",
                        response.data.len(),
                        response.usage.total_tokens
                    );
                    let embeddings: Vec<Vec<f32>> = response
                        .data
                        .into_iter()
                        .map(|data| data.embedding)
                        .collect();
                    return Ok(embeddings);
                }
                Err(e) => {
                    retries += 1;
                    let delay_ms = 1000 * 2_u64.pow(retries - 1);

                    log::warn!(
                        "Voyager API attempt {}/{} failed: {} - retrying in {}ms",
                        retries,
                        self.config.max_retries,
                        e,
                        delay_ms
                    );

                    if retries >= self.config.max_retries {
                        log::error!(
                            "Voyager API max retries exceeded - texts={}, total_chars={}, error={}",
                            texts.len(),
                            total_chars,
                            e
                        );
                        return Err(e).context(format!(
                            "Max retries ({}) exceeded for Voyager API. Batch size: {}, Total chars: {}",
                            self.config.max_retries, texts.len(), total_chars
                        ));
                    }

                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }
    }

    /// Make HTTP request to Voyager API - embedding model
    async fn make_request(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        let text_preview = request
            .input
            .iter()
            .take(2)
            .map(|s| s.chars().take(50).collect::<String>())
            .collect::<Vec<_>>()
            .join(", ");

        log::debug!(
            "Voyager API request - model={}, texts={}, preview='{}...'",
            request.model,
            request.input.len(),
            text_preview
        );

        let response = self
            .client
            .post(self.config.get_embeddings_url())
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await
            .context("Failed to send request to Voyager API")?;

        let status = response.status();

        if !status.is_success() {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read error body".to_string());

            log::error!(
                "Voyager API error - status={}, endpoint={}, error_body={}",
                status,
                self.config.get_embeddings_url(),
                error_body
            );

            return Err(anyhow::anyhow!(
                "Voyager API returned error status {}: {}",
                status,
                error_body
            ));
        }

        let embedding_response: EmbeddingResponse = response
            .json()
            .await
            .context("Failed to parse Voyager API response")?;

        Ok(embedding_response)
    }

    /// Health check for Voyager API
    pub async fn health_check(&self) -> Result<()> {
        // Test with a simple embedding request
        self.embed_text("test").await?;
        Ok(())
    }

    /// Get the model being used
    pub fn get_model(&self) -> &str {
        &self.config.model
    }

    /// Get the dimensions of embeddings produced by this model
    pub fn get_dimensions(&self) -> usize {
        1024 // voyage-finance-2 produces 1024-dimensional embeddings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_voyager_client_creation() {
        let config = VoyagerConfig {
            api_key: "test_key".to_string(),
            api_url: "https://api.voyageai.com/v1".to_string(),
            model: "voyage-finance-2".to_string(),
            max_retries: 3,
            timeout_seconds: 30,
            batch_size: 10,
        };

        let client = VoyagerClient::new(config);
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_embed_texts_empty() {
        let config = VoyagerConfig {
            api_key: "test_key".to_string(),
            api_url: "https://api.voyageai.com/v1".to_string(),
            model: "voyage-finance-2".to_string(),
            max_retries: 3,
            timeout_seconds: 30,
            batch_size: 10,
        };

        let client = VoyagerClient::new(config).unwrap();
        let result = client.embed_texts(&[]).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }
}
