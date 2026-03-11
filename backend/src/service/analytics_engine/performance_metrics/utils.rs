/// Helper function to safely extract f64 from libsql::Value
pub fn get_f64_value(row: &libsql::Row, index: i32) -> f64 {
    match row.get::<libsql::Value>(index) {
        Ok(libsql::Value::Integer(i)) => i as f64,
        Ok(libsql::Value::Real(f)) => f,
        Ok(libsql::Value::Null) => 0.0,
        _ => 0.0,
    }
}

/// Helper function to safely extract i64 from libsql::Value
pub fn get_i64_value(row: &libsql::Row, index: i32) -> i64 {
    match row.get::<libsql::Value>(index) {
        Ok(libsql::Value::Integer(i)) => i,
        Ok(libsql::Value::Null) => 0,
        _ => 0,
    }
}
