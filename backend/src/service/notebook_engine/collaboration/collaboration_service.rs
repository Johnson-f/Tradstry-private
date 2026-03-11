use anyhow::Result;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use libsql::Connection;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::models::notebook::{
    CollaboratorRole, CreateCollaboratorRequest, CreateInvitationRequest,
    CreateShareSettingsRequest, NoteCollaborator, NoteInvitation, NoteShareSettings,
    NoteVisibility, UpdateShareSettingsRequest,
};
use crate::service::email_service::CollaborationEmailService;
use crate::websocket::{ConnectionManager, EventType, WsMessage};

/// Cursor position for real-time collaboration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    pub user_id: String,
    pub user_name: String,
    pub user_color: String,
    pub line: i32,
    pub column: i32,
    pub selection_start: Option<i32>,
    pub selection_end: Option<i32>,
}

/// Collaborator presence in a note room
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaboratorPresence {
    pub user_id: String,
    pub user_email: String,
    pub user_name: Option<String>,
    pub role: CollaboratorRole,
    pub cursor: Option<CursorPosition>,
    pub is_active: bool,
    pub joined_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

/// Active note editing room
#[derive(Debug, Clone)]
pub struct NoteRoom {
    pub note_id: String,
    pub participants: Vec<CollaboratorPresence>,
    pub created_at: DateTime<Utc>,
}

/// Collaboration service for managing real-time note editing
pub struct CollaborationService {
    /// Active rooms: note_id -> NoteRoom
    rooms: Arc<DashMap<String, NoteRoom>>,
    /// User colors for cursor display
    user_colors: Arc<DashMap<String, String>>,
}

impl CollaborationService {
    pub fn new() -> Self {
        Self {
            rooms: Arc::new(DashMap::new()),
            user_colors: Arc::new(DashMap::new()),
        }
    }

    /// Get or assign a color for a user
    fn get_user_color(&self, user_id: &str) -> String {
        if let Some(color) = self.user_colors.get(user_id) {
            return color.clone();
        }
        
        let colors = [
            "#FF6B6B", "#4ECDC4", "#45B7D1", "#96CEB4", "#FFEAA7",
            "#DDA0DD", "#98D8C8", "#F7DC6F", "#BB8FCE", "#85C1E9",
        ];
        let idx = self.user_colors.len() % colors.len();
        let color = colors[idx].to_string();
        self.user_colors.insert(user_id.to_string(), color.clone());
        color
    }

    /// Join a note room for real-time collaboration
    pub async fn join_room(
        &self,
        note_id: &str,
        user_id: &str,
        user_email: &str,
        user_name: Option<String>,
        role: CollaboratorRole,
        ws_manager: &Arc<Mutex<ConnectionManager>>,
    ) -> Result<NoteRoom> {
        let now = Utc::now();
        let presence = CollaboratorPresence {
            user_id: user_id.to_string(),
            user_email: user_email.to_string(),
            user_name: user_name.clone(),
            role,
            cursor: None,
            is_active: true,
            joined_at: now,
            last_activity: now,
        };

        let mut room = self.rooms.entry(note_id.to_string()).or_insert_with(|| NoteRoom {
            note_id: note_id.to_string(),
            participants: Vec::new(),
            created_at: now,
        });

        // Check if user already in room
        let existing_idx = room.participants.iter().position(|p| p.user_id == user_id);
        if let Some(idx) = existing_idx {
            room.participants[idx] = presence.clone();
        } else {
            room.participants.push(presence.clone());
        }

        // Broadcast join event to other participants
        let manager = ws_manager.lock().await;
        for participant in &room.participants {
            if participant.user_id != user_id {
                manager.broadcast_to_user(
                    &participant.user_id,
                    WsMessage::new(
                        EventType::CollaboratorJoined,
                        json!({
                            "note_id": note_id,
                            "user_id": user_id,
                            "user_email": user_email,
                            "user_name": user_name,
                            "color": self.get_user_color(user_id),
                        }),
                    ),
                );
            }
        }

        Ok(room.clone())
    }

    /// Leave a note room
    pub async fn leave_room(
        &self,
        note_id: &str,
        user_id: &str,
        ws_manager: &Arc<Mutex<ConnectionManager>>,
    ) -> Result<()> {
        if let Some(mut room) = self.rooms.get_mut(note_id) {
            room.participants.retain(|p| p.user_id != user_id);

            // Broadcast leave event
            let manager = ws_manager.lock().await;
            for participant in &room.participants {
                manager.broadcast_to_user(
                    &participant.user_id,
                    WsMessage::new(
                        EventType::CollaboratorLeft,
                        json!({
                            "note_id": note_id,
                            "user_id": user_id,
                        }),
                    ),
                );
            }

            // Remove empty rooms
            if room.participants.is_empty() {
                drop(room);
                self.rooms.remove(note_id);
            }
        }
        Ok(())
    }

    /// Update cursor position
    pub async fn update_cursor(
        &self,
        note_id: &str,
        user_id: &str,
        cursor: CursorPosition,
        ws_manager: &Arc<Mutex<ConnectionManager>>,
    ) -> Result<()> {
        if let Some(mut room) = self.rooms.get_mut(note_id) {
            if let Some(participant) = room.participants.iter_mut().find(|p| p.user_id == user_id) {
                participant.cursor = Some(cursor.clone());
                participant.last_activity = Utc::now();
            }

            // Broadcast cursor update
            let manager = ws_manager.lock().await;
            for participant in &room.participants {
                if participant.user_id != user_id {
                    manager.broadcast_to_user(
                        &participant.user_id,
                        WsMessage::new(
                            EventType::CollaboratorCursorMove,
                            json!({
                                "note_id": note_id,
                                "cursor": cursor,
                            }),
                        ),
                    );
                }
            }
        }
        Ok(())
    }

    /// Broadcast content update to all collaborators
    pub async fn broadcast_content_update(
        &self,
        note_id: &str,
        sender_id: &str,
        content: serde_json::Value,
        version: i64,
        ws_manager: &Arc<Mutex<ConnectionManager>>,
    ) -> Result<()> {
        if let Some(room) = self.rooms.get(note_id) {
            let manager = ws_manager.lock().await;
            for participant in &room.participants {
                if participant.user_id != sender_id {
                    manager.broadcast_to_user(
                        &participant.user_id,
                        WsMessage::new(
                            EventType::NoteContentUpdate,
                            json!({
                                "note_id": note_id,
                                "sender_id": sender_id,
                                "content": content,
                                "version": version,
                            }),
                        ),
                    );
                }
            }
        }
        Ok(())
    }

    /// Get active participants in a room
    pub fn get_room_participants(&self, note_id: &str) -> Vec<CollaboratorPresence> {
        self.rooms
            .get(note_id)
            .map(|room| room.participants.clone())
            .unwrap_or_default()
    }

    /// Check if user has access to note
    pub async fn check_access(
        &self,
        conn: &Connection,
        note_id: &str,
        user_id: &str,
    ) -> Result<Option<CollaboratorRole>> {
        // Check if user is a collaborator
        if let Some(collab) = NoteCollaborator::find_by_note_and_user(conn, note_id, user_id).await? {
            return Ok(Some(collab.role));
        }

        // Check share settings for public access
        if let Some(settings) = NoteShareSettings::find_by_note(conn, note_id).await? {
            if settings.owner_id == user_id {
                return Ok(Some(CollaboratorRole::Owner));
            }
            if settings.visibility == NoteVisibility::Public {
                return Ok(Some(CollaboratorRole::Viewer));
            }
        }

        Ok(None)
    }

    /// Invite a user to collaborate on a note
    pub async fn invite_collaborator(
        &self,
        conn: &Connection,
        note_id: &str,
        inviter_id: &str,
        inviter_name: &str,
        inviter_email: &str,
        invitee_email: &str,
        note_title: &str,
        role: CollaboratorRole,
        message: Option<String>,
        ws_manager: &Arc<Mutex<ConnectionManager>>,
    ) -> Result<NoteInvitation> {
        // Check if invitation already exists
        if let Some(_existing) = NoteInvitation::find_pending_by_note_and_email(conn, note_id, invitee_email).await? {
            anyhow::bail!("Invitation already pending for this email");
        }

        let invitation = NoteInvitation::create(
            conn,
            CreateInvitationRequest {
                note_id: note_id.to_string(),
                inviter_id: inviter_id.to_string(),
                inviter_email: inviter_email.to_string(),
                invitee_email: invitee_email.to_string(),
                role: role.clone(),
                message: message.clone(),
                expires_in_days: Some(7),
            },
        )
        .await?;

        // Send email notification to invitee
        match CollaborationEmailService::new() {
            Ok(email_service) => {
                match email_service.send_invitation(
                    invitee_email,
                    inviter_name,
                    inviter_email,
                    note_title,
                    &invitation.token,
                    &role,
                    message.as_deref(),
                ).await {
                    Ok(email_id) => {
                        log::info!("Invitation email sent successfully: {}", email_id);
                    }
                    Err(e) => {
                        log::error!("Failed to send invitation email: {}", e);
                    }
                }
            }
            Err(e) => {
                log::warn!("Email service not configured, skipping invitation email: {}", e);
            }
        }

        // Also broadcast via WebSocket if user is online
        let manager = ws_manager.lock().await;
        manager.broadcast_to_user(
            invitee_email,
            WsMessage::new(
                EventType::NoteInvitationReceived,
                json!({
                    "invitation_id": invitation.id,
                    "note_id": note_id,
                    "inviter_email": inviter_email,
                    "role": invitation.role,
                    "message": invitation.message,
                }),
            ),
        );

        Ok(invitation)
    }

    /// Accept an invitation
    /// Note: The caller must also register the collaborator in the central registry
    /// using app_state.register_collaborator() for cross-database access to work
    pub async fn accept_invitation(
        &self,
        conn: &Connection,
        invitation_id: &str,
        user_id: &str,
        user_email: &str,
        user_name: Option<String>,
        note_title: &str,
        ws_manager: &Arc<Mutex<ConnectionManager>>,
    ) -> Result<NoteCollaborator> {
        let invitation = NoteInvitation::find_by_id(conn, invitation_id).await?;
        
        if !invitation.is_valid() {
            anyhow::bail!("Invitation is no longer valid");
        }

        if invitation.invitee_email != user_email {
            anyhow::bail!("This invitation is for a different email address");
        }

        // Accept the invitation
        NoteInvitation::accept(conn, invitation_id, user_id).await?;

        // Create collaborator record in owner's database
        let collaborator = NoteCollaborator::create(
            conn,
            CreateCollaboratorRequest {
                note_id: invitation.note_id.clone(),
                user_id: user_id.to_string(),
                user_email: user_email.to_string(),
                user_name: user_name.clone(),
                role: invitation.role.clone(),
                invited_by: invitation.inviter_id.clone(),
            },
        )
        .await?;

        // Send email notification to inviter
        let app_url = std::env::var("APP_URL").unwrap_or_else(|_| "https://tradstry.com".to_string());
        let note_link = format!("{}/app/notebook/{}", app_url, invitation.note_id);
        let display_name = user_name.as_deref().unwrap_or(user_email);
        
        if let Ok(email_service) = CollaborationEmailService::new() {
            let _ = email_service.send_invitation_accepted(
                &invitation.inviter_email,
                display_name,
                user_email,
                note_title,
                &note_link,
            ).await;
        }

        // Also notify via WebSocket
        let manager = ws_manager.lock().await;
        manager.broadcast_to_user(
            &invitation.inviter_id,
            WsMessage::new(
                EventType::NoteInvitationAccepted,
                json!({
                    "invitation_id": invitation_id,
                    "note_id": invitation.note_id,
                    "user_id": user_id,
                    "user_email": user_email,
                }),
            ),
        );

        Ok(collaborator)
    }

    /// Update note visibility
    pub async fn update_visibility(
        &self,
        conn: &Connection,
        note_id: &str,
        owner_id: &str,
        visibility: NoteVisibility,
        ws_manager: &Arc<Mutex<ConnectionManager>>,
    ) -> Result<NoteShareSettings> {
        // Get or create share settings
        let settings = match NoteShareSettings::find_by_note(conn, note_id).await? {
            Some(s) => {
                if s.owner_id != owner_id {
                    anyhow::bail!("Only the owner can change visibility");
                }
                NoteShareSettings::update(
                    conn,
                    &s.id,
                    UpdateShareSettingsRequest {
                        visibility: Some(visibility.clone()),
                        allow_comments: None,
                        allow_copy: None,
                        password: None,
                    },
                )
                .await?
            }
            None => {
                NoteShareSettings::create(
                    conn,
                    CreateShareSettingsRequest {
                        note_id: note_id.to_string(),
                        owner_id: owner_id.to_string(),
                        visibility: Some(visibility.clone()),
                    },
                )
                .await?
            }
        };

        // Notify collaborators
        let collaborators = NoteCollaborator::find_by_note(conn, note_id).await?;
        let manager = ws_manager.lock().await;
        for collab in collaborators {
            manager.broadcast_to_user(
                &collab.user_id,
                WsMessage::new(
                    EventType::NoteVisibilityChanged,
                    json!({
                        "note_id": note_id,
                        "visibility": visibility,
                        "public_slug": settings.public_slug,
                    }),
                ),
            );
        }

        Ok(settings)
    }
}

impl Default for CollaborationService {
    fn default() -> Self {
        Self::new()
    }
}
