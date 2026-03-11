use actix_web::http::header::HeaderMap;
use anyhow::Result;
use base64::{Engine as _, engine::general_purpose};
use chrono;
use reqwest;
use serde_json::Value;

use super::config::{SupabaseClaims, SupabaseConfig};

/// Custom error types for authentication
#[derive(Debug)]
pub enum AuthError {
    InvalidToken,
    #[allow(dead_code)]
    JWKSFetchError,
    #[allow(dead_code)]
    JWKSParseError,
    #[allow(dead_code)]
    KeyNotFound,
    TokenExpired,
    InvalidIssuer,
    #[allow(dead_code)]
    NetworkError,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::InvalidToken => write!(f, "Invalid token"),
            AuthError::JWKSFetchError => write!(f, "Failed to fetch JWKS"),
            AuthError::JWKSParseError => write!(f, "Failed to parse JWKS"),
            AuthError::KeyNotFound => write!(f, "Key not found in JWKS"),
            AuthError::TokenExpired => write!(f, "Token expired"),
            AuthError::InvalidIssuer => write!(f, "Invalid token issuer"),
            AuthError::NetworkError => write!(f, "Network error"),
        }
    }
}

impl std::error::Error for AuthError {}

/// Supabase JWT authentication
pub struct SupabaseAuth {
    config: SupabaseConfig,
}

impl SupabaseAuth {
    pub fn new(config: SupabaseConfig) -> Self {
        Self { config }
    }

    /// Extract and validate Supabase JWT from request headers
    #[allow(dead_code)]
    pub async fn extract_and_validate_token(
        &self,
        headers: &HeaderMap,
    ) -> Result<SupabaseClaims, AuthError> {
        let auth_header = headers
            .get("authorization")
            .ok_or(AuthError::InvalidToken)?
            .to_str()
            .map_err(|_| AuthError::InvalidToken)?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(AuthError::InvalidToken)?;

        self.validate_token(token).await
    }

    /// Validate Supabase JWT token using Supabase API
    pub async fn validate_token(&self, token: &str) -> Result<SupabaseClaims, AuthError> {
        // First decode and validate basic JWT structure
        let claims = decode_jwt_payload::<SupabaseClaims>(token)?;

        // Validate issuer
        if !claims.iss.starts_with(&self.config.project_url) {
            return Err(AuthError::InvalidIssuer);
        }

        // Check expiration
        let now = chrono::Utc::now().timestamp();
        if claims.exp < now {
            return Err(AuthError::TokenExpired);
        }

        // Validate token with Supabase API
        self.validate_with_supabase_api(token).await?;

        Ok(claims)
    }

    /// Validate JWT token by calling Supabase user API
    async fn validate_with_supabase_api(&self, token: &str) -> Result<(), AuthError> {
        use std::time::Duration;

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30)) // Increased from 10s to 30s
            .build()
            .map_err(|_| AuthError::NetworkError)?;

        let url = format!("{}/auth/v1/user", self.config.project_url);

        log::debug!("Validating token with Supabase API at: {}", url);

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("apikey", &self.config.anon_key)
            .send()
            .await
            .map_err(|e| {
                // Only log network errors at warn level, not error level, to reduce noise
                log::warn!(
                    "Network error validating token with Supabase (may be temporary): {}",
                    e
                );
                AuthError::NetworkError
            })?;

        let status = response.status();
        let response_body = response.text().await.unwrap_or_default();

        log::debug!(
            "Supabase validation response: {} - {}",
            status,
            response_body
        );

        if status.is_success() {
            log::debug!("Token validated successfully with Supabase API");
            Ok(())
        } else {
            log::warn!("Token validation failed: {} - {}", status, response_body);
            Err(AuthError::InvalidToken)
        }
    }
}

// Helper functions for JWT processing

/// Decode JWT header without verification
#[allow(dead_code)]
fn decode_jwt_header(token: &str) -> Result<Value, AuthError> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(AuthError::InvalidToken);
    }

    let header_b64 = parts[0];
    let header_bytes = general_purpose::URL_SAFE_NO_PAD
        .decode(header_b64)
        .map_err(|_| AuthError::InvalidToken)?;

    let header: Value =
        serde_json::from_slice(&header_bytes).map_err(|_| AuthError::InvalidToken)?;

    Ok(header)
}

/// Decode JWT payload without verification
fn decode_jwt_payload<T: serde::de::DeserializeOwned>(token: &str) -> Result<T, AuthError> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(AuthError::InvalidToken);
    }

    let payload_b64 = parts[1];
    let payload_bytes = general_purpose::URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|_| AuthError::InvalidToken)?;

    let payload: T = serde_json::from_slice(&payload_bytes).map_err(|_| AuthError::InvalidToken)?;

    Ok(payload)
}

// Removed JWKS-related functions as Supabase doesn't expose public JWKS endpoints
// Using Supabase API validation instead

/// Get user ID from Supabase claims
pub fn get_supabase_user_id(claims: &SupabaseClaims) -> String {
    claims.sub.clone()
}

/// Validate Supabase JWT token for Actix-Web (no caching)
pub async fn validate_supabase_jwt_token(
    token: &str,
    config: &SupabaseConfig,
) -> Result<SupabaseClaims, AuthError> {
    log::debug!("Validating JWT token with Supabase (no caching)");
    let supabase_auth = SupabaseAuth::new(config.clone());
    let claims = supabase_auth.validate_token(token).await?;
    log::debug!("JWT validation successful for user_id={}", claims.sub);
    Ok(claims)
}

/// Validate JWT token from query parameter (for WebSocket connections)
pub async fn validate_jwt_token_from_query(token: &str) -> Result<SupabaseClaims, AuthError> {
    // Decode and validate basic JWT structure
    let claims = decode_jwt_payload::<SupabaseClaims>(token)?;

    // Check expiration
    let now = chrono::Utc::now().timestamp();
    if claims.exp < now {
        return Err(AuthError::TokenExpired);
    }

    Ok(claims)
}
