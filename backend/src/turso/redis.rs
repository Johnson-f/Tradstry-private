#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Redis configuration loaded from environment variables
#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub url: String,
    pub token: String,
}

impl RedisConfig {
    /// Load Redis configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let url = std::env::var("UPSTASH_REDIS_REST_URL")
            .context("UPSTASH_REDIS_REST_URL environment variable not set")?;
        let token = std::env::var("UPSTASH_REDIS_REST_TOKEN")
            .context("UPSTASH_REDIS_REST_TOKEN environment variable not set")?;

        Ok(Self { url, token })
    }
}

/// Redis client wrapper using Upstash REST API
#[derive(Debug, Clone)]
pub struct RedisClient {
    client: reqwest::Client,
    base_url: String,
    token: String,
}

impl RedisClient {
    /// Create a new Redis client with HTTP client
    pub async fn new(config: RedisConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        Ok(Self {
            client,
            base_url: config.url,
            token: config.token,
        })
    }

    /// Get a value from Redis cache
    pub async fn get<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        let response = self
            .client
            .get(format!("{}/get/{}", self.base_url, key))
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await?;

        if response.status().is_success() {
            let result: UpstashResponse = response.json().await?;
            if result.result.is_null() {
                return Ok(None);
            }
            // Upstash may return our stored JSON as a raw string. Handle both cases.
            let data: T = match &result.result {
                serde_json::Value::String(s) => serde_json::from_str(s)?,
                other => serde_json::from_value(other.clone())?,
            };
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }

    /// Set a value in Redis cache with TTL
    pub async fn set<T>(&self, key: &str, value: &T, ttl_seconds: usize) -> Result<()>
    where
        T: Serialize,
    {
        let serialized = serde_json::to_string(value)?;

        self.client
            .post(format!("{}/setex/{}/{}", self.base_url, key, ttl_seconds))
            .header("Authorization", format!("Bearer {}", self.token))
            .body(serialized)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    /// Increment a key's value atomically (returns the new value)
    pub async fn incr(&self, key: &str) -> Result<i64> {
        let response = self
            .client
            .post(format!("{}/incr/{}", self.base_url, key))
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await?;

        if response.status().is_success() {
            let result: UpstashResponse = response.json().await?;
            // Upstash INCR returns a number
            let value = match result.result {
                serde_json::Value::Number(n) => n
                    .as_i64()
                    .ok_or_else(|| anyhow::anyhow!("Invalid number format"))?,
                serde_json::Value::String(s) => s
                    .parse()
                    .context("Failed to parse INCR result as integer")?,
                _ => return Err(anyhow::anyhow!("Unexpected response format from INCR")),
            };
            Ok(value)
        } else {
            Err(anyhow::anyhow!(
                "INCR operation failed with status: {}",
                response.status()
            ))
        }
    }

    /// Delete a key from Redis
    pub async fn del(&self, key: &str) -> Result<()> {
        self.client
            .post(format!("{}/del/{}", self.base_url, key))
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    /// Set expiration time for a key
    pub async fn expire(&self, key: &str, ttl_seconds: usize) -> Result<()> {
        self.client
            .post(format!("{}/expire/{}/{}", self.base_url, key, ttl_seconds))
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    /// Delete all keys matching a pattern
    pub async fn del_pattern(&self, pattern: &str) -> Result<usize> {
        // Get keys matching pattern
        let response = self
            .client
            .get(format!("{}/keys/{}", self.base_url, pattern))
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await?;

        let result: UpstashResponse = response.json().await?;

        if let Some(keys) = result.result.as_array() {
            let count = keys.len();

            // Delete each key
            for key in keys {
                if let Some(key_str) = key.as_str() {
                    self.del(key_str).await.ok(); // Ignore individual errors
                }
            }

            Ok(count)
        } else {
            Ok(0)
        }
    }

    /// Health check for Redis connection
    pub async fn health_check(&self) -> Result<()> {
        self.client
            .get(format!("{}/ping", self.base_url))
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct UpstashResponse {
    result: serde_json::Value,
}

/// Cache key patterns for consistent key generation
pub mod cache_keys {
    #[allow(dead_code)]
    pub fn user_data(user_id: &str, table: &str) -> String {
        format!("db:{}:{}:all", user_id, table)
    }

    #[allow(dead_code)]
    pub fn user_list(user_id: &str, table: &str, query_hash: &str) -> String {
        format!("db:{}:{}:list:{}", user_id, table, query_hash)
    }

    #[allow(dead_code)]
    pub fn user_item(user_id: &str, table: &str, id: &str) -> String {
        format!("db:{}:{}:item:{}", user_id, table, id)
    }

    #[allow(dead_code)]
    pub fn analytics(user_id: &str, table: &str, time_range: &str) -> String {
        format!("analytics:db:{}:{}:{}", user_id, table, time_range)
    }

    #[allow(dead_code)]
    pub fn analytics_metric(user_id: &str, table: &str, metric: &str) -> String {
        format!("analytics:db:{}:{}:{}", user_id, table, metric)
    }
}

/// TTL constants for different data types
pub mod ttl {
    pub const STOCKS_LIST: usize = 1800; // 30 minutes
    pub const OPTIONS_LIST: usize = 1800; // 30 minutes
    pub const TRADE_NOTES_LIST: usize = 1800; // 30 minutes
    pub const PLAYBOOK_LIST: usize = 3600; // 1 hour
    pub const NOTEBOOK_NOTES_LIST: usize = 600; // 10 minutes
    pub const IMAGES_LIST: usize = 3600; // 1 hour
    pub const ANALYTICS: usize = 900; // 15 minutes
    #[allow(dead_code)]
    pub const ANALYTICS_PNL: usize = 1800; // 30 minutes
    pub const CALENDAR_EVENTS: usize = 300; // 5 minutes
    pub const PUBLIC_HOLIDAYS: usize = 86400; // 24 hours
    #[allow(dead_code)]
    pub const MARKET_DATA: usize = 120; // 2 minutes
    #[allow(dead_code)]
    pub const MARKET_MOVERS: usize = 300; // 5 minutes
}
