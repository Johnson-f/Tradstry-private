use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use log::{debug, error, info};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::models::notebook::CalendarConnection;

/// OAuth configuration for Google Calendar
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    pub google_client_id: String,
    pub google_client_secret: String,
    pub google_redirect_uri: String,
}

impl OAuthConfig {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        Ok(Self {
            google_client_id: std::env::var("GOOGLE_CLIENT_ID")
                .unwrap_or_default(),
            google_client_secret: std::env::var("GOOGLE_CLIENT_SECRET")
                .unwrap_or_default(),
            google_redirect_uri: std::env::var("GOOGLE_REDIRECT_URI")
                .unwrap_or_else(|_| "http://localhost:3000/api/calendar/auth/google/callback".to_string()),
        })
    }

    pub fn is_configured(&self) -> bool {
        !self.google_client_id.is_empty() && !self.google_client_secret.is_empty()
    }
}

/// OAuth tokens returned from token exchange
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
    pub token_type: String,
    pub scope: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
    token_type: String,
    scope: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleUserInfo {
    email: String,
    name: Option<String>,
}

/// OAuth service for handling Google authentication
pub struct OAuthService {
    config: OAuthConfig,
    client: Client,
}

impl OAuthService {
    pub fn new(config: OAuthConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self::new(OAuthConfig::from_env()?))
    }

    /// Generate Google OAuth authorization URL
    pub fn get_auth_url(&self, state: &str) -> Result<String> {
        if !self.config.is_configured() {
            anyhow::bail!("Google OAuth is not configured");
        }

        let scopes = [
            "https://www.googleapis.com/auth/calendar.readonly",
            "https://www.googleapis.com/auth/userinfo.email",
        ].join(" ");

        let url = format!(
            "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}&access_type=offline&prompt=consent",
            urlencoding::encode(&self.config.google_client_id),
            urlencoding::encode(&self.config.google_redirect_uri),
            urlencoding::encode(&scopes),
            urlencoding::encode(state)
        );

        Ok(url)
    }

    /// Exchange authorization code for tokens
    pub async fn exchange_code(&self, code: &str) -> Result<(OAuthTokens, String, Option<String>)> {
        info!("Exchanging Google authorization code for tokens");

        let response = self.client
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("client_id", self.config.google_client_id.as_str()),
                ("client_secret", self.config.google_client_secret.as_str()),
                ("code", code),
                ("grant_type", "authorization_code"),
                ("redirect_uri", self.config.google_redirect_uri.as_str()),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            error!("Google token exchange failed: {}", error_text);
            anyhow::bail!("Failed to exchange Google authorization code: {}", error_text);
        }

        let token_response: GoogleTokenResponse = response.json().await?;
        let expires_at = Utc::now() + Duration::seconds(token_response.expires_in);

        let tokens = OAuthTokens {
            access_token: token_response.access_token.clone(),
            refresh_token: token_response.refresh_token.unwrap_or_default(),
            expires_at,
            token_type: token_response.token_type,
            scope: token_response.scope,
        };

        // Fetch user info
        let (email, name) = self.get_user_info(&tokens.access_token).await?;

        Ok((tokens, email, name))
    }

    async fn get_user_info(&self, access_token: &str) -> Result<(String, Option<String>)> {
        let response = self.client
            .get("https://www.googleapis.com/oauth2/v2/userinfo")
            .bearer_auth(access_token)
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to fetch Google user info");
        }

        let user_info: GoogleUserInfo = response.json().await?;
        Ok((user_info.email, user_info.name))
    }

    /// Refresh access token using refresh token
    pub async fn refresh_tokens(&self, refresh_token: &str) -> Result<OAuthTokens> {
        debug!("Refreshing Google access token");

        let response = self.client
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("client_id", self.config.google_client_id.as_str()),
                ("client_secret", self.config.google_client_secret.as_str()),
                ("refresh_token", refresh_token),
                ("grant_type", "refresh_token"),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to refresh Google token: {}", error_text);
        }

        let token_response: GoogleTokenResponse = response.json().await?;
        let expires_at = Utc::now() + Duration::seconds(token_response.expires_in);

        Ok(OAuthTokens {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token.unwrap_or_else(|| refresh_token.to_string()),
            expires_at,
            token_type: token_response.token_type,
            scope: token_response.scope,
        })
    }

    /// Check if token needs refresh (expires within 5 minutes)
    pub fn needs_refresh(connection: &CalendarConnection) -> bool {
        connection.token_expires_at < Utc::now() + Duration::minutes(5)
    }
}
