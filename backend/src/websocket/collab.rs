//! WebSocket handler for Y.js collaboration protocol
//!
//! This module implements the y-websocket protocol for real-time collaborative editing.
//! It handles:
//! - JWT authentication from query parameters
//! - Y.js sync protocol (SyncStep1, SyncStep2, Update)
//! - Awareness protocol for presence
//! - Role-based permissions (viewer, editor, owner)

use actix_web::{
    web::{self, Data, Payload},
    HttpRequest, HttpResponse, Result,
};
use actix_ws::{handle, Message};
use futures_util::StreamExt;
use log::{debug, error, info, warn};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::models::notebook::CollaboratorRole;
use crate::service::notebook_engine::collaboration::{
    ClientConnection, CollabMessage, RoomManager,
};
use crate::turso::config::SupabaseConfig;
use crate::turso::{validate_supabase_jwt_token, AppState};

/// Y.js protocol message types
/// These follow the y-websocket protocol specification
const MSG_SYNC: u8 = 0;
const MSG_AWARENESS: u8 = 1;

/// Sync message subtypes
const MSG_SYNC_STEP1: u8 = 0;
const MSG_SYNC_STEP2: u8 = 1;
const MSG_SYNC_UPDATE: u8 = 2;

/// Query parameters for WebSocket connection
#[derive(Debug, serde::Deserialize)]
pub struct CollabWsQuery {
    pub token: String,
}

/// WebSocket handler for collaboration endpoint `/ws/collab/{note_id}`
pub async fn collab_ws_handler(
    req: HttpRequest,
    stream: Payload,
    path: web::Path<String>,
    room_manager: Data<Arc<Mutex<RoomManager>>>,
    app_state: Data<AppState>,
    supabase_config: Data<SupabaseConfig>,
) -> Result<HttpResponse> {
    let note_id = path.into_inner();
    
    // Extract token from query parameters
    let token = req
        .uri()
        .query()
        .and_then(|q| {
            q.split('&')
                .find(|pair| pair.starts_with("token="))
                .and_then(|pair| pair.split('=').nth(1))
        })
        .ok_or_else(|| {
            warn!("Missing authentication token for collab WebSocket");
            actix_web::error::ErrorUnauthorized("Missing authentication token")
        })?;

    // Validate JWT token
    let claims = validate_supabase_jwt_token(token, &supabase_config)
        .await
        .map_err(|e| {
            warn!("Invalid JWT token for collab WebSocket: {}", e);
            actix_web::error::ErrorUnauthorized(format!("Invalid token: {}", e))
        })?;

    let user_id = claims.sub.clone();
    let user_email = claims.email.clone().unwrap_or_else(|| "unknown@email.com".to_string());
    
    info!(
        "Collab WebSocket connection attempt for note {} by user {}",
        note_id, user_id
    );

    // Verify collaboration access via central registry
    let (owner_id, role) = verify_collaboration_access(&app_state, &note_id, &user_id).await?;

    info!(
        "User {} has {:?} access to note {} (owner: {})",
        user_id, role, note_id, owner_id
    );

    // Handle WebSocket upgrade
    let (res, session, mut msg_stream) = handle(&req, stream)?;

    // Create channel for sending messages to this client
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<CollabMessage>();

    // Generate a user color based on user_id hash
    let user_color = generate_user_color(&user_id);
    
    // Get user name from email or use a default
    let user_name = user_email.split('@').next().unwrap_or("User").to_string();

    // Create client connection
    let client = ClientConnection::new(
        user_id.clone(),
        user_email,
        user_name,
        user_color,
        role.clone(),
        tx.clone(),
    );

    // Get or create room and add client
    let room_manager_clone = room_manager.get_ref().clone();
    let app_state_clone = app_state.get_ref().clone();
    
    {
        let manager = room_manager_clone.lock().await;
        let room = manager
            .get_or_create_room(&note_id, &owner_id, &app_state_clone)
            .await
            .map_err(|e| {
                error!("Failed to get/create room for note {}: {}", note_id, e);
                actix_web::error::ErrorInternalServerError("Failed to join collaboration room")
            })?;

        let mut room_guard = room.lock().await;
        room_guard.add_client(client).map_err(|e| {
            error!("Failed to add client to room: {}", e);
            actix_web::error::ErrorInternalServerError("Failed to join collaboration room")
        })?;

        // Send initial sync to the new client
        room_guard.send_initial_sync(&user_id).map_err(|e| {
            error!("Failed to send initial sync: {}", e);
            actix_web::error::ErrorInternalServerError("Failed to initialize sync")
        })?;
    }

    info!(
        "User {} joined collaboration room for note {}",
        user_id, note_id
    );

    // Spawn handler for this connection
    let note_id_clone = note_id.clone();
    let user_id_clone = user_id.clone();
    let role_clone = role.clone();
    
    actix_web::rt::spawn(async move {
        // Spawn task to send messages from room to client
        let user_id_send = user_id_clone.clone();
        let mut session_send = session;
        
        let send_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let binary_msg = encode_collab_message(&msg);
                if let Err(e) = session_send.binary(binary_msg).await {
                    error!("Failed to send message to client {}: {}", user_id_send, e);
                    break;
                }
            }
        });

        // Handle incoming messages
        while let Some(msg_result) = msg_stream.next().await {
            match msg_result {
                Ok(msg) => {
                    match msg {
                        Message::Binary(data) => {
                            if let Err(e) = handle_binary_message(
                                &data,
                                &note_id_clone,
                                &user_id_clone,
                                &role_clone,
                                &room_manager_clone,
                            )
                            .await
                            {
                                warn!(
                                    "Error handling binary message from {}: {}",
                                    user_id_clone, e
                                );
                            }
                        }
                        Message::Text(text) => {
                            debug!("Received text message from {}: {}", user_id_clone, text);
                            // Y.js protocol uses binary messages, text messages are ignored
                        }
                        Message::Close(reason) => {
                            info!(
                                "WebSocket closing for user {} on note {}: {:?}",
                                user_id_clone, note_id_clone, reason
                            );
                            break;
                        }
                        Message::Ping(data) => {
                            debug!("Received ping from {}", user_id_clone);
                            // Pong is handled automatically by actix-ws
                            let _ = data;
                        }
                        Message::Pong(_) => {
                            debug!("Received pong from {}", user_id_clone);
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    error!(
                        "WebSocket error for user {} on note {}: {:?}",
                        user_id_clone, note_id_clone, e
                    );
                    break;
                }
            }
        }

        // Client disconnected - remove from room
        {
            let manager = room_manager_clone.lock().await;
            if let Err(e) = manager
                .remove_client_from_room(&note_id_clone, &user_id_clone, &app_state_clone)
                .await
            {
                error!(
                    "Failed to remove client {} from room {}: {}",
                    user_id_clone, note_id_clone, e
                );
            }
        }

        // Cancel send task
        send_task.abort();

        info!(
            "User {} disconnected from collaboration room for note {}",
            user_id_clone, note_id_clone
        );
    });

    Ok(res)
}

/// Verify that a user has collaboration access to a note
/// Returns (owner_id, role) if access is granted
/// 
/// In the multi-tenant architecture, each user has their own database.
/// If a note exists in a user's database, they are the owner.
/// Collaborators are tracked in the central registry.
async fn verify_collaboration_access(
    app_state: &AppState,
    note_id: &str,
    user_id: &str,
) -> Result<(String, CollaboratorRole), actix_web::Error> {
    // First check if user is the owner by checking their own database
    // In multi-tenant architecture, if note exists in user's DB, they own it
    if let Ok(Some(conn)) = app_state.get_user_db_connection(user_id).await {
        // Check if note exists in user's database (they're the owner)
        if crate::models::notebook::NotebookNote::find_by_id(&conn, note_id).await.is_ok() {
            // Note exists in user's database - they are the owner
            return Ok((user_id.to_string(), CollaboratorRole::Owner));
        }
    }

    // Check central registry for collaborator access
    match app_state.get_collaborator_entry(note_id, user_id).await {
        Ok(Some(entry)) => {
            let role = match entry.role.as_str() {
                "owner" => CollaboratorRole::Owner,
                "editor" => CollaboratorRole::Editor,
                "viewer" => CollaboratorRole::Viewer,
                _ => CollaboratorRole::Viewer,
            };
            Ok((entry.owner_id, role))
        }
        Ok(None) => {
            warn!(
                "User {} has no collaboration access to note {}",
                user_id, note_id
            );
            Err(actix_web::error::ErrorForbidden(
                "No access to this note",
            ))
        }
        Err(e) => {
            error!("Failed to check collaboration access: {}", e);
            Err(actix_web::error::ErrorInternalServerError(
                "Failed to verify access",
            ))
        }
    }
}

/// Handle binary Y.js protocol messages
async fn handle_binary_message(
    data: &[u8],
    note_id: &str,
    user_id: &str,
    role: &CollaboratorRole,
    room_manager: &Arc<Mutex<RoomManager>>,
) -> anyhow::Result<()> {
    if data.is_empty() {
        return Ok(());
    }

    let msg_type = data[0];
    let payload = &data[1..];

    match msg_type {
        MSG_SYNC => handle_sync_message(payload, note_id, user_id, role, room_manager).await,
        MSG_AWARENESS => handle_awareness_message(payload, note_id, user_id, room_manager).await,
        _ => {
            warn!("Unknown message type {} from user {}", msg_type, user_id);
            Ok(())
        }
    }
}

/// Handle Y.js sync protocol messages
async fn handle_sync_message(
    data: &[u8],
    note_id: &str,
    user_id: &str,
    role: &CollaboratorRole,
    room_manager: &Arc<Mutex<RoomManager>>,
) -> anyhow::Result<()> {
    if data.is_empty() {
        return Ok(());
    }

    let sync_type = data[0];
    let payload = &data[1..];

    let manager = room_manager.lock().await;
    let room = manager
        .get_room(note_id)
        .ok_or_else(|| anyhow::anyhow!("Room not found for note {}", note_id))?;

    let mut room_guard = room.lock().await;

    match sync_type {
        MSG_SYNC_STEP1 => {
            // Client sends their state vector, we respond with diff
            debug!("Received SyncStep1 from user {} for note {}", user_id, note_id);
            room_guard.handle_sync_step1(payload, user_id)?;
        }
        MSG_SYNC_STEP2 => {
            // Client sends update/diff - only editors and owners can apply
            debug!("Received SyncStep2 from user {} for note {}", user_id, note_id);
            if can_edit(role) {
                room_guard.apply_update(payload, user_id)?;
            } else {
                warn!(
                    "Viewer {} attempted to send SyncStep2 update to note {}",
                    user_id, note_id
                );
            }
        }
        MSG_SYNC_UPDATE => {
            // Incremental update - only editors and owners can apply
            debug!("Received Update from user {} for note {}", user_id, note_id);
            if can_edit(role) {
                room_guard.apply_update(payload, user_id)?;
            } else {
                warn!(
                    "Viewer {} attempted to send update to note {}",
                    user_id, note_id
                );
            }
        }
        _ => {
            warn!("Unknown sync message type {} from user {}", sync_type, user_id);
        }
    }

    Ok(())
}

/// Handle Y.js awareness protocol messages
async fn handle_awareness_message(
    data: &[u8],
    note_id: &str,
    user_id: &str,
    room_manager: &Arc<Mutex<RoomManager>>,
) -> anyhow::Result<()> {
    debug!(
        "Received awareness update from user {} for note {}",
        user_id, note_id
    );

    let manager = room_manager.lock().await;
    let room = manager
        .get_room(note_id)
        .ok_or_else(|| anyhow::anyhow!("Room not found for note {}", note_id))?;

    let mut room_guard = room.lock().await;
    room_guard.apply_awareness_update(data, user_id)?;

    Ok(())
}

/// Check if a role allows editing
fn can_edit(role: &CollaboratorRole) -> bool {
    matches!(role, CollaboratorRole::Owner | CollaboratorRole::Editor)
}

/// Encode a CollabMessage to binary format for y-websocket protocol
fn encode_collab_message(msg: &CollabMessage) -> Vec<u8> {
    match msg {
        CollabMessage::SyncStep1(data) => {
            let mut result = Vec::with_capacity(2 + data.len());
            result.push(MSG_SYNC);
            result.push(MSG_SYNC_STEP1);
            result.extend_from_slice(data);
            result
        }
        CollabMessage::SyncStep2(data) => {
            let mut result = Vec::with_capacity(2 + data.len());
            result.push(MSG_SYNC);
            result.push(MSG_SYNC_STEP2);
            result.extend_from_slice(data);
            result
        }
        CollabMessage::Update(data) => {
            let mut result = Vec::with_capacity(2 + data.len());
            result.push(MSG_SYNC);
            result.push(MSG_SYNC_UPDATE);
            result.extend_from_slice(data);
            result
        }
        CollabMessage::Awareness(data) => {
            let mut result = Vec::with_capacity(1 + data.len());
            result.push(MSG_AWARENESS);
            result.extend_from_slice(data);
            result
        }
    }
}

/// Generate a consistent color for a user based on their ID
fn generate_user_color(user_id: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    user_id.hash(&mut hasher);
    let hash = hasher.finish();

    // Generate a pleasant color from the hash
    let hue = (hash % 360) as u16;
    let saturation = 70;
    let lightness = 50;

    // Convert HSL to hex
    hsl_to_hex(hue, saturation, lightness)
}

/// Convert HSL to hex color string
fn hsl_to_hex(h: u16, s: u8, l: u8) -> String {
    let s = s as f64 / 100.0;
    let l = l as f64 / 100.0;

    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h as f64 / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r, g, b) = match h {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    let r = ((r + m) * 255.0) as u8;
    let g = ((g + m) * 255.0) as u8;
    let b = ((b + m) * 255.0) as u8;

    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_user_color() {
        let color1 = generate_user_color("user-123");
        let color2 = generate_user_color("user-456");
        let color3 = generate_user_color("user-123");

        // Same user should get same color
        assert_eq!(color1, color3);
        // Different users should get different colors (usually)
        assert_ne!(color1, color2);
        // Color should be valid hex
        assert!(color1.starts_with('#'));
        assert_eq!(color1.len(), 7);
    }

    #[test]
    fn test_encode_sync_step1() {
        let data = vec![1, 2, 3, 4];
        let msg = CollabMessage::SyncStep1(data.clone());
        let encoded = encode_collab_message(&msg);

        assert_eq!(encoded[0], MSG_SYNC);
        assert_eq!(encoded[1], MSG_SYNC_STEP1);
        assert_eq!(&encoded[2..], &data[..]);
    }

    #[test]
    fn test_encode_awareness() {
        let data = vec![5, 6, 7, 8];
        let msg = CollabMessage::Awareness(data.clone());
        let encoded = encode_collab_message(&msg);

        assert_eq!(encoded[0], MSG_AWARENESS);
        assert_eq!(&encoded[1..], &data[..]);
    }

    #[test]
    fn test_can_edit() {
        assert!(can_edit(&CollaboratorRole::Owner));
        assert!(can_edit(&CollaboratorRole::Editor));
        assert!(!can_edit(&CollaboratorRole::Viewer));
    }

    #[test]
    fn test_hsl_to_hex() {
        // Red
        let red = hsl_to_hex(0, 100, 50);
        assert_eq!(red, "#ff0000");

        // Green
        let green = hsl_to_hex(120, 100, 50);
        assert_eq!(green, "#00ff00");

        // Blue
        let blue = hsl_to_hex(240, 100, 50);
        assert_eq!(blue, "#0000ff");
    }
}
