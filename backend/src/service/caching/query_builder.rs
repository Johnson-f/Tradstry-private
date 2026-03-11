use libsql::Connection;

/// Check if a table has a specific column
pub async fn table_has_column(conn: &Connection, table_name: &str, column_name: &str) -> bool {
    let query = format!("PRAGMA table_info({})", table_name);

    if let Ok(stmt) = conn.prepare(&query).await
        && let Ok(mut rows) = stmt.query(libsql::params![]).await
    {
        loop {
            match rows.next().await {
                Ok(Some(row)) => {
                    if let Ok(col_name) = row.get::<String>(1) {
                        // Column name is at index 1
                        if col_name == column_name {
                            return true;
                        }
                    }
                }
                Ok(None) => break,
                Err(_) => break,
            }
        }
    }

    false
}

/// Build dynamic SELECT query for a table with column validation
pub async fn build_select_query_with_validation(
    conn: &Connection,
    table_name: &str,
    _user_id: &str,
) -> String {
    // Check for common ordering columns and use the first one found
    let ordering_columns = vec![
        ("created_at", "DESC"),
        ("last_synced_at", "DESC"),
        ("updated_at", "DESC"),
        ("holiday_date", "ASC"),
        ("start_date", "ASC"),
        ("id", "ASC"),
    ];

    for (column, order) in ordering_columns {
        if table_has_column(conn, table_name, column).await {
            return format!("SELECT * FROM {} ORDER BY {} {}", table_name, column, order);
        }
    }

    // Fallback to no ordering
    format!("SELECT * FROM {}", table_name)
}

/// Build dynamic SELECT query for a table (legacy method for backward compatibility)
pub fn build_select_query(table_name: &str, _user_id: &str) -> String {
    match table_name {
        // Tables with created_at column for ordering
        "stocks"
        | "options"
        | "trade_notes"
        | "playbook"
        | "notebook_notes"
        | "images"
        | "external_calendar_connections"
        | "notebook_tags"
        | "notebook_templates"
        | "notebook_reminders"
        | "calendar_events"
        | "user_profile" => {
            format!("SELECT * FROM {} ORDER BY created_at DESC", table_name)
        }
        // Tables with last_synced_at column for ordering
        "external_calendar_events" => {
            format!("SELECT * FROM {} ORDER BY last_synced_at DESC", table_name)
        }
        // Tables with specific ordering
        "public_holidays" => {
            format!("SELECT * FROM {} ORDER BY holiday_date ASC", table_name)
        }
        // Tables with different ordering
        "replicache_clients" | "replicache_space_version" => {
            format!("SELECT * FROM {} ORDER BY updated_at DESC", table_name)
        }
        // Default case - just select all
        _ => {
            format!("SELECT * FROM {}", table_name)
        }
    }
}
