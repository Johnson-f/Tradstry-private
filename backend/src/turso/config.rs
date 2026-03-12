use serde::{Deserialize, Serialize};
use std::env;

/// Configuration for Turso database connections
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TursoConfig {
    /// Single shared database URL
    pub db_url: String,
    /// Single shared database auth token
    pub db_token: String,
    /// Supabase configuration
    pub supabase: SupabaseConfig,
    /// Cron secret for external sync endpoint
    pub cron_secret: String,
    /// Web Push (VAPID) configuration
    pub web_push: WebPushConfig,
    /// SnapTrade service URL
    pub snaptrade_service_url: String,
}

/// Supabase authentication configuration
#[derive(Debug, Clone)]
pub struct SupabaseConfig {
    pub project_url: String,
    #[allow(dead_code)]
    pub anon_key: String,
    #[allow(dead_code)]
    pub service_role_key: String,
    #[allow(dead_code)]
    pub jwks_url: String,
}


impl TursoConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let supabase_config = SupabaseConfig::from_env()?;
        let web_push_config = WebPushConfig::from_env()?;

        Ok(Self {
            db_url: env::var("DATABASE_URL")
                .map_err(|_| "DATABASE_URL environment variable not set")?,
            db_token: env::var("DATABASE_TOKEN")
                .map_err(|_| "DATABASE_TOKEN environment variable not set")?,
            supabase: supabase_config,
            cron_secret: env::var("CRON_SECRET")
                .map_err(|_| "CRON_SECRET environment variable not set")?,
            web_push: web_push_config,
            snaptrade_service_url: env::var("SNAPTRADE_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
        })
    }
}

impl SupabaseConfig {
    /// Load Supabase configuration from environment variables
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let project_url =
            env::var("SUPABASE_URL").map_err(|_| "SUPABASE_URL environment variable not set")?;
        let anon_key = env::var("SUPABASE_ANON_KEY")
            .map_err(|_| "SUPABASE_ANON_KEY environment variable not set")?;
        let service_role_key = env::var("SUPABASE_SERVICE_ROLE_KEY")
            .map_err(|_| "SUPABASE_SERVICE_ROLE_KEY environment variable not set")?;

        // Supabase JWKS endpoint follows standard format
        // Should be: https://your-project.supabase.co/auth/v1/.well-known/jwks
        let jwks_url = format!("{}/auth/v1/.well-known/jwks", project_url);

        Ok(Self {
            project_url,
            anon_key,
            service_role_key,
            jwks_url,
        })
    }
}


/// Web Push (VAPID) configuration
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct WebPushConfig {
    pub vapid_public_key: String,
    pub vapid_private_key: String,
    pub subject: String,
}

impl WebPushConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            vapid_public_key: env::var("VAPID_PUBLIC_KEY")
                .map_err(|_| "VAPID_PUBLIC_KEY environment variable not set")?,
            vapid_private_key: env::var("VAPID_PRIVATE_KEY")
                .map_err(|_| "VAPID_PRIVATE_KEY environment variable not set")?,
            subject: env::var("WEB_PUSH_SUBJECT")
                .map_err(|_| "WEB_PUSH_SUBJECT environment variable not set")?,
        })
    }
}

/// JWT Claims structure from Supabase Auth
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupabaseClaims {
    pub aud: String,           // "authenticated"
    pub exp: i64,              // Expiration timestamp
    pub iat: i64,              // Issued at timestamp
    pub iss: String,           // Issuer (Supabase URL)
    pub sub: String,           // User UUID
    pub email: Option<String>, // User email
    pub phone: Option<String>, // User phone
    pub role: String,          // "authenticated"
    pub aal: String,           // Authentication assurance level
    pub amr: Vec<AmrEntry>,    // Authentication method reference
    pub session_id: String,    // Session identifier
    pub is_anonymous: Option<bool>,

    // User metadata
    pub user_metadata: Option<serde_json::Value>,
    pub app_metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmrEntry {
    pub method: String,
    pub timestamp: i64,
}
