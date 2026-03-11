use anyhow::Result;
use reqwest::Client;
use serde::Serialize;
use std::time::Duration;

/// HTTP client for communicating with Go SnapTrade microservice
#[derive(Clone)]
pub struct SnapTradeClient {
    base_url: String,
    http: Client,
}

impl SnapTradeClient {
    pub fn new(base_url: String) -> Result<Self> {
        let http = Client::builder().timeout(Duration::from_secs(30)).build()?;

        Ok(Self { base_url, http })
    }

    pub async fn call_go_service<T: Serialize>(
        &self,
        method: &str,
        path: &str,
        body: Option<&T>,
        user_id: &str,
        user_secret: Option<&str>,
    ) -> Result<reqwest::Response> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = match method {
            "GET" => self.http.get(&url),
            "POST" => self.http.post(&url),
            "PUT" => self.http.put(&url),
            "DELETE" => self.http.delete(&url),
            _ => return Err(anyhow::anyhow!("Unsupported method")),
        };

        req = req.header("X-User-Id", user_id);
        req = req.header("Content-Type", "application/json");

        if let Some(secret) = user_secret {
            req = req.header("X-User-Secret", secret);
        }

        if let Some(b) = body {
            req = req.json(b);
        }

        let resp = req.send().await?;
        Ok(resp)
    }
}
