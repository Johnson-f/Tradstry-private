use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use libsql::Connection;
use log::{error, info, warn};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{interval, Duration as TokioDuration};
use yrs::XmlFragment;

use super::room::{ClientConnection, CollabMessage, CollaborationRoom};
use crate::models::notebook::{CollaboratorRole, NotebookNote};
use crate::turso::AppState;

/// Configuration for the RoomManager
#[derive(Debug, Clone)]
pub struct RoomManagerConfig {
    /// How often to persist room state (in seconds)
    pub persistence_interval_secs: u64,
    /// How long to keep empty rooms before cleanup (in seconds)
    pub empty_room_timeout_secs: u64,
    /// Maximum number of concurrent rooms
    pub max_rooms: usize,
}

impl Default for RoomManagerConfig {
    fn default() -> Self {
        Self {
            persistence_interval_secs: 30,
            empty_room_timeout_secs: 60,
            max_rooms: 1000,
        }
    }
}

/// Manages all active collaboration rooms
/// 
/// The RoomManager is responsible for:
/// - Creating and retrieving collaboration rooms (lazy loading)
/// - Persisting room state to the database
/// - Cleaning up empty rooms
/// - Periodic persistence of active rooms
pub struct RoomManager {
    /// Active rooms: note_id -> CollaborationRoom
    rooms: Arc<DashMap<String, Arc<Mutex<CollaborationRoom>>>>,
    /// Configuration
    config: RoomManagerConfig,
    /// Shutdown signal
    shutdown: Arc<RwLock<bool>>,
}

impl RoomManager {
    /// Create a new RoomManager
    pub fn new(config: RoomManagerConfig) -> Self {
        Self {
            rooms: Arc::new(DashMap::new()),
            config,
            shutdown: Arc::new(RwLock::new(false)),
        }
    }

    /// Create a new RoomManager with default configuration
    pub fn with_defaults() -> Self {
        Self::new(RoomManagerConfig::default())
    }

    /// Get or create a collaboration room for a note
    /// 
    /// This method implements lazy loading:
    /// - If the room exists in memory, return it
    /// - If not, load the Y.js state from the database and create a new room
    pub async fn get_or_create_room(
        &self,
        note_id: &str,
        owner_id: &str,
        app_state: &AppState,
    ) -> Result<Arc<Mutex<CollaborationRoom>>> {
        // Check if room already exists
        if let Some(room) = self.rooms.get(note_id) {
            info!("Returning existing room for note: {}", note_id);
            return Ok(Arc::clone(room.value()));
        }

        // Check max rooms limit
        if self.rooms.len() >= self.config.max_rooms {
            anyhow::bail!("Maximum number of collaboration rooms reached");
        }

        // Load Y.js state from database
        let y_state = self.load_y_state_from_db(note_id, owner_id, app_state).await?;

        // Create new room
        let room = CollaborationRoom::new(
            note_id.to_string(),
            owner_id.to_string(),
            y_state,
        )?;

        let room = Arc::new(Mutex::new(room));
        
        // Store in map
        self.rooms.insert(note_id.to_string(), Arc::clone(&room));
        
        info!("Created new collaboration room for note: {}", note_id);
        Ok(room)
    }

    /// Get an existing room without creating one
    pub fn get_room(&self, note_id: &str) -> Option<Arc<Mutex<CollaborationRoom>>> {
        self.rooms.get(note_id).map(|r| Arc::clone(r.value()))
    }

    /// Check if a room exists
    pub fn has_room(&self, note_id: &str) -> bool {
        self.rooms.contains_key(note_id)
    }

    /// Get the number of active rooms
    pub fn room_count(&self) -> usize {
        self.rooms.len()
    }

    /// Close a room and persist its state
    /// 
    /// This is called when the last client disconnects from a room
    pub async fn close_room(&self, note_id: &str, app_state: &AppState) -> Result<()> {
        if let Some((_, room)) = self.rooms.remove(note_id) {
            let room_guard = room.lock().await;
            
            // Persist final state if there are unsaved changes
            if room_guard.has_unsaved_changes {
                let y_state = room_guard.encode_state_as_update();
                let owner_id = room_guard.owner_id.clone();
                drop(room_guard); // Release lock before async DB operation
                
                self.persist_room_state(note_id, &owner_id, &y_state, app_state).await?;
            }
            
            info!("Closed collaboration room for note: {}", note_id);
        }
        Ok(())
    }

    /// Persist a room's state to the database
    pub async fn persist_room(&self, note_id: &str, app_state: &AppState) -> Result<()> {
        if let Some(room_ref) = self.rooms.get(note_id) {
            let mut room = room_ref.lock().await;
            
            if room.has_unsaved_changes {
                let y_state = room.encode_state_as_update();
                let owner_id = room.owner_id.clone();
                
                // Mark as persisted before releasing lock
                room.has_unsaved_changes = false;
                room.last_persisted = Utc::now();
                drop(room); // Release lock before async DB operation
                
                self.persist_room_state(note_id, &owner_id, &y_state, app_state).await?;
                
                info!("Persisted room state for note: {}", note_id);
            }
        }
        Ok(())
    }

    /// Remove a client from a room and handle cleanup
    /// 
    /// Returns true if the room was closed (last client left)
    pub async fn remove_client_from_room(
        &self,
        note_id: &str,
        user_id: &str,
        app_state: &AppState,
    ) -> Result<bool> {
        if let Some(room_ref) = self.rooms.get(note_id) {
            let mut room = room_ref.lock().await;
            room.remove_client(user_id)?;
            
            // Check if room is now empty
            if room.is_empty() {
                drop(room); // Release lock before closing
                drop(room_ref); // Release DashMap reference
                
                self.close_room(note_id, app_state).await?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Start the periodic persistence task
    /// 
    /// This spawns a background task that periodically persists all rooms
    /// with unsaved changes
    pub fn start_periodic_persistence(&self, app_state: Arc<AppState>) {
        let rooms = Arc::clone(&self.rooms);
        let interval_secs = self.config.persistence_interval_secs;
        let shutdown = Arc::clone(&self.shutdown);

        tokio::spawn(async move {
            let mut ticker = interval(TokioDuration::from_secs(interval_secs));
            
            loop {
                ticker.tick().await;
                
                // Check shutdown signal
                if *shutdown.read().await {
                    info!("Periodic persistence task shutting down");
                    break;
                }

                // Persist all rooms with unsaved changes
                for entry in rooms.iter() {
                    let note_id = entry.key().clone();
                    let room_ref = entry.value();
                    
                    let mut room = room_ref.lock().await;
                    if room.has_unsaved_changes {
                        let y_state = room.encode_state_as_update();
                        let owner_id = room.owner_id.clone();
                        
                        room.has_unsaved_changes = false;
                        room.last_persisted = Utc::now();
                        drop(room);
                        
                        // Persist to database
                        if let Err(e) = Self::persist_room_state_static(
                            &note_id,
                            &owner_id,
                            &y_state,
                            &app_state,
                        ).await {
                            error!("Failed to persist room {}: {}", note_id, e);
                        } else {
                            info!("Periodic persistence completed for room: {}", note_id);
                        }
                    }
                }
            }
        });
        
        info!("Started periodic persistence task (interval: {}s)", interval_secs);
    }

    /// Signal shutdown for background tasks
    pub async fn shutdown(&self) {
        let mut shutdown = self.shutdown.write().await;
        *shutdown = true;
        info!("RoomManager shutdown signaled");
    }

    /// Persist all rooms (called during graceful shutdown)
    pub async fn persist_all_rooms(&self, app_state: &AppState) -> Result<()> {
        info!("Persisting all {} rooms...", self.rooms.len());
        
        for entry in self.rooms.iter() {
            let note_id = entry.key();
            if let Err(e) = self.persist_room(note_id, app_state).await {
                error!("Failed to persist room {}: {}", note_id, e);
            }
        }
        
        info!("All rooms persisted");
        Ok(())
    }

    // Private helper methods

    /// Load Y.js state from the owner's database
    async fn load_y_state_from_db(
        &self,
        note_id: &str,
        owner_id: &str,
        app_state: &AppState,
    ) -> Result<Option<Vec<u8>>> {
        // Get connection to owner's database
        let conn = app_state
            .get_user_db_connection(owner_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get owner database connection: {}", e))?
            .ok_or_else(|| anyhow::anyhow!("Owner database not found: {}", owner_id))?;

        // Load Y.js state using the model method
        let y_state = NotebookNote::get_y_state(&conn, note_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to load Y.js state from database: {}", e))?;

        if y_state.is_some() {
            info!("Loaded existing Y.js state for note: {}", note_id);
        } else {
            info!("No existing Y.js state for note: {}", note_id);
        }

        Ok(y_state)
    }

    /// Persist room state to the owner's database
    async fn persist_room_state(
        &self,
        note_id: &str,
        owner_id: &str,
        y_state: &[u8],
        app_state: &AppState,
    ) -> Result<()> {
        Self::persist_room_state_static(note_id, owner_id, y_state, app_state).await
    }

    /// Static version of persist_room_state for use in spawned tasks
    async fn persist_room_state_static(
        note_id: &str,
        owner_id: &str,
        y_state: &[u8],
        app_state: &AppState,
    ) -> Result<()> {
        // Get connection to owner's database
        let conn = app_state
            .get_user_db_connection(owner_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get owner database connection: {}", e))?
            .ok_or_else(|| anyhow::anyhow!("Owner database not found: {}", owner_id))?;

        // Convert Y.js state to Lexical JSON content
        let (content, content_plain_text) = Self::convert_yjs_to_lexical(y_state)?;

        // Update both Y.js state and content in database
        NotebookNote::update_y_state_and_content(
            &conn,
            note_id,
            y_state,
            &content,
            content_plain_text.as_deref(),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to persist Y.js state to database: {}", e))?;

        info!("Persisted Y.js state for note: {}", note_id);
        Ok(())
    }

    /// Convert Y.js document state to Lexical JSON format
    /// 
    /// This extracts the text content from the Y.js document and creates
    /// a Lexical-compatible JSON structure.
    fn convert_yjs_to_lexical(y_state: &[u8]) -> Result<(Value, Option<String>)> {
        use yrs::{Doc, ReadTxn, StateVector, Transact, Update, GetString};
        use yrs::updates::decoder::Decode;

        // Create a new doc and apply the state
        let doc = Doc::new();
        
        if !y_state.is_empty() {
            let update = Update::decode_v1(y_state)
                .map_err(|e| anyhow::anyhow!("Failed to decode Y.js state: {}", e))?;
            let mut txn = doc.transact_mut();
            txn.apply_update(update);
        }

        // Try to extract text from the Y.js document
        // The Lexical Y.js binding typically uses a root "root" key
        let txn = doc.transact();
        
        // Try to get the text content from common Y.js structures
        let mut plain_text = String::new();
        
        // Check for XmlFragment (used by Lexical's Y.js binding)
        if let Some(xml_fragment) = txn.get_xml_fragment("root") {
            plain_text = Self::extract_text_from_xml_fragment(&xml_fragment, &txn);
        }
        
        // If no XML fragment, try Text type
        if plain_text.is_empty() {
            if let Some(text) = txn.get_text("root") {
                plain_text = text.get_string(&txn);
            }
        }

        // Create a minimal Lexical JSON structure
        // Note: The actual Lexical state is stored in Y.js, this is just for
        // backward compatibility with non-collaborative views
        let content = if plain_text.is_empty() {
            serde_json::json!({
                "root": {
                    "children": [],
                    "direction": null,
                    "format": "",
                    "indent": 0,
                    "type": "root",
                    "version": 1
                }
            })
        } else {
            serde_json::json!({
                "root": {
                    "children": [{
                        "children": [{
                            "detail": 0,
                            "format": 0,
                            "mode": "normal",
                            "style": "",
                            "text": plain_text.clone(),
                            "type": "text",
                            "version": 1
                        }],
                        "direction": "ltr",
                        "format": "",
                        "indent": 0,
                        "type": "paragraph",
                        "version": 1
                    }],
                    "direction": "ltr",
                    "format": "",
                    "indent": 0,
                    "type": "root",
                    "version": 1
                }
            })
        };

        let plain_text_opt = if plain_text.is_empty() {
            None
        } else {
            Some(plain_text)
        };

        Ok((content, plain_text_opt))
    }

    /// Extract plain text from a Y.js XmlFragment
    fn extract_text_from_xml_fragment<T: yrs::ReadTxn>(
        fragment: &yrs::XmlFragmentRef,
        txn: &T,
    ) -> String {
        use yrs::GetString;
        let mut text = String::new();
        
        // Iterate through children and extract text
        for child in fragment.successors(txn) {
            match child {
                yrs::XmlNode::Element(elem) => {
                    // Recursively extract text from element children
                    text.push_str(&Self::extract_text_from_xml_element(&elem, txn));
                }
                yrs::XmlNode::Fragment(frag) => {
                    text.push_str(&Self::extract_text_from_xml_fragment(&frag, txn));
                }
                yrs::XmlNode::Text(txt) => {
                    text.push_str(&txt.get_string(txn));
                }
            }
        }
        
        text
    }

    /// Extract plain text from a Y.js XmlElement
    fn extract_text_from_xml_element<T: yrs::ReadTxn>(
        element: &yrs::XmlElementRef,
        txn: &T,
    ) -> String {
        use yrs::GetString;
        let mut text = String::new();
        
        for child in element.successors(txn) {
            match child {
                yrs::XmlNode::Element(elem) => {
                    text.push_str(&Self::extract_text_from_xml_element(&elem, txn));
                }
                yrs::XmlNode::Fragment(frag) => {
                    text.push_str(&Self::extract_text_from_xml_fragment(&frag, txn));
                }
                yrs::XmlNode::Text(txt) => {
                    text.push_str(&txt.get_string(txn));
                }
            }
        }
        
        // Add newline after block elements
        let tag = element.tag();
        if matches!(tag.as_ref(), "p" | "div" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "li" | "br") {
            text.push('\n');
        }
        
        text
    }
}

impl Default for RoomManager {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_manager_config_default() {
        let config = RoomManagerConfig::default();
        assert_eq!(config.persistence_interval_secs, 30);
        assert_eq!(config.empty_room_timeout_secs, 60);
        assert_eq!(config.max_rooms, 1000);
    }

    #[test]
    fn test_room_manager_creation() {
        let manager = RoomManager::with_defaults();
        assert_eq!(manager.room_count(), 0);
        assert!(!manager.has_room("test-note"));
    }

    #[test]
    fn test_convert_empty_yjs_to_lexical() {
        let (content, plain_text) = RoomManager::convert_yjs_to_lexical(&[]).unwrap();
        assert!(plain_text.is_none());
        assert!(content.get("root").is_some());
    }
}
