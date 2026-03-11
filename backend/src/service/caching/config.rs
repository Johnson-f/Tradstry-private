use crate::turso::redis::ttl;
use crate::turso::schema::TableSchema;
use std::collections::HashMap;

/// Get list of tables that should be cached for users
pub fn get_cacheable_tables(schema_cache: &HashMap<String, TableSchema>) -> Vec<String> {
    // Define tables that should be cached with their TTL settings
    let cacheable_tables = vec![
        ("stocks", ttl::STOCKS_LIST),
        ("options", ttl::OPTIONS_LIST),
        ("trade_notes", ttl::TRADE_NOTES_LIST),
        ("playbook", ttl::PLAYBOOK_LIST),
        ("notebook_notes", ttl::NOTEBOOK_NOTES_LIST),
        ("images", ttl::IMAGES_LIST),
        ("external_calendar_connections", ttl::CALENDAR_EVENTS),
        ("external_calendar_events", ttl::CALENDAR_EVENTS),
        ("notebook_tags", 3600),      // 1 hour
        ("notebook_templates", 3600), // 1 hour
        ("notebook_reminders", 1800), // 30 minutes
        ("calendar_events", ttl::CALENDAR_EVENTS),
        ("public_holidays", ttl::PUBLIC_HOLIDAYS),
    ];

    // Filter to only include tables that exist in our schema
    cacheable_tables
        .into_iter()
        .filter(|(table_name, _)| schema_cache.contains_key(*table_name))
        .map(|(table_name, _)| table_name.to_string())
        .collect()
}

/// Get TTL for a specific table
pub fn get_table_ttl(table_name: &str) -> usize {
    match table_name {
        "stocks" | "options" | "trade_notes" => ttl::STOCKS_LIST,
        "playbook" | "notebook_tags" | "notebook_templates" | "images" => ttl::PLAYBOOK_LIST,
        "notebook_notes" | "notebook_reminders" => ttl::NOTEBOOK_NOTES_LIST,
        "external_calendar_connections" | "external_calendar_events" | "calendar_events" => {
            ttl::CALENDAR_EVENTS
        }
        "public_holidays" => ttl::PUBLIC_HOLIDAYS,
        _ => ttl::STOCKS_LIST, // Default TTL
    }
}
