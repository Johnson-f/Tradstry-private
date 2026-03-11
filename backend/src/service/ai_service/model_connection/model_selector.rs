#![allow(dead_code)]

use crate::service::ai_service::model_connection::gemini::GeminiClient;
use crate::service::ai_service::model_connection::openrouter::{ChatMessage, OpenRouterClient};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Provider type enum
#[derive(Debug, Clone, PartialEq)]
pub enum ProviderType {
    Gemini,
    OpenRouter,
}

/// Trait for model providers
#[async_trait]
pub trait ModelProvider: Send + Sync {
    async fn generate_chat(&self, messages: Vec<ChatMessage>) -> Result<String>;
    async fn generate_chat_stream(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<mpsc::Receiver<String>>;
    fn get_model_name(&self) -> &str;
    fn get_provider_type(&self) -> ProviderType;
}

/// Gemini provider wrapper
pub struct GeminiProvider {
    client: Arc<GeminiClient>,
}

impl GeminiProvider {
    pub fn new(client: Arc<GeminiClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl ModelProvider for GeminiProvider {
    async fn generate_chat(&self, messages: Vec<ChatMessage>) -> Result<String> {
        self.client.generate_chat(messages).await
    }

    async fn generate_chat_stream(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<mpsc::Receiver<String>> {
        self.client.generate_chat_stream(messages).await
    }

    fn get_model_name(&self) -> &str {
        self.client.get_model()
    }

    fn get_provider_type(&self) -> ProviderType {
        ProviderType::Gemini
    }
}

/// OpenRouter provider wrapper
pub struct OpenRouterProvider {
    client: Arc<OpenRouterClient>,
    model_name: String,
}

impl OpenRouterProvider {
    pub fn new(client: Arc<OpenRouterClient>, model_name: String) -> Self {
        Self { client, model_name }
    }
}

#[async_trait]
impl ModelProvider for OpenRouterProvider {
    async fn generate_chat(&self, messages: Vec<ChatMessage>) -> Result<String> {
        self.client
            .generate_chat_with_model(messages, Some(self.model_name.clone()))
            .await
    }

    async fn generate_chat_stream(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<mpsc::Receiver<String>> {
        self.client
            .generate_chat_stream_with_model(messages, Some(self.model_name.clone()))
            .await
    }

    fn get_model_name(&self) -> &str {
        &self.model_name
    }

    fn get_provider_type(&self) -> ProviderType {
        ProviderType::OpenRouter
    }
}

/// Model selector with automatic fallback
pub struct ModelSelector {
    providers: Vec<Box<dyn ModelProvider>>,
    current_provider_index: usize,
    fallback_enabled: bool,
    max_fallback_attempts: u32,
}

impl ModelSelector {
    /// Create a new ModelSelector
    pub fn new(
        gemini_client: Option<Arc<GeminiClient>>,
        openrouter_client: Arc<OpenRouterClient>,
        openrouter_models: Vec<String>,
        gemini_priority: bool,
        fallback_enabled: bool,
        max_fallback_attempts: u32,
    ) -> Self {
        let mut providers: Vec<Box<dyn ModelProvider>> = Vec::new();

        // Free OpenRouter models to use as fallback
        let free_openrouter_models = vec![
            "nvidia/nemotron-3-nano-30b-a3b:free".to_string(),
            "amazon/nova-2-lite-v1:free".to_string(),
            "allenai/olmo-3.1-32b-think:free".to_string(),
            "qwen/qwen3-coder:free".to_string(),
            "tngtech/deepseek-r1t2-chimera:free".to_string(),
        ];

        // Combine configured models with free models (avoid duplicates)
        let mut all_openrouter_models = openrouter_models.clone();
        for free_model in free_openrouter_models {
            if !all_openrouter_models.contains(&free_model) {
                all_openrouter_models.push(free_model);
            }
        }

        // Add Gemini first if priority is enabled and client is available
        if gemini_priority && let Some(ref gemini) = gemini_client {
            providers.push(Box::new(GeminiProvider::new(Arc::clone(gemini))));
            log::info!("ModelSelector: Added Gemini as primary provider");
        }

        // Add OpenRouter models
        for model in all_openrouter_models {
            let provider = OpenRouterProvider::new(Arc::clone(&openrouter_client), model.clone());
            providers.push(Box::new(provider));
            log::info!("ModelSelector: Added OpenRouter model: {}", model);
        }

        // If Gemini wasn't added as primary, add it now if available
        if !gemini_priority && let Some(ref gemini) = gemini_client {
            providers.push(Box::new(GeminiProvider::new(Arc::clone(gemini))));
            log::info!("ModelSelector: Added Gemini as fallback provider");
        }

        if providers.is_empty() {
            log::warn!("ModelSelector: No providers available! Using default OpenRouter model");
            // Fallback to default OpenRouter model
            let default_provider = OpenRouterProvider::new(
                Arc::clone(&openrouter_client),
                openrouter_client.get_model().to_string(),
            );
            providers.push(Box::new(default_provider));
        }

        log::info!(
            "ModelSelector initialized with {} providers, fallback_enabled={}",
            providers.len(),
            fallback_enabled
        );

        Self {
            providers,
            current_provider_index: 0,
            fallback_enabled,
            max_fallback_attempts,
        }
    }

    /// Generate chat with automatic fallback
    pub async fn generate_chat_with_fallback(
        &mut self,
        messages: Vec<ChatMessage>,
    ) -> Result<String> {
        if !self.fallback_enabled {
            // No fallback, just try current provider
            return self
                .providers
                .get(self.current_provider_index)
                .ok_or_else(|| anyhow::anyhow!("No providers available"))?
                .generate_chat(messages)
                .await;
        }

        let start_index = self.current_provider_index;
        // Try ALL providers, not just max_fallback_attempts
        let total_providers = self.providers.len();
        let mut attempts = 0;
        #[allow(unused_assignments)]
        let mut last_error = None::<anyhow::Error>;
        let mut tried_providers = Vec::new();

        while attempts < total_providers {
            let provider_index = (start_index + attempts) % total_providers;
            let provider = &self.providers[provider_index];

            let provider_name = format!(
                "{:?}:{}",
                provider.get_provider_type(),
                provider.get_model_name()
            );

            log::info!(
                "ModelSelector: Attempting provider {} (attempt {}/{})",
                provider_name,
                attempts + 1,
                total_providers
            );

            match provider.generate_chat(messages.clone()).await {
                Ok(response) => {
                    // Success! Update current provider index and return
                    if provider_index != self.current_provider_index {
                        log::info!(
                            "ModelSelector: Successfully used fallback provider {}",
                            provider_name
                        );
                        self.current_provider_index = provider_index;
                    } else {
                        log::info!(
                            "ModelSelector: Successfully used primary provider {}",
                            provider_name
                        );
                    }
                    return Ok(response);
                }
                Err(e) => {
                    last_error = Some(e);
                    let error_str = last_error.as_ref().unwrap().to_string();
                    tried_providers.push(provider_name.clone());

                    // Check if this is a non-retryable error (but 429 is retryable - rate limiting should trigger fallback)
                    let is_non_retryable = error_str.contains("API key")
                        || error_str.contains("401")
                        || error_str.contains("403")
                        || (error_str.contains("400") && !error_str.contains("429"));

                    if is_non_retryable {
                        log::error!(
                            "ModelSelector: Non-retryable error from {}: {}",
                            provider_name,
                            error_str
                        );
                        // Still try other providers unless it's an auth error
                        if error_str.contains("API key")
                            || error_str.contains("401")
                            || error_str.contains("403")
                        {
                            return Err(last_error.unwrap());
                        }
                    }

                    // Log rate limiting specifically
                    if error_str.contains("429")
                        || error_str.contains("rate limit")
                        || error_str.contains("quota")
                    {
                        log::warn!(
                            "ModelSelector: Rate limit/quota exceeded for {}: {}. Trying next provider...",
                            provider_name,
                            error_str
                        );
                    } else {
                        log::warn!(
                            "ModelSelector: Provider {} failed: {}. Trying next provider...",
                            provider_name,
                            error_str
                        );
                    }
                    attempts += 1;
                }
            }
        }

        // All providers failed - return user-friendly error
        log::error!(
            "ModelSelector: All {} providers failed. Tried: {:?}",
            total_providers,
            tried_providers
        );

        let error_message = if tried_providers.len() == 1 {
            "Unable to generate a response. The AI service is currently unavailable. Please try again in a few moments."
        } else {
            "Unable to generate a response. All AI services are currently unavailable. This may be due to rate limits or temporary service issues. Please try again in a few moments."
        };

        Err(anyhow::anyhow!("{}", error_message))
    }

    /// Generate streaming chat with automatic fallback
    pub async fn generate_chat_stream_with_fallback(
        &mut self,
        messages: Vec<ChatMessage>,
    ) -> Result<mpsc::Receiver<String>> {
        if !self.fallback_enabled {
            // No fallback, just try current provider
            return self
                .providers
                .get(self.current_provider_index)
                .ok_or_else(|| anyhow::anyhow!("No providers available"))?
                .generate_chat_stream(messages)
                .await;
        }

        let start_index = self.current_provider_index;
        // Try ALL providers, not just max_fallback_attempts
        let total_providers = self.providers.len();
        let mut attempts = 0;
        #[allow(unused_assignments)]
        let mut last_error = None::<anyhow::Error>;
        let mut tried_providers = Vec::new();

        while attempts < total_providers {
            let provider_index = (start_index + attempts) % total_providers;
            let provider = &self.providers[provider_index];

            let provider_name = format!(
                "{:?}:{}",
                provider.get_provider_type(),
                provider.get_model_name()
            );

            log::info!(
                "ModelSelector: Attempting streaming provider {} (attempt {}/{})",
                provider_name,
                attempts + 1,
                total_providers
            );

            match provider.generate_chat_stream(messages.clone()).await {
                Ok(receiver) => {
                    // Success! Update current provider index and return
                    if provider_index != self.current_provider_index {
                        log::info!(
                            "ModelSelector: Successfully used fallback streaming provider {}",
                            provider_name
                        );
                        self.current_provider_index = provider_index;
                    } else {
                        log::info!(
                            "ModelSelector: Successfully used primary streaming provider {}",
                            provider_name
                        );
                    }
                    return Ok(receiver);
                }
                Err(e) => {
                    last_error = Some(e);
                    let error_str = last_error.as_ref().unwrap().to_string();
                    tried_providers.push(provider_name.clone());

                    // Check if this is a non-retryable error (but 429 is retryable - rate limiting should trigger fallback)
                    let is_non_retryable = error_str.contains("API key")
                        || error_str.contains("401")
                        || error_str.contains("403")
                        || (error_str.contains("400") && !error_str.contains("429"));

                    if is_non_retryable {
                        log::error!(
                            "ModelSelector: Non-retryable error from {}: {}",
                            provider_name,
                            error_str
                        );
                        // Still try other providers unless it's an auth error
                        if error_str.contains("API key")
                            || error_str.contains("401")
                            || error_str.contains("403")
                        {
                            return Err(last_error.unwrap());
                        }
                    }

                    // Log rate limiting specifically
                    if error_str.contains("429")
                        || error_str.contains("rate limit")
                        || error_str.contains("quota")
                    {
                        log::warn!(
                            "ModelSelector: Rate limit/quota exceeded for {}: {}. Trying next provider...",
                            provider_name,
                            error_str
                        );
                    } else {
                        log::warn!(
                            "ModelSelector: Streaming provider {} failed: {}. Trying next provider...",
                            provider_name,
                            error_str
                        );
                    }
                    attempts += 1;
                }
            }
        }

        // All providers failed - return user-friendly error
        log::error!(
            "ModelSelector: All {} streaming providers failed. Tried: {:?}",
            total_providers,
            tried_providers
        );

        let error_message = if tried_providers.len() == 1 {
            "Unable to generate a streaming response. The AI service is currently unavailable. Please try again in a few moments."
        } else {
            "Unable to generate a streaming response. All AI services are currently unavailable. This may be due to rate limits or temporary service issues. Please try again in a few moments."
        };

        Err(anyhow::anyhow!("{}", error_message))
    }

    /// Reset to primary provider
    pub fn reset_to_primary(&mut self) {
        self.current_provider_index = 0;
        log::info!("ModelSelector: Reset to primary provider");
    }

    /// Get current provider name
    pub fn get_current_provider_name(&self) -> String {
        if let Some(provider) = self.providers.get(self.current_provider_index) {
            format!(
                "{:?}:{}",
                provider.get_provider_type(),
                provider.get_model_name()
            )
        } else {
            "Unknown".to_string()
        }
    }

    /// Get number of available providers
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }
}
