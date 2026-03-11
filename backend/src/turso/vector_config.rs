#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::env;

/// Configuration for Upstash Vector database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorConfig {
    pub url: String,
    pub token: String,
    pub dimensions: usize,
    pub namespace_prefix: String,
    pub max_retries: u32,
    pub timeout_seconds: u64,
}

impl VectorConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(VectorConfig {
            url: env::var("UPSTASH_VECTOR_REST_URL")
                .map_err(|_| "UPSTASH_VECTOR_REST_URL environment variable not set")?,
            token: env::var("UPSTASH_VECTOR_REST_TOKEN")
                .map_err(|_| "UPSTASH_VECTOR_REST_TOKEN environment variable not set")?,
            dimensions: 1024, // voyage-finance-2 uses 1024 dimensions
            namespace_prefix: "user".to_string(),
            max_retries: 3,
            timeout_seconds: 30,
        })
    }

    /// Generate namespace for a specific user
    pub fn get_user_namespace(&self, user_id: &str) -> String {
        format!("{}_{}", self.namespace_prefix, user_id)
    }

    /// Get the base URL for vector operations
    pub fn get_base_url(&self) -> String {
        self.url.clone()
    }
}

/// Configuration for Voyager API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoyagerConfig {
    pub api_key: String,
    pub api_url: String,
    pub model: String,
    pub max_retries: u32,
    pub timeout_seconds: u64,
    pub batch_size: usize,
}

impl VoyagerConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(VoyagerConfig {
            api_key: env::var("VOYAGER_API_KEY")
                .map_err(|_| "VOYAGER_API_KEY environment variable not set")?,
            api_url: env::var("VOYAGER_API_URL")
                .unwrap_or_else(|_| "https://api.voyageai.com/v1".to_string()),
            model: "voyage-finance-2".to_string(),
            max_retries: 3,
            timeout_seconds: 30,
            batch_size: 10, // Voyager API limit
        })
    }

    /// Get the embeddings endpoint URL
    pub fn get_embeddings_url(&self) -> String {
        format!("{}/embeddings", self.api_url)
    }
}

/// Configuration for OpenRouter API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterConfig {
    pub api_key: String,
    pub model: String,
    pub site_url: Option<String>,
    pub site_name: Option<String>,
    pub max_retries: u32,
    pub timeout_seconds: u64,
    pub max_tokens: u32,
    pub temperature: f32,
}

impl OpenRouterConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(OpenRouterConfig {
            api_key: env::var("OPENROUTER_API_KEY")
                .map_err(|_| "OPENROUTER_API_KEY environment variable not set")?,
            model: env::var("OPENROUTER_MODEL")
                .unwrap_or_else(|_| "deepseek/deepseek-chat-v3.1:free".to_string()),
            site_url: env::var("OPENROUTER_SITE_URL").ok(),
            site_name: env::var("OPENROUTER_SITE_NAME").ok(),
            max_retries: 3,
            timeout_seconds: 60,
            max_tokens: 4096,
            temperature: 0.7,
        })
    }

    /// Get the chat completion endpoint URL
    pub fn get_chat_url(&self) -> String {
        "https://openrouter.ai/api/v1/chat/completions".to_string()
    }
}

/// Configuration for Google Gemini API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiConfig {
    pub api_key: String,
    pub model: String,
    pub api_url: String,
    pub max_retries: u32,
    pub timeout_seconds: u64,
    pub max_tokens: u32,
    pub temperature: f32,
}

impl GeminiConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(GeminiConfig {
            api_key: env::var("GEMINI_API_KEY")
                .map_err(|_| "GEMINI_API_KEY environment variable not set")?,
            model: env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-2.0-flash-exp".to_string()),
            api_url: env::var("GEMINI_API_URL")
                .unwrap_or_else(|_| "https://generativelanguage.googleapis.com/v1beta".to_string()),
            max_retries: env::var("GEMINI_MAX_RETRIES")
                .unwrap_or_else(|_| "3".to_string())
                .parse()
                .unwrap_or(3),
            timeout_seconds: env::var("GEMINI_TIMEOUT_SECONDS")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .unwrap_or(60),
            max_tokens: env::var("GEMINI_MAX_TOKENS")
                .unwrap_or_else(|_| "4096".to_string())
                .parse()
                .unwrap_or(4096),
            temperature: env::var("GEMINI_TEMPERATURE")
                .unwrap_or_else(|_| "0.7".to_string())
                .parse()
                .unwrap_or(0.7),
        })
    }

    /// Get the chat completion endpoint URL
    pub fn get_chat_url(&self) -> String {
        format!("{}/models/{}:generateContent", self.api_url, self.model)
    }
}

/// Configuration for Upstash Search database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    pub url: String,
    pub token: String,
    pub max_retries: u32,
    pub timeout_seconds: u64,
    pub namespace_prefix: String,
}

impl SearchConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(SearchConfig {
            url: env::var("UPSTASH_SEARCH_REST_URL")
                .map_err(|_| "UPSTASH_SEARCH_REST_URL environment variable not set")?,
            token: env::var("UPSTASH_SEARCH_REST_TOKEN")
                .map_err(|_| "UPSTASH_SEARCH_REST_TOKEN environment variable not set")?,
            max_retries: 3,
            timeout_seconds: 30,
            namespace_prefix: "user".to_string(),
        })
    }

    /// Generate namespace for a specific user
    pub fn get_user_namespace(&self, user_id: &str) -> String {
        format!("{}_{}", self.namespace_prefix, user_id)
    }

    /// Get index name for a specific user (same as namespace for Upstash Search)
    pub fn get_index_name(&self, user_id: &str) -> String {
        self.get_user_namespace(user_id)
    }

    /// Get the base URL for search operations
    pub fn get_base_url(&self) -> String {
        self.url.clone()
    }
}

/// Configuration for Qdrant (self-hosted or Cloud)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QdrantConfig {
    pub url: String,
    pub api_key: String,
    pub collection_prefix: String,
    pub max_retries: u32,
    pub timeout_seconds: u64,
}

impl QdrantConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(QdrantConfig {
            url: env::var("QDRANT_URL").map_err(|_| "QDRANT_URL environment variable not set")?,
            // API key is optional for self-hosted instances without authentication
            // Required for Qdrant Cloud or self-hosted instances with authentication enabled
            api_key: env::var("QDRANT_API_KEY").unwrap_or_default(),
            collection_prefix: "tradistry".to_string(),
            max_retries: 3,
            timeout_seconds: 30,
        })
    }

    pub fn get_collection_name(&self, user_id: &str) -> String {
        format!("{}_{}", self.collection_prefix, user_id)
    }
}

/// Model selector configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSelectorConfig {
    pub gemini_priority: bool,
    pub openrouter_fallback_models: Vec<String>,
    pub fallback_enabled: bool,
    pub max_fallback_attempts: u32,
}

impl ModelSelectorConfig {
    pub fn from_env() -> Self {
        Self {
            gemini_priority: env::var("GEMINI_PRIORITY")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            openrouter_fallback_models: env::var("OPENROUTER_FALLBACK_MODELS")
                .unwrap_or_else(|_| {
                    "x-ai/grok-4.1-fast:free,deepseek/deepseek-r1:free,kwaipilot/kat-coder-pro:free"
                        .to_string()
                })
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            fallback_enabled: env::var("MODEL_FALLBACK_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            max_fallback_attempts: env::var("MAX_FALLBACK_ATTEMPTS")
                .unwrap_or_else(|_| "3".to_string())
                .parse()
                .unwrap_or(3),
        }
    }
}

/// Hybrid search configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchConfig {
    pub enabled: bool,
    pub ai_reranking_enabled: bool,
    pub vector_weight: f32,
    pub keyword_weight: f32,
    pub max_results: usize,
}

impl HybridSearchConfig {
    pub fn from_env() -> Self {
        Self {
            enabled: env::var("HYBRID_SEARCH_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            ai_reranking_enabled: env::var("AI_RERANKING_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            vector_weight: env::var("HYBRID_VECTOR_WEIGHT")
                .unwrap_or_else(|_| "0.6".to_string())
                .parse()
                .unwrap_or(0.6),
            keyword_weight: env::var("HYBRID_KEYWORD_WEIGHT")
                .unwrap_or_else(|_| "0.4".to_string())
                .parse()
                .unwrap_or(0.4),
            max_results: env::var("HYBRID_MAX_RESULTS")
                .unwrap_or_else(|_| "20".to_string())
                .parse()
                .unwrap_or(20),
        }
    }
}

/// AI Configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIConfig {
    pub vector_config: VectorConfig,
    pub qdrant_config: QdrantConfig,
    pub voyager_config: VoyagerConfig,
    pub openrouter_config: OpenRouterConfig,
    pub gemini_config: Option<GeminiConfig>,
    pub hybrid_config: HybridSearchConfig,
    pub max_context_vectors: usize,
    pub insights_schedule_enabled: bool,
    pub insights_schedule_hour: u8,
    pub batch_vectorization_interval_minutes: u64,
}

impl AIConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(AIConfig {
            vector_config: VectorConfig::from_env()?,
            qdrant_config: QdrantConfig::from_env()?,
            voyager_config: VoyagerConfig::from_env()?,
            openrouter_config: OpenRouterConfig::from_env()?,
            gemini_config: GeminiConfig::from_env().ok(), // Optional - only if GEMINI_API_KEY is set
            hybrid_config: HybridSearchConfig::from_env(),
            max_context_vectors: env::var("MAX_CONTEXT_VECTORS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .unwrap_or(10),
            insights_schedule_enabled: env::var("AI_INSIGHTS_SCHEDULE_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            insights_schedule_hour: env::var("AI_INSIGHTS_SCHEDULE_HOUR")
                .unwrap_or_else(|_| "2".to_string())
                .parse()
                .unwrap_or(2),
            batch_vectorization_interval_minutes: env::var("BATCH_VECTORIZATION_INTERVAL_MINUTES")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .unwrap_or(60),
        })
    }
}
