use super::{ConnectionManager, messages::*};
use actix_web::web::Data;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Resource type for broadcast operations
#[derive(Debug, Clone, Copy)]
pub enum ResourceType {
    Stock,
    Option,
    Note,
    Playbook,
    ChatMessage,
    ChatSession,
}

impl ResourceType {
    /// Convert resource type and action to EventType
    fn to_event_type(self, action: &str) -> Option<EventType> {
        match (self, action) {
            (ResourceType::Stock, "created") => Some(EventType::StockCreated),
            (ResourceType::Stock, "updated") => Some(EventType::StockUpdated),
            (ResourceType::Stock, "deleted") => Some(EventType::StockDeleted),

            (ResourceType::Option, "created") => Some(EventType::OptionCreated),
            (ResourceType::Option, "updated") => Some(EventType::OptionUpdated),
            (ResourceType::Option, "deleted") => Some(EventType::OptionDeleted),

            (ResourceType::Note, "created") => Some(EventType::NoteCreated),
            (ResourceType::Note, "updated") => Some(EventType::NoteUpdated),
            (ResourceType::Note, "deleted") => Some(EventType::NoteDeleted),

            (ResourceType::Playbook, "created") => Some(EventType::PlaybookCreated),
            (ResourceType::Playbook, "updated") => Some(EventType::PlaybookUpdated),
            (ResourceType::Playbook, "deleted") => Some(EventType::PlaybookDeleted),

            (ResourceType::ChatMessage, "created") => Some(EventType::ChatMessageCreated),
            (ResourceType::ChatMessage, "updated") => Some(EventType::ChatMessageUpdated),

            (ResourceType::ChatSession, "created") => Some(EventType::ChatSessionCreated),
            (ResourceType::ChatSession, "updated") => Some(EventType::ChatSessionUpdated),
            (ResourceType::ChatSession, "deleted") => Some(EventType::ChatSessionDeleted),

            _ => None,
        }
    }
}

/// Generic broadcast function for any resource type
pub async fn broadcast_resource_update(
    manager: Data<Arc<Mutex<ConnectionManager>>>,
    user_id: &str,
    resource_type: ResourceType,
    action: &str,
    data: impl Serialize,
) {
    if let Some(event_type) = resource_type.to_event_type(action) {
        broadcast_to_user(manager, user_id, event_type, data).await;
    }
}

/// Broadcast a message to all connections for a specific user
pub async fn broadcast_to_user(
    manager: Data<Arc<Mutex<ConnectionManager>>>,
    user_id: &str,
    event: EventType,
    data: impl Serialize,
) {
    let envelope = WsMessage::new(
        event,
        serde_json::to_value(data).unwrap_or(serde_json::Value::Null),
    );

    let manager = manager.as_ref().clone();
    let user_id_owned = user_id.to_string();
    tokio::spawn(async move {
        let manager = manager.lock().await;
        manager.broadcast_to_user(&user_id_owned, envelope);
    });
}

// Legacy convenience functions for backward compatibility
// These now delegate to the generic broadcast_resource_update function

/// Broadcast a stock update
#[inline]
pub async fn broadcast_stock_update(
    manager: Data<Arc<Mutex<ConnectionManager>>>,
    user_id: &str,
    action: &str,
    stock: impl Serialize,
) {
    broadcast_resource_update(manager, user_id, ResourceType::Stock, action, stock).await;
}

/// Broadcast an option update
#[inline]
pub async fn broadcast_option_update(
    manager: Data<Arc<Mutex<ConnectionManager>>>,
    user_id: &str,
    action: &str,
    option: impl Serialize,
) {
    broadcast_resource_update(manager, user_id, ResourceType::Option, action, option).await;
}

/// Broadcast a note update
#[inline]
#[allow(dead_code)]
pub async fn broadcast_note_update(
    manager: Data<Arc<Mutex<ConnectionManager>>>,
    user_id: &str,
    action: &str,
    note: impl Serialize,
) {
    broadcast_resource_update(manager, user_id, ResourceType::Note, action, note).await;
}

/// Broadcast a playbook update
#[inline]
#[allow(dead_code)]
pub async fn broadcast_playbook_update(
    manager: Data<Arc<Mutex<ConnectionManager>>>,
    user_id: &str,
    action: &str,
    playbook: impl Serialize,
) {
    broadcast_resource_update(manager, user_id, ResourceType::Playbook, action, playbook).await;
}

/// Broadcast a chat message update
#[inline]
pub async fn broadcast_chat_message(
    manager: Data<Arc<Mutex<ConnectionManager>>>,
    user_id: &str,
    action: &str,
    message: impl Serialize,
) {
    broadcast_resource_update(manager, user_id, ResourceType::ChatMessage, action, message).await;
}

/// Broadcast a chat session update
#[inline]
pub async fn broadcast_chat_session(
    manager: Data<Arc<Mutex<ConnectionManager>>>,
    user_id: &str,
    action: &str,
    session: impl Serialize,
) {
    broadcast_resource_update(manager, user_id, ResourceType::ChatSession, action, session).await;
}
