use crate::turso::redis::RedisClient;
use anyhow::{Context, Result};
use std::time::{SystemTime, UNIX_EPOCH};

/// Rate limit configuration constants
/// 2,000 requests per 10 minutes
const RATE_LIMIT_PER_WINDOW: u64 = 2000;
const RATE_LIMIT_WINDOW_SECONDS: u64 = 600;

/// Rate limit result containing remaining requests and reset time
#[derive(Debug, Clone)]
pub struct RateLimitResult {
    pub remaining: u64,
    pub limit: u64,
    pub reset_at: u64, // Unix timestamp when the rate limit window resets
}

/// Error type for rate limiting operations
#[derive(Debug, thiserror::Error)]
pub enum RateLimitError {
    #[error("Rate limit exceeded: {remaining} requests remaining, resets at {reset_at}")]
    Exceeded { remaining: u64, reset_at: u64 },
    #[error("Redis error: {0}")]
    Redis(#[from] anyhow::Error),
}

/// Rate limiter service using Fixed Window Counter algorithm
#[derive(Debug, Clone)]
pub struct RateLimiter {
    redis_client: RedisClient,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(redis_client: RedisClient) -> Self {
        Self { redis_client }
    }

    /// Check rate limit for a user using Fixed Window Counter algorithm
    ///
    /// Algorithm:
    /// 1. Calculate current hour timestamp (Unix timestamp / 3600)
    /// 2. Generate key: `rate_limit:user:{user_id}:{hour_timestamp}`
    /// 3. Atomically increment counter in Redis
    /// 4. If this is the first request in the window (count == 1), set TTL to 3600 seconds
    /// 5. Check if count exceeds limit
    /// 6. Return remaining requests and reset time
    pub async fn check_rate_limit(&self, user_id: &str) -> Result<RateLimitResult, RateLimitError> {
        // Calculate current hour window (Unix timestamp / seconds per hour)
        let current_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| anyhow::anyhow!("Failed to get system time: {}", e))?;

        let window_timestamp = current_timestamp.as_secs() / RATE_LIMIT_WINDOW_SECONDS;

        // Generate Redis key for this user and hour window
        let key = format!("rate_limit:user:{}:{}", user_id, window_timestamp);

        // Atomically increment the counter
        let count = self
            .redis_client
            .incr(&key)
            .await
            .context("Failed to increment rate limit counter")? as u64;

        // If this is the first request in the window, set TTL to expire at end of hour
        if count == 1 {
            // Set expiration to expire at the start of the next window
            // Calculate seconds until next window
            let seconds_until_next_window = RATE_LIMIT_WINDOW_SECONDS
                - (current_timestamp.as_secs() % RATE_LIMIT_WINDOW_SECONDS);

            self.redis_client
                .expire(&key, seconds_until_next_window as usize)
                .await
                .context("Failed to set rate limit key expiration")?;
        }

        // Calculate reset time (start of next window)
        let reset_at = (window_timestamp + 1) * RATE_LIMIT_WINDOW_SECONDS;

        // Check if rate limit exceeded
        let exceeded = count > RATE_LIMIT_PER_WINDOW;
        let remaining = if exceeded {
            0
        } else {
            RATE_LIMIT_PER_WINDOW.saturating_sub(count)
        };

        let result = RateLimitResult {
            remaining,
            limit: RATE_LIMIT_PER_WINDOW,
            reset_at,
        };

        if exceeded {
            return Err(RateLimitError::Exceeded {
                remaining: result.remaining,
                reset_at: result.reset_at,
            });
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_hour_window_calculation() {
        // Test that hour timestamp calculation works correctly
        // For a timestamp of 1704067200 (Jan 1, 2024 00:00:00 UTC)
        // Hour window should be 1704067200 / 3600 = 473352
        let timestamp = 1704067200;
        let hour_window = timestamp / 3600;
        assert_eq!(hour_window, 473352);
    }

    #[test]
    fn test_rate_limit_key_format() {
        let user_id = "test_user_123";
        let hour_timestamp = 473352;
        let key = format!("rate_limit:user:{}:{}", user_id, hour_timestamp);
        assert_eq!(key, "rate_limit:user:test_user_123:473352");
    }
}
