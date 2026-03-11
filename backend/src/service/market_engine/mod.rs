pub mod client_helper;
pub mod historical;
pub mod hours;
pub mod news;
pub mod quotes;
pub mod search;
pub mod sectors;
pub mod stream_service;
// Additional modules
pub mod analysts;
pub mod caching;
pub mod calendar;
pub mod financials;
pub mod holders;
pub mod indices;
pub mod movers;
pub mod transcripts;

// Re-export modules with aliases for backward compatibility
pub use calendar as earnings_calendar;
pub use transcripts as earnings_transcripts;

// Re-export stream service for easy access
pub use stream_service::MarketStreamService;
