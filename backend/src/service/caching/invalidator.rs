use crate::turso::redis::RedisClient;
use anyhow::{Context, Result};

/// Invalidate cache keys matching a pattern
pub async fn invalidate_pattern(redis_client: &RedisClient, pattern: &str) -> Result<usize> {
    let deleted_count = redis_client
        .del_pattern(pattern)
        .await
        .context("Failed to invalidate cache pattern")?;

    log::info!(
        "Invalidated {} cache keys matching pattern: {}",
        deleted_count,
        pattern
    );
    Ok(deleted_count)
}

/// Invalidate all user database cache
pub async fn invalidate_user_cache(redis_client: &RedisClient, user_id: &str) -> Result<usize> {
    let pattern = format!("db:{}:*", user_id);
    invalidate_pattern(redis_client, &pattern).await
}

/// Invalidate analytics cache for a user database
pub async fn invalidate_user_analytics(redis_client: &RedisClient, user_id: &str) -> Result<usize> {
    let pattern = format!("analytics:db:{}:*", user_id);
    invalidate_pattern(redis_client, &pattern).await
}

/// Invalidate cache for a specific table in user database
pub async fn invalidate_table_cache(
    redis_client: &RedisClient,
    user_id: &str,
    table_name: &str,
) -> Result<usize> {
    let pattern = format!("db:{}:{}:*", user_id, table_name);
    invalidate_pattern(redis_client, &pattern).await
}
