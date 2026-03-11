use crate::turso::redis::RedisClient;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Get cached data with fallback to database
pub async fn get_or_fetch<T, F, Fut>(
    redis_client: &RedisClient,
    cache_key: &str,
    ttl_seconds: u64,
    fetch_fn: F,
) -> Result<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Clone,
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    // Try to get from cache first
    if let Some(cached_data) = redis_client.get::<T>(cache_key).await? {
        log::debug!("Cache hit for key: {}", cache_key);
        return Ok(cached_data);
    }

    log::debug!("Cache miss for key: {}, fetching from database", cache_key);

    // Fetch from database
    let data = fetch_fn().await?;

    // Store in cache
    redis_client
        .set(cache_key, &data, ttl_seconds as usize)
        .await
        .context("Failed to cache fetched data")?;

    Ok(data)
}
