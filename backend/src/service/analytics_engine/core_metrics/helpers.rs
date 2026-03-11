/// Helper function to safely extract f64 from libsql::Value
pub(crate) fn get_f64_value(row: &libsql::Row, index: usize) -> f64 {
    match row.get::<libsql::Value>(index as i32) {
        Ok(libsql::Value::Integer(i)) => i as f64,
        Ok(libsql::Value::Real(f)) => f,
        Ok(libsql::Value::Null) => 0.0,
        _ => 0.0,
    }
}

/// Helper function to safely extract i64 from libsql::Value
pub(crate) fn get_i64_value(row: &libsql::Row, index: usize) -> i64 {
    match row.get::<libsql::Value>(index as i32) {
        Ok(libsql::Value::Integer(i)) => i,
        Ok(libsql::Value::Null) => 0,
        _ => 0,
    }
}

/// Helper function to safely extract Option<f64> from libsql::Value
pub(crate) fn get_optional_f64_value(row: &libsql::Row, index: usize) -> Option<f64> {
    match row.get::<libsql::Value>(index as i32) {
        Ok(libsql::Value::Integer(i)) => Some(i as f64),
        Ok(libsql::Value::Real(f)) => Some(f),
        Ok(libsql::Value::Null) => None,
        _ => None,
    }
}
