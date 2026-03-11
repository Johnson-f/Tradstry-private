#![allow(dead_code)]

use crate::turso::redis::RedisClient;
use crate::turso::schema::{TableSchema, get_expected_schema};
use anyhow::Result;
use libsql::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

mod analytics;
mod cache_operations;
mod config;
mod converter;
mod health;
mod invalidator;
mod preloader;
mod query_builder;
mod types;

pub use types::CacheStats;

/// Cache service for managing Redis operations with dynamic schema discovery
#[derive(Debug, Clone)]
pub struct CacheService {
    redis_client: RedisClient,
    schema_cache: HashMap<String, TableSchema>,
}

impl CacheService {
    /// Create a new cache service
    pub fn new(redis_client: RedisClient) -> Self {
        Self {
            redis_client,
            schema_cache: HashMap::new(),
        }
    }

    /// Initialize cache service with schema information
    pub async fn initialize(&mut self) -> Result<()> {
        log::info!("Initializing cache service with schema discovery");

        // Load expected schema from schema.rs
        let expected_schema = get_expected_schema();

        // Cache schema information
        for table_schema in expected_schema {
            self.schema_cache
                .insert(table_schema.name.clone(), table_schema);
        }

        log::info!(
            "Cache service initialized with {} tables",
            self.schema_cache.len()
        );
        Ok(())
    }

    /// Preload all user data into cache after login using dynamic schema discovery
    pub async fn preload_user_data(&self, conn: &Connection, user_id: &str) -> Result<()> {
        preloader::preload_user_data(&self.redis_client, &self.schema_cache, conn, user_id).await
    }

    /// Get cached data with fallback to database
    pub async fn get_or_fetch<T, F, Fut>(
        &self,
        cache_key: &str,
        ttl_seconds: u64,
        fetch_fn: F,
    ) -> Result<T>
    where
        T: Serialize + for<'de> Deserialize<'de> + Clone,
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        cache_operations::get_or_fetch(&self.redis_client, cache_key, ttl_seconds, fetch_fn).await
    }

    /// Invalidate cache keys matching a pattern
    pub async fn invalidate_pattern(&self, pattern: &str) -> Result<usize> {
        invalidator::invalidate_pattern(&self.redis_client, pattern).await
    }

    /// Invalidate all user database cache
    #[allow(dead_code)]
    pub async fn invalidate_user_cache(&self, user_id: &str) -> Result<usize> {
        invalidator::invalidate_user_cache(&self.redis_client, user_id).await
    }

    /// Invalidate analytics cache for a user database
    pub async fn invalidate_user_analytics(&self, user_id: &str) -> Result<usize> {
        invalidator::invalidate_user_analytics(&self.redis_client, user_id).await
    }

    /// Invalidate cache for a specific table in user database
    pub async fn invalidate_table_cache(&self, user_id: &str, table_name: &str) -> Result<usize> {
        invalidator::invalidate_table_cache(&self.redis_client, user_id, table_name).await
    }

    /// Get cache statistics
    #[allow(dead_code)]
    pub async fn get_cache_stats(&self) -> Result<CacheStats> {
        health::get_cache_stats(&self.redis_client).await
    }

    /// Health check for cache service
    pub async fn health_check(&self) -> Result<()> {
        health::health_check(&self.redis_client).await
    }

    /// Get list of cached tables for a user database
    #[allow(dead_code)]
    pub async fn get_cached_tables(&self, _user_id: &str) -> Result<Vec<String>> {
        // This would require implementing keys() method in RedisClient
        // For now, return the expected tables
        Ok(config::get_cacheable_tables(&self.schema_cache))
    }
}
