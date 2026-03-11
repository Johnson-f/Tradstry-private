use resend_rs::{Resend, Result as ResendResult};
use resend_rs::types::CreateEmailBaseOptions;
use std::env;

/// Resend email client wrapper for Tradstry
pub struct EmailClient {
    client: Resend,
    from_email: String,
}

impl EmailClient {
    /// Create new EmailClient from environment variables
    /// Requires: RESEND_API_KEY, RESEND_FROM_EMAIL
    pub fn new() -> Result<Self, EmailConfigError> {
        let api_key = env::var("RESEND_API_KEY")
            .map_err(|_| EmailConfigError::MissingApiKey)?;
        
        let from_email = env::var("RESEND_FROM_EMAIL")
            .unwrap_or_else(|_| "Tradstry <noreply@tradstry.com>".to_string());

        Ok(Self {
            client: Resend::new(&api_key),
            from_email,
        })
    }

    /// Create client with explicit API key
    pub fn with_api_key(api_key: &str, from_email: Option<&str>) -> Self {
        Self {
            client: Resend::new(api_key),
            from_email: from_email
                .unwrap_or("Tradstry <noreply@tradstry.com>")
                .to_string(),
        }
    }

    /// Send a simple HTML email
    pub async fn send_html(
        &self,
        to: &[&str],
        subject: &str,
        html: &str,
    ) -> ResendResult<String> {
        let to_strings: Vec<String> = to.iter().map(|s| s.to_string()).collect();
        let email = CreateEmailBaseOptions::new(self.from_email.clone(), to_strings, subject)
            .with_html(html);

        let response = self.client.emails.send(email).await?;
        Ok(response.id.to_string())
    }

    /// Send a plain text email
    pub async fn send_text(
        &self,
        to: &[&str],
        subject: &str,
        text: &str,
    ) -> ResendResult<String> {
        let to_strings: Vec<String> = to.iter().map(|s| s.to_string()).collect();
        let email = CreateEmailBaseOptions::new(self.from_email.clone(), to_strings, subject)
            .with_text(text);

        let response = self.client.emails.send(email).await?;
        Ok(response.id.to_string())
    }

    /// Send email with both HTML and text fallback
    pub async fn send(
        &self,
        to: &[&str],
        subject: &str,
        html: &str,
        text: Option<&str>,
    ) -> ResendResult<String> {
        let to_strings: Vec<String> = to.iter().map(|s| s.to_string()).collect();
        let mut email = CreateEmailBaseOptions::new(self.from_email.clone(), to_strings, subject)
            .with_html(html);

        if let Some(text_content) = text {
            email = email.with_text(text_content);
        }

        let response = self.client.emails.send(email).await?;
        Ok(response.id.to_string())
    }

    /// Send email with custom from address (override default)
    pub async fn send_from(
        &self,
        from: &str,
        to: &[&str],
        subject: &str,
        html: &str,
    ) -> ResendResult<String> {
        let to_strings: Vec<String> = to.iter().map(|s| s.to_string()).collect();
        let email = CreateEmailBaseOptions::new(from.to_string(), to_strings, subject)
            .with_html(html);

        let response = self.client.emails.send(email).await?;
        Ok(response.id.to_string())
    }

    /// Get the configured from email
    pub fn from_email(&self) -> &str {
        &self.from_email
    }
}

impl Default for EmailClient {
    fn default() -> Self {
        Self::new().expect("Failed to create EmailClient from environment")
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EmailConfigError {
    #[error("RESEND_API_KEY environment variable not set")]
    MissingApiKey,
}
