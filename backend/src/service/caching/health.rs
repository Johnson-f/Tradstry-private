use super::types::CacheStats;
use crate::turso::redis::RedisClient;
use anyhow::Result;

/// Health check for cache service
pub async fn health_check(redis_client: &RedisClient) -> Result<()> {
    redis_client.health_check().await
}

/// Get cache statistics
pub async fn get_cache_stats(_redis_client: &RedisClient) -> Result<CacheStats> {
    // For now, return basic stats since info() method is not available
    let stats = CacheStats {
        total_keys: 0,
        memory_usage: 0,
        hit_rate: 0.0,
        connected_clients: 0,
    };

    Ok(stats)
}
