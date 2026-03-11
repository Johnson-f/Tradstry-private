// Vector service module - handles vector embeddings and storage
pub mod client;
pub mod formatter;
pub mod qdrant;
pub mod vec_generation;
pub mod vectors;

// Re-export commonly used types
pub use client::VoyagerClient;
pub use qdrant::QdrantDocumentClient;
pub use vec_generation::TradeVectorService;
pub use vectors::chat::ChatVectorization;
pub use vectors::notebook::NotebookVectorization;
pub use vectors::playbook::PlaybookVectorization;
pub use vectors::trade::TradeVectorization;
