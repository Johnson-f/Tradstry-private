use super::messages::WsMessage;
use dashmap::DashMap;
use std::sync::Arc;

/// WebSocket session data
#[allow(dead_code)]
pub struct WsSession {
    pub user_id: String,
    pub sender: tokio::sync::mpsc::UnboundedSender<String>,
}

/// Manages WebSocket connections for all users
#[derive(Clone)]
pub struct ConnectionManager {
    /// Maps user_id -> Vec<sender>
    pub(crate) clients: Arc<DashMap<String, Vec<tokio::sync::mpsc::UnboundedSender<String>>>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(DashMap::new()),
        }
    }

    /// Register a new WebSocket connection for a user
    #[allow(dead_code)]
    pub fn register(&self, user_id: String, sender: tokio::sync::mpsc::UnboundedSender<String>) {
        self.clients.entry(user_id).or_default().push(sender);
    }

    /// Unregister a WebSocket connection
    /// Returns true if the connection was found and removed
    #[allow(dead_code)]
    pub fn unregister(
        &self,
        user_id: &str,
        sender: &tokio::sync::mpsc::UnboundedSender<String>,
    ) -> bool {
        if let Some(mut clients) = self.clients.get_mut(user_id) {
            let sender_ptr: *const _ = sender;
            let initial_len = clients.len();

            clients.retain(|s| {
                let s_ptr: *const _ = s;
                s_ptr != sender_ptr
            });

            let removed = clients.len() < initial_len;

            if clients.is_empty() {
                drop(clients);
                self.clients.remove(user_id);
            }

            removed
        } else {
            false
        }
    }

    /// Get the number of active connections for a user
    #[allow(dead_code)]
    pub fn connection_count(&self, user_id: &str) -> usize {
        self.clients.get(user_id).map(|c| c.len()).unwrap_or(0)
    }

    /// Get total number of connected users
    #[allow(dead_code)]
    pub fn user_count(&self) -> usize {
        self.clients.len()
    }

    /// Get total number of connections across all users
    #[allow(dead_code)]
    pub fn total_connections(&self) -> usize {
        self.clients.iter().map(|entry| entry.value().len()).sum()
    }

    /// Broadcast a message to all connections for a specific user
    pub fn broadcast_to_user(&self, user_id: &str, message: WsMessage) {
        if let Some(clients) = self.clients.get(user_id) {
            let message_json = serde_json::to_string(&message).unwrap_or_else(|_| "{}".to_string());
            for sender in clients.iter() {
                let _ = sender.send(message_json.clone());
            }
        }
    }

    /// Broadcast to all users (for admin messages)
    #[allow(dead_code)]
    pub fn broadcast_to_all(&self, message: WsMessage) {
        let message_json = serde_json::to_string(&message).unwrap_or_else(|_| "{}".to_string());
        for clients in self.clients.iter() {
            for sender in clients.value().iter() {
                let _ = sender.send(message_json.clone());
            }
        }
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionManager {
    /// Get mutable reference to clients for direct manipulation
    #[allow(dead_code)]
    pub fn as_mut(&self) -> &DashMap<String, Vec<tokio::sync::mpsc::UnboundedSender<String>>> {
        &self.clients
    }
}
