mod broadcast;
mod collab;
mod manager;
mod messages;
mod server;

pub use manager::ConnectionManager;
pub use messages::{EventType, WsMessage};
// Re-export message types only where needed to avoid unused warnings
pub use broadcast::{
    broadcast_chat_message,
    broadcast_chat_session,
    broadcast_note_update,
    broadcast_option_update,
    broadcast_playbook_update,
    // Legacy convenience functions
    broadcast_stock_update,
};
pub use collab::collab_ws_handler;
pub use server::ws_handler;
