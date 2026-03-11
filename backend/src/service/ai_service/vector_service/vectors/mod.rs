// Vectors module - handles vectorization for different data types
pub mod chat;
pub mod notebook;
pub mod playbook;
pub mod trade;

// Re-export commonly used types
pub use chat::ChatVectorization;
// pub use trade::TradeVectorization; // Commented out until used
