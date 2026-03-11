use crate::turso::redis::RedisClient;
use crate::turso::schema::TableSchema;
use anyhow::Result;
use libsql::Connection;
use std::collections::HashMap;

use super::analytics::preload_analytics;
use super::config::{get_cacheable_tables, get_table_ttl};
use super::converter::row_to_json;
use super::query_builder::build_select_query;

/// Preload all user data into cache after login using dynamic schema discovery
pub async fn preload_user_data(
    redis_client: &RedisClient,
    schema_cache: &HashMap<String, TableSchema>,
    conn: &Connection,
    user_id: &str,
) -> Result<()> {
    log::info!(
        "Starting dynamic cache preload for user database: {}",
        user_id
    );

    // Get all tables that should be cached for this user
    let cacheable_tables = get_cacheable_tables(schema_cache);

    // Preload data from all cacheable tables in parallel
    let mut preload_tasks = Vec::new();

    for table_name in &cacheable_tables {
        let table_name = table_name.clone();
        let redis_client = redis_client.clone();
        let conn = conn.clone();
        let user_id = user_id.to_string();

        let task = tokio::spawn(async move {
            preload_table_data(&redis_client, &conn, &user_id, &table_name).await
        });

        preload_tasks.push(task);
    }

    // Wait for all preload tasks to complete
    let mut total_records = 0;
    for task in preload_tasks {
        match task.await {
            Ok(Ok(record_count)) => {
                total_records += record_count;
            }
            Ok(Err(e)) => {
                log::warn!("Failed to preload table data: {}", e);
            }
            Err(e) => {
                log::warn!("Preload task panicked: {}", e);
            }
        }
    }

    // Pre-compute and cache analytics for known analytics tables
    preload_analytics(redis_client, schema_cache, conn, user_id).await?;

    log::info!(
        "Dynamic cache preload completed for user database: {} - {} total records cached",
        user_id,
        total_records
    );

    Ok(())
}

/// Preload data from a specific table
pub async fn preload_table_data(
    redis_client: &RedisClient,
    conn: &Connection,
    user_id: &str,
    table_name: &str,
) -> Result<usize> {
    log::debug!("Preloading data for table: {}", table_name);

    // Build dynamic query based on table structure (no user_id needed since each user has their own DB)
    let query = build_select_query(table_name, user_id);

    // Execute query and get results with better error handling
    let stmt = match conn.prepare(&query).await {
        Ok(stmt) => stmt,
        Err(e) => {
            log::warn!("Failed to prepare query for table {}: {}", table_name, e);
            return Ok(0); // Return 0 instead of erroring
        }
    };

    let mut rows = match stmt.query(libsql::params![]).await {
        Ok(rows) => rows,
        Err(e) => {
            log::warn!("Failed to execute query for table {}: {}", table_name, e);
            return Ok(0); // Return 0 instead of erroring
        }
    };

    let mut records = Vec::new();
    loop {
        match rows.next().await {
            Ok(Some(row)) => {
                match row_to_json(&row, table_name) {
                    Ok(record) => records.push(record),
                    Err(e) => {
                        log::warn!(
                            "Failed to convert row to JSON for table {}: {}",
                            table_name,
                            e
                        );
                        // Continue with other rows
                    }
                }
            }
            Ok(None) => break,
            Err(e) => {
                log::warn!("Error reading row from table {}: {}", table_name, e);
                break; // Continue with what we have
            }
        }
    }

    if records.is_empty() {
        log::debug!("No records found for table: {}", table_name);
        return Ok(0);
    }

    // Cache the data (user_id represents the user's database)
    let cache_key = format!("db:{}:{}:all", user_id, table_name);
    let ttl_seconds = get_table_ttl(table_name);

    match redis_client.set(&cache_key, &records, ttl_seconds).await {
        Ok(_) => {
            log::debug!("Cached {} records for table: {}", records.len(), table_name);
            Ok(records.len())
        }
        Err(e) => {
            log::warn!("Failed to cache data for table {}: {}", table_name, e);
            Ok(0) // Return 0 instead of erroring
        }
    }
}
