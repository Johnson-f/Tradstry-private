//! Turso integration module for Rust backend
//!
//! This module provides database connection and user-scoped access
//! for a single shared Turso database with user_id isolation.

pub mod schema;

pub mod auth;
pub mod client;
pub mod config;
pub mod redis;
pub mod vector_config;

// Re-export commonly used items
pub use auth::{get_supabase_user_id, validate_jwt_token_from_query, validate_supabase_jwt_token};
pub use client::{TursoClient, UserDb};
pub use config::{SupabaseClaims, TursoConfig};

use crate::service::ai_service::{
    AIChatService, AiReportsService, ChatVectorization, GeminiClient, ModelSelector,
    NotebookVectorization, OpenRouterClient, PlaybookVectorization, QdrantDocumentClient,
    TradeAnalysisService, TradeVectorService, TradeVectorization, VoyagerClient,
};
use crate::service::caching::CacheService;
use crate::service::tool_engine::ToolEngine;
use crate::service::trade_notes_service::TradeNotesService;
use crate::service::utils::account_deletion::AccountDeletionService;
use crate::service::utils::rate_limiter::RateLimiter;
use crate::service::utils::storage_quota::StorageQuotaService;
use std::sync::Arc;

/// Application state containing Turso configuration and connections
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<TursoConfig>,
    pub turso_client: Arc<TursoClient>,
    pub cache_service: Arc<CacheService>,
    pub rate_limiter: Arc<RateLimiter>,
    pub storage_quota_service: Arc<StorageQuotaService>,
    pub account_deletion_service: Arc<AccountDeletionService>,
    pub ai_chat_service: Arc<AIChatService>,
    #[allow(dead_code)]
    pub ai_reports_service: Arc<AiReportsService>,
    pub analysis_service: Arc<TradeAnalysisService>,
    pub trade_notes_service: Arc<TradeNotesService>,
    pub trade_vector_service: Arc<TradeVectorService>,
    pub trade_vectorization: Arc<TradeVectorization>,
    pub notebook_vector_service: Arc<NotebookVectorization>,
    pub playbook_vector_service: Arc<PlaybookVectorization>,
    #[allow(dead_code)]
    pub gemini_client: Option<Arc<GeminiClient>>,
    #[allow(dead_code)]
    pub model_selector: Arc<tokio::sync::Mutex<ModelSelector>>,
}

impl AppState {
    /// Initialize application state with Turso connections
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Load configuration from environment
        let config = Arc::new(TursoConfig::from_env()?);

        // Initialize Turso client
        let turso_client = Arc::new(TursoClient::new((*config).clone()).await?);

        // Initialize Redis client
        let redis_config = crate::turso::redis::RedisConfig::from_env()
            .map_err(|e| format!("Failed to load Redis config: {}", e))?;

        let redis_client = crate::turso::redis::RedisClient::new(redis_config)
            .await
            .map_err(|e| format!("Failed to create Redis client: {}", e))?;

        // Initialize cache service
        let mut cache_service = CacheService::new(redis_client.clone());
        cache_service
            .initialize()
            .await
            .map_err(|e| format!("Failed to initialize cache service: {}", e))?;

        let cache_service = Arc::new(cache_service);

        // Initialize rate limiter (uses same Redis client)
        let rate_limiter = Arc::new(RateLimiter::new(redis_client));

        // Initialize storage quota service
        let storage_quota_service = Arc::new(StorageQuotaService::new(Arc::clone(&turso_client)));

        // Initialize AI services
        let openrouter_config = crate::turso::vector_config::OpenRouterConfig::from_env()
            .map_err(|e| format!("Failed to load OpenRouter config: {}", e))?;
        let openrouter_client = Arc::new(OpenRouterClient::new(openrouter_config)?);

        // Initialize Gemini client (optional - only if GEMINI_API_KEY is set)
        let gemini_client = match crate::turso::vector_config::GeminiConfig::from_env() {
            Ok(gemini_config) => match GeminiClient::new(gemini_config) {
                Ok(client) => {
                    log::info!("Gemini client initialized successfully");
                    Some(Arc::new(client))
                }
                Err(e) => {
                    log::warn!(
                        "Failed to initialize Gemini client: {}. Continuing without Gemini support.",
                        e
                    );
                    None
                }
            },
            Err(_) => {
                log::debug!("GEMINI_API_KEY not set, skipping Gemini client initialization");
                None
            }
        };

        // Initialize ModelSelector with fallback models
        let model_selector_config = crate::turso::vector_config::ModelSelectorConfig::from_env();
        let model_selector = ModelSelector::new(
            gemini_client.clone(),
            Arc::clone(&openrouter_client),
            model_selector_config.openrouter_fallback_models.clone(),
            model_selector_config.gemini_priority,
            model_selector_config.fallback_enabled,
            model_selector_config.max_fallback_attempts,
        );
        let model_selector = Arc::new(tokio::sync::Mutex::new(model_selector));

        let voyager_config = crate::turso::vector_config::VoyagerConfig::from_env()
            .map_err(|e| format!("Failed to load Voyager config: {}", e))?;
        let voyager_client = Arc::new(VoyagerClient::new(voyager_config)?);

        let qdrant_config = crate::turso::vector_config::QdrantConfig::from_env()
            .map_err(|e| format!("Failed to load Qdrant config: {}", e))?;
        let qdrant_client = Arc::new(
            QdrantDocumentClient::new(qdrant_config.clone())
                .await
                .map_err(|e| format!("Failed to create Qdrant client: {}", e))?,
        );

        // Perform health check (non-blocking - log warning if it fails but don't fail startup)
        let qdrant_client_for_health = Arc::clone(&qdrant_client);
        tokio::spawn(async move {
            if let Err(e) = qdrant_client_for_health.health_check().await {
                log::warn!(
                    "Qdrant health check failed: {}. The service will continue, but vector operations may fail.",
                    e
                );
            }
        });

        // Initialize ChatVectorization service
        let chat_vector_service = Arc::new(ChatVectorization::new(
            Arc::clone(&voyager_client),
            Arc::clone(&qdrant_client),
        ));

        // Initialize PlaybookVectorization service
        let playbook_vector_service = Arc::new(PlaybookVectorization::new(
            Arc::clone(&voyager_client),
            Arc::clone(&qdrant_client),
        ));

        // Initialize NotebookVectorization service
        let notebook_vector_service = Arc::new(NotebookVectorization::new(
            Arc::clone(&voyager_client),
            Arc::clone(&qdrant_client),
        ));

        // Initialize ToolEngine for market data tools
        let tool_engine = Arc::new(ToolEngine::new());

        let ai_chat_service = Arc::new(AIChatService::new(
            Arc::clone(&chat_vector_service),
            Arc::clone(&qdrant_client),
            Arc::clone(&model_selector),
            Arc::clone(&openrouter_client),
            Arc::clone(&turso_client),
            Arc::clone(&voyager_client),
            Arc::clone(&tool_engine),
            10000, // max_context_vectors
        ));

        let ai_reports_service = Arc::new(AiReportsService::new(Arc::clone(&turso_client)));

        // Initialize TradeAnalysisService for real-time analysis
        let analysis_service = Arc::new(TradeAnalysisService::new(
            Arc::clone(&qdrant_client),
            Arc::clone(&voyager_client),
            Arc::clone(&model_selector),
            Arc::clone(&turso_client),
        ));

        let trade_notes_service = Arc::new(TradeNotesService::new(Arc::clone(&cache_service)));

        // Initialize AccountDeletionService
        let supabase_url = std::env::var("SUPABASE_URL")
            .map_err(|_| "SUPABASE_URL environment variable not set")?;
        let supabase_service_role_key = std::env::var("SUPABASE_SERVICE_ROLE_KEY")
            .map_err(|_| "SUPABASE_SERVICE_ROLE_KEY environment variable not set")?;

        let account_deletion_service = Arc::new(AccountDeletionService::new(
            Arc::clone(&turso_client),
            Arc::clone(&qdrant_client),
            supabase_url,
            supabase_service_role_key,
        ));

        // Initialize TradeVectorService for vectorizing trade mistakes and notes
        let trade_vector_service = Arc::new(TradeVectorService::new(
            Arc::clone(&voyager_client),
            Arc::clone(&qdrant_client),
        ));

        // Initialize TradeVectorization for new structured trade vectorization
        let trade_vectorization = Arc::new(TradeVectorization::new(
            Arc::clone(&voyager_client),
            Arc::clone(&qdrant_client),
        ));

        Ok(Self {
            config,
            turso_client,
            cache_service,
            rate_limiter,
            storage_quota_service,
            account_deletion_service,
            ai_chat_service,
            ai_reports_service,
            analysis_service,
            trade_notes_service,
            trade_vector_service,
            trade_vectorization,
            notebook_vector_service,
            playbook_vector_service,
            gemini_client,
            model_selector,
        })
    }

    /// Get scoped UserDb for a specific user
    pub fn get_user_db(
        &self,
        user_id: &str,
    ) -> Result<client::UserDb, Box<dyn std::error::Error>> {
        Ok(self.turso_client.get_user_db(user_id)?)
    }

    /// Health check for all services
    pub async fn health_check(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Check registry database connection
        self.turso_client.health_check().await?;

        // Check Redis connection
        self.cache_service.health_check().await?;

        log::info!("All services healthy (Turso + Redis)");
        Ok(())
    }

    /// Register a public note in the central registry
    pub async fn register_public_note(
        &self,
        slug: &str,
        note_id: &str,
        owner_id: &str,
        title: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.turso_client
            .register_public_note(slug, note_id, owner_id, title)
            .await?;
        Ok(())
    }

    /// Unregister a public note from the central registry
    pub async fn unregister_public_note(&self, slug: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.turso_client.unregister_public_note(slug).await?;
        Ok(())
    }

    /// Get public note entry by slug
    pub async fn get_public_note_entry(
        &self,
        slug: &str,
    ) -> Result<Option<client::PublicNoteEntry>, Box<dyn std::error::Error>> {
        Ok(self.turso_client.get_public_note_entry(slug).await?)
    }

    /// Register an invitation in the central registry
    pub async fn register_invitation(
        &self,
        token: &str,
        inviter_id: &str,
        note_id: &str,
        invitee_email: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.turso_client
            .register_invitation(token, inviter_id, note_id, invitee_email)
            .await?;
        Ok(())
    }

    /// Unregister an invitation from the central registry
    pub async fn unregister_invitation(&self, token: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.turso_client.unregister_invitation(token).await?;
        Ok(())
    }

    /// Get invitation entry by token
    pub async fn get_invitation_entry(
        &self,
        token: &str,
    ) -> Result<Option<client::InvitationEntry>, Box<dyn std::error::Error>> {
        Ok(self.turso_client.get_invitation_entry(token).await?)
    }

    /// Register a collaborator in the central registry for cross-database access
    pub async fn register_collaborator(
        &self,
        id: &str,
        note_id: &str,
        owner_id: &str,
        collaborator_id: &str,
        role: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.turso_client
            .register_collaborator(id, note_id, owner_id, collaborator_id, role)
            .await?;
        Ok(())
    }

    /// Unregister a collaborator from the central registry
    pub async fn unregister_collaborator(
        &self,
        note_id: &str,
        collaborator_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.turso_client
            .unregister_collaborator(note_id, collaborator_id)
            .await?;
        Ok(())
    }

    /// Get collaborator entry for cross-database note access
    pub async fn get_collaborator_entry(
        &self,
        note_id: &str,
        user_id: &str,
    ) -> Result<Option<client::CollaboratorEntry>, Box<dyn std::error::Error>> {
        Ok(self.turso_client.get_collaborator_entry(note_id, user_id).await?)
    }

    /// Get all notes shared with a user
    pub async fn get_shared_notes_for_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<client::CollaboratorEntry>, Box<dyn std::error::Error>> {
        Ok(self.turso_client.get_shared_notes_for_user(user_id).await?)
    }
}
