use serde::{Deserialize, Serialize};

/// Cache statistics structure
#[derive(Debug, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_keys: usize,
    pub memory_usage: usize,
    pub hit_rate: f64,
    pub connected_clients: usize,
}
