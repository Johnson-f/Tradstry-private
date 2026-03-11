pub mod analysis;
pub mod chat;
pub mod reports;

pub use analysis::configure_ai_analysis_routes;
pub use chat::configure_ai_chat_routes;
pub use reports::configure_ai_reports_routes;
