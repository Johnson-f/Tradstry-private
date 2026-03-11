use anyhow::{Context, Result};
use libsql::Row;

/// Convert database row to JSON for caching
pub fn row_to_json(row: &Row, table_name: &str) -> Result<serde_json::Value> {
    let mut record = serde_json::Map::new();

    // Get column count
    let column_count = row.column_count();

    for i in 0..column_count {
        let column_name = row
            .column_name(i)
            .context(format!("Failed to get column name for index {}", i))?;

        // Get value based on column type
        let value = get_column_value(row, i as usize, table_name, column_name)?;
        record.insert(column_name.to_string(), value);
    }

    Ok(serde_json::Value::Object(record))
}

/// Get column value with proper type handling - safer approach
pub fn get_column_value(
    row: &Row,
    index: usize,
    _table_name: &str,
    column_name: &str,
) -> Result<serde_json::Value> {
    // Use libsql's Value type to safely handle all types
    use libsql::Value;

    // Get the raw value first to avoid panics
    let raw_value = match row.get_value(index as i32) {
        Ok(value) => value,
        Err(e) => {
            log::warn!(
                "Failed to get value for column {} at index {}: {}",
                column_name,
                index,
                e
            );
            return Ok(serde_json::Value::Null);
        }
    };

    // Convert based on the actual SQLite value type
    let json_value = match raw_value {
        Value::Null => serde_json::Value::Null,
        Value::Integer(i) => {
            // Handle boolean columns that are stored as integers
            if column_name.contains("is_")
                || column_name == "deleted"
                || column_name.contains("active")
                || column_name.contains("synced")
            {
                serde_json::Value::Bool(i != 0)
            } else {
                serde_json::Value::Number(serde_json::Number::from(i))
            }
        }
        Value::Real(f) => match serde_json::Number::from_f64(f) {
            Some(num) => serde_json::Value::Number(num),
            None => {
                log::warn!("Invalid float value for column {}: {}", column_name, f);
                serde_json::Value::Null
            }
        },
        Value::Text(s) => {
            // Try to parse text values based on column name patterns
            if column_name.contains("_id") || column_name == "id" {
                // ID columns might be integers stored as text
                if let Ok(int_val) = s.parse::<i64>() {
                    serde_json::Value::Number(serde_json::Number::from(int_val))
                } else {
                    serde_json::Value::String(s)
                }
            } else if column_name.contains("price")
                || column_name.contains("amount")
                || column_name.contains("quantity")
                || column_name.contains("size")
            {
                // Numeric columns that might be stored as text
                if let Ok(float_val) = s.parse::<f64>() {
                    match serde_json::Number::from_f64(float_val) {
                        Some(num) => serde_json::Value::Number(num),
                        None => serde_json::Value::String(s),
                    }
                } else if let Ok(int_val) = s.parse::<i64>() {
                    serde_json::Value::Number(serde_json::Number::from(int_val))
                } else {
                    serde_json::Value::String(s)
                }
            } else if column_name.contains("is_") || column_name == "deleted" {
                // Boolean columns that might be stored as text
                match s.to_lowercase().as_str() {
                    "true" | "1" | "yes" => serde_json::Value::Bool(true),
                    "false" | "0" | "no" => serde_json::Value::Bool(false),
                    _ => {
                        if let Ok(int_val) = s.parse::<i64>() {
                            serde_json::Value::Bool(int_val != 0)
                        } else {
                            serde_json::Value::String(s)
                        }
                    }
                }
            } else {
                serde_json::Value::String(s)
            }
        }
        Value::Blob(b) => {
            // Convert blob to base64 string for JSON serialization
            use base64::Engine;
            let base64_string = base64::engine::general_purpose::STANDARD.encode(&b);
            serde_json::Value::String(format!(
                "data:application/octet-stream;base64,{}",
                base64_string
            ))
        }
    };

    Ok(json_value)
}
