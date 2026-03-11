use actix_web::{web, HttpRequest, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::models::notebook::{
    CollaboratorRole, NoteCollaborator, NoteInvitation, NoteShareSettings, NoteVisibility,
    UpdateCollaboratorRequest,
};
use crate::service::notebook_engine::CollaborationService;
use crate::turso::config::SupabaseConfig;
use crate::turso::{AppState, SupabaseClaims};
use crate::websocket::ConnectionManager;

// ==================== Request/Response Types ====================

#[derive(Debug, Deserialize)]
pub struct InviteRequest {
    pub invitee_email: String,
    pub role: CollaboratorRole,
    pub message: Option<String>,
    pub inviter_name: Option<String>,
    pub note_title: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AcceptInvitationRequest {
    pub token: String,
    pub user_name: Option<String>,
    pub note_title: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateVisibilityRequest {
    pub visibility: NoteVisibility,
    pub allow_comments: Option<bool>,
    pub allow_copy: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct JoinRoomRequest {
    pub user_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CursorUpdateRequest {
    pub line: i32,
    pub column: i32,
    pub selection_start: Option<i32>,
    pub selection_end: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct ContentUpdateRequest {
    pub content: serde_json::Value,
    pub version: i64,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
}

// ==================== Helper Functions ====================

fn extract_token_from_request(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

async fn get_authenticated_user(
    req: &HttpRequest,
    supabase_config: &SupabaseConfig,
) -> Result<SupabaseClaims, actix_web::Error> {
    let token = extract_token_from_request(req)
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("Missing authorization token"))?;

    crate::turso::validate_supabase_jwt_token(&token, supabase_config)
        .await
        .map_err(|e| actix_web::error::ErrorUnauthorized(format!("Invalid token: {}", e)))
}

// ==================== Route Handlers ====================

/// Get collaborators for a note
pub async fn get_collaborators(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<String>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let note_id = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("Database not found"))?;

    let collaborators = NoteCollaborator::find_by_note(&conn, &note_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Collaborators retrieved".to_string(),
        data: Some(collaborators),
    }))
}

/// Invite a collaborator to a note
pub async fn invite_collaborator(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    collab_service: web::Data<Arc<CollaborationService>>,
    ws_manager: web::Data<Arc<Mutex<ConnectionManager>>>,
    path: web::Path<String>,
    payload: web::Json<InviteRequest>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let note_id = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("Database not found"))?;

    let user_email = claims.email.as_deref().unwrap_or("unknown@email.com");
    let inviter_name = payload.inviter_name.as_deref().unwrap_or(user_email);
    let note_title = payload.note_title.as_deref().unwrap_or("Untitled Note");

    let invitation = collab_service
        .invite_collaborator(
            &conn,
            &note_id,
            &claims.sub,
            inviter_name,
            user_email,
            &payload.invitee_email,
            note_title,
            payload.role.clone(),
            payload.message.clone(),
            ws_manager.get_ref(),
        )
        .await
        .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;

    // Register invitation in central registry so invitee can find it
    let _ = app_state
        .register_invitation(&invitation.token, &claims.sub, &note_id, &payload.invitee_email)
        .await;

    Ok(HttpResponse::Created().json(ApiResponse {
        success: true,
        message: "Invitation sent".to_string(),
        data: Some(invitation),
    }))
}

/// Get pending invitations for the current user
pub async fn get_my_invitations(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("Database not found"))?;

    let user_email = claims.email.as_deref().unwrap_or("unknown@email.com");
    let invitations = NoteInvitation::find_pending_by_email(&conn, user_email)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Invitations retrieved".to_string(),
        data: Some(invitations),
    }))
}

/// Accept an invitation
pub async fn accept_invitation(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    collab_service: web::Data<Arc<CollaborationService>>,
    ws_manager: web::Data<Arc<Mutex<ConnectionManager>>>,
    payload: web::Json<AcceptInvitationRequest>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    log::info!("[accept_invitation] Received request with token: {}", &payload.token);
    log::info!("[accept_invitation] user_name: {:?}, note_title: {:?}", &payload.user_name, &payload.note_title);
    
    let claims = match get_authenticated_user(&req, &supabase_config).await {
        Ok(c) => {
            log::info!("[accept_invitation] Authenticated user: {}", &c.sub);
            c
        }
        Err(e) => {
            log::error!("[accept_invitation] Auth failed: {:?}", e);
            return Err(e);
        }
    };

    // Look up invitation in central registry to find the inviter's database
    log::info!("[accept_invitation] Looking up invitation in registry...");
    let invitation_entry = match app_state.get_invitation_entry(&payload.token).await {
        Ok(Some(entry)) => {
            log::info!("[accept_invitation] Found invitation entry - inviter_id: {}, note_id: {}", &entry.inviter_id, &entry.note_id);
            entry
        }
        Ok(None) => {
            log::error!("[accept_invitation] Invitation not found in registry for token: {}", &payload.token);
            return Err(actix_web::error::ErrorNotFound("Invitation not found"));
        }
        Err(e) => {
            log::error!("[accept_invitation] Error looking up invitation: {}", e);
            return Err(actix_web::error::ErrorInternalServerError(e.to_string()));
        }
    };

    // Get the inviter's database connection (where the invitation is stored)
    log::info!("[accept_invitation] Getting inviter's database connection...");
    let conn = match app_state.get_user_db_connection(&invitation_entry.inviter_id).await {
        Ok(Some(c)) => {
            log::info!("[accept_invitation] Got inviter's database connection");
            c
        }
        Ok(None) => {
            log::error!("[accept_invitation] Inviter database not found for user: {}", &invitation_entry.inviter_id);
            return Err(actix_web::error::ErrorNotFound("Inviter database not found"));
        }
        Err(e) => {
            log::error!("[accept_invitation] Error getting inviter's database: {}", e);
            return Err(actix_web::error::ErrorInternalServerError(e.to_string()));
        }
    };

    // Find invitation by token in the inviter's database
    log::info!("[accept_invitation] Finding invitation by token in inviter's database...");
    let invitation = match NoteInvitation::find_by_token(&conn, &payload.token).await {
        Ok(Some(inv)) => {
            log::info!("[accept_invitation] Found invitation: id={}, note_id={}, status={:?}", &inv.id, &inv.note_id, &inv.status);
            inv
        }
        Ok(None) => {
            log::error!("[accept_invitation] Invitation not found in inviter's database for token: {}", &payload.token);
            return Err(actix_web::error::ErrorNotFound("Invitation not found"));
        }
        Err(e) => {
            log::error!("[accept_invitation] Error finding invitation: {}", e);
            return Err(actix_web::error::ErrorInternalServerError(e.to_string()));
        }
    };

    let note_title = payload.note_title.as_deref().unwrap_or("Untitled Note");
    let user_email = claims.email.as_deref().unwrap_or("unknown@email.com");
    log::info!("[accept_invitation] Accepting invitation for user: {} ({})", &claims.sub, user_email);

    let collaborator = match collab_service
        .accept_invitation(
            &conn,
            &invitation.id,
            &claims.sub,
            user_email,
            payload.user_name.clone(),
            note_title,
            ws_manager.get_ref(),
        )
        .await
    {
        Ok(c) => {
            log::info!("[accept_invitation] Successfully accepted invitation, collaborator_id: {}", &c.id);
            c
        }
        Err(e) => {
            log::error!("[accept_invitation] Failed to accept invitation: {}", e);
            return Err(actix_web::error::ErrorBadRequest(e.to_string()));
        }
    };

    // Register collaborator in central registry for cross-database access
    let role_str = format!("{:?}", collaborator.role).to_lowercase();
    if let Err(e) = app_state.register_collaborator(
        &collaborator.id,
        &invitation_entry.note_id,
        &invitation_entry.inviter_id,  // owner_id
        &claims.sub,                    // collaborator_id
        &role_str,
    ).await {
        log::error!("[accept_invitation] Failed to register collaborator in central registry: {}", e);
        // Don't fail the request, the collaborator record is already created in owner's DB
    } else {
        log::info!("[accept_invitation] Registered collaborator in central registry");
    }

    // Remove invitation from central registry
    let _ = app_state.unregister_invitation(&payload.token).await;

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Invitation accepted".to_string(),
        data: Some(collaborator),
    }))
}

/// Decline an invitation
pub async fn decline_invitation(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<String>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let invitation_id = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("Database not found"))?;

    let invitation = NoteInvitation::decline(&conn, &invitation_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Invitation declined".to_string(),
        data: Some(invitation),
    }))
}

/// Remove a collaborator from a note
pub async fn remove_collaborator(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<(String, String)>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let (note_id, collaborator_id) = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("Database not found"))?;

    // Verify requester has permission (owner or self-removal)
    let collaborator = NoteCollaborator::find_by_id(&conn, &collaborator_id)
        .await
        .map_err(|e| actix_web::error::ErrorNotFound(e.to_string()))?;

    if collaborator.note_id != note_id {
        return Err(actix_web::error::ErrorBadRequest("Collaborator not in this note"));
    }

    // Check if user is owner or removing themselves
    let requester_collab = NoteCollaborator::find_by_note_and_user(&conn, &note_id, &claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    let is_owner = requester_collab.as_ref().map(|c| c.role == CollaboratorRole::Owner).unwrap_or(false);
    let is_self = collaborator.user_id == claims.sub;

    if !is_owner && !is_self {
        return Err(actix_web::error::ErrorForbidden("Only owner can remove collaborators"));
    }

    NoteCollaborator::delete(&conn, &collaborator_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(ApiResponse::<()> {
        success: true,
        message: "Collaborator removed".to_string(),
        data: None,
    }))
}

/// Update collaborator role
pub async fn update_collaborator_role(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<(String, String)>,
    payload: web::Json<UpdateCollaboratorRequest>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let (note_id, collaborator_id) = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("Database not found"))?;

    // Verify requester is owner
    let requester_collab = NoteCollaborator::find_by_note_and_user(&conn, &note_id, &claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?
        .ok_or_else(|| actix_web::error::ErrorForbidden("Not a collaborator"))?;

    if requester_collab.role != CollaboratorRole::Owner {
        return Err(actix_web::error::ErrorForbidden("Only owner can update roles"));
    }

    let collaborator = NoteCollaborator::update(&conn, &collaborator_id, payload.into_inner())
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Collaborator updated".to_string(),
        data: Some(collaborator),
    }))
}

/// Get share settings for a note
pub async fn get_share_settings(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<String>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let note_id = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("Database not found"))?;

    let settings = NoteShareSettings::find_by_note(&conn, &note_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Share settings retrieved".to_string(),
        data: settings,
    }))
}

/// Update note visibility
pub async fn update_visibility(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    collab_service: web::Data<Arc<CollaborationService>>,
    ws_manager: web::Data<Arc<Mutex<ConnectionManager>>>,
    path: web::Path<String>,
    payload: web::Json<UpdateVisibilityRequest>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let note_id = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("Database not found"))?;

    // Get the old settings to check if we need to unregister
    let old_settings = NoteShareSettings::find_by_note(&conn, &note_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    let settings = collab_service
        .update_visibility(&conn, &note_id, &claims.sub, payload.visibility.clone(), ws_manager.get_ref())
        .await
        .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;

    // Handle public notes registry
    if payload.visibility == NoteVisibility::Public {
        // Register in central registry
        if let Some(ref slug) = settings.public_slug {
            // Get note title
            let note = crate::models::notebook::NotebookNote::find_by_id(&conn, &note_id)
                .await
                .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
            
            let _ = app_state
                .register_public_note(slug, &note_id, &claims.sub, &note.title)
                .await;
        }
    } else if let Some(old) = old_settings {
        // Unregister from central registry if was previously public
        if old.visibility == NoteVisibility::Public {
            if let Some(ref slug) = old.public_slug {
                let _ = app_state.unregister_public_note(slug).await;
            }
        }
    }

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Visibility updated".to_string(),
        data: Some(settings),
    }))
}

/// Get public note by slug
pub async fn get_public_note(
    app_state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let slug = path.into_inner();

    // Look up the note in the central registry
    let entry = app_state
        .get_public_note_entry(&slug)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    let entry = match entry {
        Some(e) => e,
        None => {
            return Ok(HttpResponse::NotFound().json(ApiResponse::<()> {
                success: false,
                message: "Note not found or not public".to_string(),
                data: None,
            }));
        }
    };

    // Get the owner's database connection
    let conn = app_state
        .get_user_db_connection(&entry.owner_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("Owner database not found"))?;

    // Verify the note is still public
    let share_settings = NoteShareSettings::find_by_slug(&conn, &slug)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    let share_settings = match share_settings {
        Some(s) if s.visibility == NoteVisibility::Public => s,
        _ => {
            return Ok(HttpResponse::NotFound().json(ApiResponse::<()> {
                success: false,
                message: "Note not found or not public".to_string(),
                data: None,
            }));
        }
    };

    // Get the actual note content
    let note = crate::models::notebook::NotebookNote::find_by_id(&conn, &entry.note_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    // Increment view count
    let _ = NoteShareSettings::increment_view_count(&conn, &share_settings.id).await;

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Public note retrieved".to_string(),
        data: Some(json!({
            "id": note.id,
            "title": note.title,
            "content": note.content,
            "content_plain_text": note.content_plain_text,
            "word_count": note.word_count,
            "created_at": note.created_at,
            "updated_at": note.updated_at,
            "view_count": share_settings.view_count + 1,
            "allow_comments": share_settings.allow_comments,
            "allow_copy": share_settings.allow_copy,
        })),
    }))
}

/// Join a collaboration room
pub async fn join_room(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    collab_service: web::Data<Arc<CollaborationService>>,
    ws_manager: web::Data<Arc<Mutex<ConnectionManager>>>,
    path: web::Path<String>,
    payload: web::Json<JoinRoomRequest>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let note_id = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("Database not found"))?;

    // Check access
    let role = collab_service
        .check_access(&conn, &note_id, &claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?
        .ok_or_else(|| actix_web::error::ErrorForbidden("No access to this note"))?;

    let user_email = claims.email.as_deref().unwrap_or("unknown@email.com");
    let room = collab_service
        .join_room(
            &note_id,
            &claims.sub,
            user_email,
            payload.user_name.clone(),
            role,
            ws_manager.get_ref(),
        )
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Joined room".to_string(),
        data: Some(json!({
            "note_id": room.note_id,
            "participants": room.participants,
        })),
    }))
}

/// Leave a collaboration room
pub async fn leave_room(
    _app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    collab_service: web::Data<Arc<CollaborationService>>,
    ws_manager: web::Data<Arc<Mutex<ConnectionManager>>>,
    path: web::Path<String>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let note_id = path.into_inner();

    collab_service
        .leave_room(&note_id, &claims.sub, ws_manager.get_ref())
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(ApiResponse::<()> {
        success: true,
        message: "Left room".to_string(),
        data: None,
    }))
}

/// Update cursor position
pub async fn update_cursor(
    supabase_config: web::Data<SupabaseConfig>,
    collab_service: web::Data<Arc<CollaborationService>>,
    ws_manager: web::Data<Arc<Mutex<ConnectionManager>>>,
    path: web::Path<String>,
    payload: web::Json<CursorUpdateRequest>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let note_id = path.into_inner();

    let cursor = crate::service::notebook_engine::CursorPosition {
        user_id: claims.sub.clone(),
        user_name: claims.email.clone().unwrap_or_else(|| "Unknown".to_string()),
        user_color: "#4ECDC4".to_string(), // Will be assigned by service
        line: payload.line,
        column: payload.column,
        selection_start: payload.selection_start,
        selection_end: payload.selection_end,
    };

    collab_service
        .update_cursor(&note_id, &claims.sub, cursor, ws_manager.get_ref())
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(ApiResponse::<()> {
        success: true,
        message: "Cursor updated".to_string(),
        data: None,
    }))
}

/// Broadcast content update
pub async fn broadcast_content(
    supabase_config: web::Data<SupabaseConfig>,
    collab_service: web::Data<Arc<CollaborationService>>,
    ws_manager: web::Data<Arc<Mutex<ConnectionManager>>>,
    path: web::Path<String>,
    payload: web::Json<ContentUpdateRequest>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let note_id = path.into_inner();

    collab_service
        .broadcast_content_update(
            &note_id,
            &claims.sub,
            payload.content.clone(),
            payload.version,
            ws_manager.get_ref(),
        )
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(ApiResponse::<()> {
        success: true,
        message: "Content broadcasted".to_string(),
        data: None,
    }))
}

/// Get active participants in a room
pub async fn get_participants(
    supabase_config: web::Data<SupabaseConfig>,
    collab_service: web::Data<Arc<CollaborationService>>,
    path: web::Path<String>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let _claims = get_authenticated_user(&req, &supabase_config).await?;
    let note_id = path.into_inner();

    let participants = collab_service.get_room_participants(&note_id);

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Participants retrieved".to_string(),
        data: Some(participants),
    }))
}

// ==================== Route Configuration ====================

pub fn configure_collaboration_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/notebook/collaboration")
            // Invitations
            .route("/invitations", web::get().to(get_my_invitations))
            .route("/invitations/accept", web::post().to(accept_invitation))
            .route("/invitations/{id}/decline", web::post().to(decline_invitation))
            // Note-specific collaboration
            .route("/notes/{note_id}/collaborators", web::get().to(get_collaborators))
            .route("/notes/{note_id}/invite", web::post().to(invite_collaborator))
            .route("/notes/{note_id}/collaborators/{collab_id}", web::delete().to(remove_collaborator))
            .route("/notes/{note_id}/collaborators/{collab_id}", web::patch().to(update_collaborator_role))
            // Sharing
            .route("/notes/{note_id}/share", web::get().to(get_share_settings))
            .route("/notes/{note_id}/visibility", web::put().to(update_visibility))
            .route("/public/{slug}", web::get().to(get_public_note))
            // Real-time collaboration
            .route("/notes/{note_id}/join", web::post().to(join_room))
            .route("/notes/{note_id}/leave", web::post().to(leave_room))
            .route("/notes/{note_id}/cursor", web::post().to(update_cursor))
            .route("/notes/{note_id}/content", web::post().to(broadcast_content))
            .route("/notes/{note_id}/participants", web::get().to(get_participants))
    );
}
