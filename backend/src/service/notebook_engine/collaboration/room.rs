use anyhow::Result;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use yrs::{Doc, ReadTxn, StateVector, Transact, Update};
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use y_sync::awareness::{Awareness, AwarenessUpdate};

use crate::models::notebook::CollaboratorRole;

/// Awareness state for a user in a collaboration room
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwarenessState {
    pub user: UserInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<CursorState>,
}

/// User information for awareness
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub name: String,
    pub email: String,
    pub color: String,
}

/// Cursor state for awareness
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorState {
    pub anchor: u32,
    pub head: u32,
}

/// Message types that can be sent to clients
#[derive(Debug, Clone)]
pub enum CollabMessage {
    /// Y.js sync step 1 (state vector)
    SyncStep1(Vec<u8>),
    /// Y.js sync step 2 (update/diff)
    SyncStep2(Vec<u8>),
    /// Incremental Y.js update
    Update(Vec<u8>),
    /// Awareness update
    Awareness(Vec<u8>),
}

/// Client connection in a collaboration room
#[derive(Debug)]
pub struct ClientConnection {
    pub user_id: String,
    pub user_email: String,
    pub user_name: String,
    pub user_color: String,
    pub role: CollaboratorRole,
    pub sender: mpsc::UnboundedSender<CollabMessage>,
    pub joined_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

impl ClientConnection {
    pub fn new(
        user_id: String,
        user_email: String,
        user_name: String,
        user_color: String,
        role: CollaboratorRole,
        sender: mpsc::UnboundedSender<CollabMessage>,
    ) -> Self {
        let now = Utc::now();
        Self {
            user_id,
            user_email,
            user_name,
            user_color,
            role,
            sender,
            joined_at: now,
            last_activity: now,
        }
    }

    /// Check if this client can send edits
    pub fn can_edit(&self) -> bool {
        matches!(self.role, CollaboratorRole::Owner | CollaboratorRole::Editor)
    }
}


/// A collaboration room for real-time Y.js document editing
pub struct CollaborationRoom {
    /// The note ID this room is for
    pub note_id: String,
    /// The owner's user ID (for database routing)
    pub owner_id: String,
    /// The Y.js document
    doc: Doc,
    /// Awareness state for presence
    awareness: Awareness,
    /// Connected clients: user_id -> ClientConnection
    clients: HashMap<String, ClientConnection>,
    /// When the room was created
    pub created_at: DateTime<Utc>,
    /// Last time the document was persisted
    pub last_persisted: DateTime<Utc>,
    /// Whether the document has unsaved changes
    pub has_unsaved_changes: bool,
}

impl CollaborationRoom {
    /// Create a new collaboration room
    pub fn new(note_id: String, owner_id: String, initial_state: Option<Vec<u8>>) -> Result<Self> {
        let doc = Doc::new();
        
        // Apply initial state if provided
        if let Some(state) = initial_state {
            if !state.is_empty() {
                let update = Update::decode_v1(&state)
                    .map_err(|e| anyhow::anyhow!("Failed to decode Y.js state: {}", e))?;
                let mut txn = doc.transact_mut();
                txn.apply_update(update);
            }
        }
        
        let awareness = Awareness::new(doc.clone());
        let now = Utc::now();
        
        Ok(Self {
            note_id,
            owner_id,
            doc,
            awareness,
            clients: HashMap::new(),
            created_at: now,
            last_persisted: now,
            has_unsaved_changes: false,
        })
    }

    /// Add a client to the room
    pub fn add_client(&mut self, client: ClientConnection) -> Result<()> {
        let user_id = client.user_id.clone();
        
        // Set awareness state for this client
        let awareness_state = AwarenessState {
            user: UserInfo {
                id: client.user_id.clone(),
                name: client.user_name.clone(),
                email: client.user_email.clone(),
                color: client.user_color.clone(),
            },
            cursor: None,
        };
        
        // Generate a unique client ID for awareness
        let _client_id = self.generate_client_id(&user_id);
        
        // Set local awareness state
        let state_json = serde_json::to_string(&awareness_state)?;
        self.awareness.set_local_state(state_json);
        
        // Store the client
        self.clients.insert(user_id.clone(), client);
        
        // Broadcast awareness update to all other clients
        self.broadcast_awareness_to_others(&user_id)?;
        
        log::info!("Client {} joined room {}", user_id, self.note_id);
        Ok(())
    }

    /// Remove a client from the room
    pub fn remove_client(&mut self, user_id: &str) -> Result<bool> {
        if let Some(_client) = self.clients.remove(user_id) {
            // Clear awareness state for this client
            self.awareness.clean_local_state();
            
            // Broadcast awareness removal to remaining clients
            self.broadcast_awareness_to_others(user_id)?;
            
            log::info!("Client {} left room {}", user_id, self.note_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Apply a Y.js update from a client
    pub fn apply_update(&mut self, update_data: &[u8], sender_id: &str) -> Result<()> {
        // Check if sender can edit
        if let Some(client) = self.clients.get(sender_id) {
            if !client.can_edit() {
                anyhow::bail!("Client {} does not have edit permissions", sender_id);
            }
        } else {
            anyhow::bail!("Client {} not found in room", sender_id);
        }
        
        // Decode and apply the update
        let update = Update::decode_v1(update_data)
            .map_err(|e| anyhow::anyhow!("Failed to decode update: {}", e))?;
        
        {
            let mut txn = self.doc.transact_mut();
            txn.apply_update(update);
        }
        
        self.has_unsaved_changes = true;
        
        // Update sender's last activity
        if let Some(client) = self.clients.get_mut(sender_id) {
            client.last_activity = Utc::now();
        }
        
        // Broadcast update to all other clients
        self.broadcast_update_to_others(update_data, sender_id)?;
        
        Ok(())
    }

    /// Get the current state vector
    pub fn get_state_vector(&self) -> Vec<u8> {
        let txn = self.doc.transact();
        txn.state_vector().encode_v1()
    }

    /// Encode the full document state as an update
    pub fn encode_state_as_update(&self) -> Vec<u8> {
        let txn = self.doc.transact();
        txn.encode_state_as_update_v1(&StateVector::default())
    }

    /// Encode state diff from a given state vector
    pub fn encode_state_diff(&self, state_vector: &[u8]) -> Result<Vec<u8>> {
        let sv = StateVector::decode_v1(state_vector)
            .map_err(|e| anyhow::anyhow!("Failed to decode state vector: {}", e))?;
        let txn = self.doc.transact();
        Ok(txn.encode_state_as_update_v1(&sv))
    }


    /// Update awareness state for a user
    pub fn update_awareness(&mut self, user_id: &str, state: AwarenessState) -> Result<()> {
        // Update the client's last activity
        if let Some(client) = self.clients.get_mut(user_id) {
            client.last_activity = Utc::now();
        }
        
        // Set local awareness state
        let state_json = serde_json::to_string(&state)?;
        self.awareness.set_local_state(state_json);
        
        // Broadcast to other clients
        self.broadcast_awareness_to_others(user_id)?;
        
        Ok(())
    }

    /// Apply an awareness update from a client
    pub fn apply_awareness_update(&mut self, update_data: &[u8], sender_id: &str) -> Result<()> {
        // Decode the awareness update
        let update = AwarenessUpdate::decode_v1(update_data)
            .map_err(|e| anyhow::anyhow!("Failed to decode awareness update: {}", e))?;
        
        // Apply the update
        self.awareness.apply_update(update)
            .map_err(|e| anyhow::anyhow!("Failed to apply awareness update: {}", e))?;
        
        // Update sender's last activity
        if let Some(client) = self.clients.get_mut(sender_id) {
            client.last_activity = Utc::now();
        }
        
        // Broadcast to other clients
        self.broadcast_awareness_to_others(sender_id)?;
        
        Ok(())
    }

    /// Broadcast awareness state to all participants
    pub fn broadcast_awareness(&self) -> Result<()> {
        let update = self.awareness.update()
            .map_err(|e| anyhow::anyhow!("Failed to get awareness update: {}", e))?;
        let encoded = update.encode_v1();
        
        for client in self.clients.values() {
            let _ = client.sender.send(CollabMessage::Awareness(encoded.clone()));
        }
        
        Ok(())
    }

    /// Get the number of connected clients
    pub fn client_count(&self) -> usize {
        self.clients.len()
    }

    /// Check if the room is empty
    pub fn is_empty(&self) -> bool {
        self.clients.is_empty()
    }

    /// Get list of active participants
    pub fn get_participants(&self) -> Vec<ParticipantInfo> {
        self.clients.values().map(|c| ParticipantInfo {
            user_id: c.user_id.clone(),
            user_email: c.user_email.clone(),
            user_name: c.user_name.clone(),
            user_color: c.user_color.clone(),
            role: c.role.clone(),
            is_active: true,
            joined_at: c.joined_at,
            last_activity: c.last_activity,
        }).collect()
    }

    /// Get the encoded awareness state for all clients
    pub fn get_encoded_awareness(&self) -> Result<Vec<u8>> {
        let update = self.awareness.update()
            .map_err(|e| anyhow::anyhow!("Failed to get awareness update: {}", e))?;
        Ok(update.encode_v1())
    }

    /// Build awareness states from connected clients
    pub fn build_awareness_states(&self) -> Vec<AwarenessState> {
        self.clients.values().map(|c| AwarenessState {
            user: UserInfo {
                id: c.user_id.clone(),
                name: c.user_name.clone(),
                email: c.user_email.clone(),
                color: c.user_color.clone(),
            },
            cursor: None,
        }).collect()
    }

    /// Check if a user is in the room
    pub fn has_client(&self, user_id: &str) -> bool {
        self.clients.contains_key(user_id)
    }

    /// Get a client's role
    pub fn get_client_role(&self, user_id: &str) -> Option<CollaboratorRole> {
        self.clients.get(user_id).map(|c| c.role.clone())
    }

    /// Send initial sync to a newly connected client
    pub fn send_initial_sync(&self, user_id: &str) -> Result<()> {
        if let Some(client) = self.clients.get(user_id) {
            // Send SyncStep1 (our state vector)
            let state_vector = self.get_state_vector();
            client.sender.send(CollabMessage::SyncStep1(state_vector))?;
            
            // Send full document state as SyncStep2
            let full_state = self.encode_state_as_update();
            client.sender.send(CollabMessage::SyncStep2(full_state))?;
            
            // Send current awareness state
            if let Ok(update) = self.awareness.update() {
                let encoded = update.encode_v1();
                client.sender.send(CollabMessage::Awareness(encoded))?;
            }
        }
        Ok(())
    }

    /// Handle SyncStep1 from a client (they send their state vector)
    pub fn handle_sync_step1(&self, state_vector: &[u8], sender_id: &str) -> Result<()> {
        if let Some(client) = self.clients.get(sender_id) {
            // Calculate diff and send as SyncStep2
            let diff = self.encode_state_diff(state_vector)?;
            client.sender.send(CollabMessage::SyncStep2(diff))?;
        }
        Ok(())
    }

    // Private helper methods

    fn generate_client_id(&self, user_id: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        user_id.hash(&mut hasher);
        self.note_id.hash(&mut hasher);
        Utc::now().timestamp_nanos_opt().unwrap_or(0).hash(&mut hasher);
        hasher.finish()
    }

    fn broadcast_update_to_others(&self, update_data: &[u8], sender_id: &str) -> Result<()> {
        let msg = CollabMessage::Update(update_data.to_vec());
        for (uid, client) in &self.clients {
            if uid != sender_id {
                let _ = client.sender.send(msg.clone());
            }
        }
        Ok(())
    }

    fn broadcast_awareness_to_others(&self, sender_id: &str) -> Result<()> {
        if let Ok(update) = self.awareness.update() {
            let encoded = update.encode_v1();
            let msg = CollabMessage::Awareness(encoded);
            for (uid, client) in &self.clients {
                if uid != sender_id {
                    let _ = client.sender.send(msg.clone());
                }
            }
        }
        Ok(())
    }
}

/// Information about a participant in a room
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantInfo {
    pub user_id: String,
    pub user_email: String,
    pub user_name: String,
    pub user_color: String,
    pub role: CollaboratorRole,
    pub is_active: bool,
    pub joined_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_room() {
        let room = CollaborationRoom::new(
            "note-123".to_string(),
            "owner-456".to_string(),
            None,
        ).unwrap();
        
        assert_eq!(room.note_id, "note-123");
        assert_eq!(room.owner_id, "owner-456");
        assert!(room.is_empty());
        assert!(!room.has_unsaved_changes);
    }

    #[test]
    fn test_create_room_with_initial_state() {
        // Create a doc and get its state
        let doc = Doc::new();
        let state = {
            let txn = doc.transact();
            txn.encode_state_as_update_v1(&StateVector::default())
        };
        
        let room = CollaborationRoom::new(
            "note-123".to_string(),
            "owner-456".to_string(),
            Some(state),
        ).unwrap();
        
        assert_eq!(room.note_id, "note-123");
        assert!(room.is_empty());
    }

    #[test]
    fn test_state_vector_encoding() {
        let room = CollaborationRoom::new(
            "note-123".to_string(),
            "owner-456".to_string(),
            None,
        ).unwrap();
        
        let sv = room.get_state_vector();
        assert!(!sv.is_empty());
        
        let full_state = room.encode_state_as_update();
        assert!(!full_state.is_empty());
    }
}
