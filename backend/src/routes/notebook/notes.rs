use actix_web::{HttpRequest, HttpResponse, Result, web};
use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::models::notebook::notebook_notes::{
    CreateNoteRequest, NoteQuery, NotebookNote, UpdateNoteRequest,
};
use crate::service::notebook_engine::images::NotebookImageService;
use crate::turso::auth::validate_supabase_jwt_token;
use crate::turso::config::SupabaseConfig;
use crate::turso::{AppState, SupabaseClaims};

fn extract_token_from_request(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("authorization")?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(|s| s.to_string())
}

async fn get_authenticated_user(
    req: &HttpRequest,
    supabase_config: &SupabaseConfig,
) -> Result<SupabaseClaims, actix_web::Error> {
    let token = extract_token_from_request(req)
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("Missing authorization token"))?;

    validate_supabase_jwt_token(&token, supabase_config)
        .await
        .map_err(|e| {
            error!("[NotebookNotes] JWT validation failed: {}", e);
            actix_web::error::ErrorUnauthorized("Invalid or expired token")
        })
}

#[derive(Debug, Serialize)]
pub struct NoteResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<NotebookNote>,
}

#[derive(Debug, Serialize)]
pub struct NoteListResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<Vec<NotebookNote>>,
}


/// Get all notes with optional filtering
pub async fn get_notes(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    query: web::Query<NoteQuery>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = &claims.sub;

    let conn = app_state
        .get_user_db_connection(user_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    match NotebookNote::find_all(&conn, Some(query.into_inner())).await {
        Ok(notes) => Ok(HttpResponse::Ok().json(NoteListResponse {
            success: true,
            message: format!("Retrieved {} notes", notes.len()),
            data: Some(notes),
        })),
        Err(e) => {
            error!("[NotebookNotes] Failed to get notes: {}", e);
            Ok(HttpResponse::InternalServerError().json(NoteListResponse {
                success: false,
                message: format!("Failed to get notes: {}", e),
                data: None,
            }))
        }
    }
}

/// Get a single note by ID
/// Supports cross-database access for shared notes (Option A collaboration)
pub async fn get_note(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<String>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let note_id = path.into_inner();

    // First, try to get the note from user's own database
    let user_conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    match NotebookNote::find_by_id(&user_conn, &note_id).await {
        Ok(note) => {
            return Ok(HttpResponse::Ok().json(NoteResponse {
                success: true,
                message: "Note retrieved".to_string(),
                data: Some(note),
            }));
        }
        Err(_) => {
            // Note not found in user's database, check if they're a collaborator
            info!("[get_note] Note {} not found in user's DB, checking collaborator registry", note_id);
        }
    }

    // Check if user is a collaborator on this note (cross-database access)
    match app_state.get_collaborator_entry(&note_id, &claims.sub).await {
        Ok(Some(collab_entry)) => {
            info!("[get_note] User {} is a collaborator on note {}, fetching from owner {}", 
                  &claims.sub, &note_id, &collab_entry.owner_id);
            
            // Get the owner's database connection
            let owner_conn = app_state
                .get_user_db_connection(&collab_entry.owner_id)
                .await
                .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
                .ok_or_else(|| actix_web::error::ErrorNotFound("Owner database not found"))?;

            match NotebookNote::find_by_id(&owner_conn, &note_id).await {
                Ok(note) => Ok(HttpResponse::Ok().json(NoteResponse {
                    success: true,
                    message: "Note retrieved (shared)".to_string(),
                    data: Some(note),
                })),
                Err(e) => Ok(HttpResponse::NotFound().json(NoteResponse {
                    success: false,
                    message: format!("Shared note not found: {}", e),
                    data: None,
                })),
            }
        }
        Ok(None) => {
            // Not a collaborator, note doesn't exist for this user
            Ok(HttpResponse::NotFound().json(NoteResponse {
                success: false,
                message: "Note not found".to_string(),
                data: None,
            }))
        }
        Err(e) => {
            error!("[get_note] Error checking collaborator registry: {}", e);
            Ok(HttpResponse::NotFound().json(NoteResponse {
                success: false,
                message: "Note not found".to_string(),
                data: None,
            }))
        }
    }
}


/// Create a new note
pub async fn create_note(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    payload: web::Json<CreateNoteRequest>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    match NotebookNote::create(&conn, payload.into_inner()).await {
        Ok(note) => {
            info!("[NotebookNotes] Note created: {}", note.id);
            Ok(HttpResponse::Created().json(NoteResponse {
                success: true,
                message: "Note created".to_string(),
                data: Some(note),
            }))
        }
        Err(e) => {
            error!("[NotebookNotes] Failed to create note: {}", e);
            Ok(HttpResponse::InternalServerError().json(NoteResponse {
                success: false,
                message: format!("Failed to create note: {}", e),
                data: None,
            }))
        }
    }
}

/// Update a note
/// Supports cross-database access for shared notes (Option A collaboration)
pub async fn update_note(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<String>,
    payload: web::Json<UpdateNoteRequest>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let note_id = path.into_inner();

    // First, try to update in user's own database
    let user_conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    // Check if note exists in user's database
    if NotebookNote::find_by_id(&user_conn, &note_id).await.is_ok() {
        // Note is in user's own database, update it there
        match NotebookNote::update(&user_conn, &note_id, payload.into_inner()).await {
            Ok(note) => {
                return Ok(HttpResponse::Ok().json(NoteResponse {
                    success: true,
                    message: "Note updated".to_string(),
                    data: Some(note),
                }));
            }
            Err(e) => {
                error!("[NotebookNotes] Failed to update note: {}", e);
                return Ok(HttpResponse::InternalServerError().json(NoteResponse {
                    success: false,
                    message: format!("Failed to update note: {}", e),
                    data: None,
                }));
            }
        }
    }

    // Note not in user's database, check if they're a collaborator with edit access
    match app_state.get_collaborator_entry(&note_id, &claims.sub).await {
        Ok(Some(collab_entry)) => {
            // Check if collaborator has edit permissions (editor or owner role)
            if collab_entry.role != "editor" && collab_entry.role != "owner" {
                return Ok(HttpResponse::Forbidden().json(NoteResponse {
                    success: false,
                    message: "You don't have permission to edit this note".to_string(),
                    data: None,
                }));
            }

            info!("[update_note] User {} updating shared note {} owned by {}", 
                  &claims.sub, &note_id, &collab_entry.owner_id);
            
            // Get the owner's database connection
            let owner_conn = app_state
                .get_user_db_connection(&collab_entry.owner_id)
                .await
                .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
                .ok_or_else(|| actix_web::error::ErrorNotFound("Owner database not found"))?;

            match NotebookNote::update(&owner_conn, &note_id, payload.into_inner()).await {
                Ok(note) => Ok(HttpResponse::Ok().json(NoteResponse {
                    success: true,
                    message: "Note updated (shared)".to_string(),
                    data: Some(note),
                })),
                Err(e) => {
                    error!("[NotebookNotes] Failed to update shared note: {}", e);
                    Ok(HttpResponse::InternalServerError().json(NoteResponse {
                        success: false,
                        message: format!("Failed to update note: {}", e),
                        data: None,
                    }))
                }
            }
        }
        Ok(None) => {
            // Not a collaborator, note doesn't exist for this user
            Ok(HttpResponse::NotFound().json(NoteResponse {
                success: false,
                message: "Note not found".to_string(),
                data: None,
            }))
        }
        Err(e) => {
            error!("[update_note] Error checking collaborator registry: {}", e);
            Ok(HttpResponse::NotFound().json(NoteResponse {
                success: false,
                message: "Note not found".to_string(),
                data: None,
            }))
        }
    }
}


/// Soft delete a note
pub async fn delete_note(
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
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    match NotebookNote::soft_delete(&conn, &note_id).await {
        Ok(true) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Note deleted"
        }))),
        Ok(false) => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "message": "Note not found"
        }))),
        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "message": format!("Failed to delete note: {}", e)
        }))),
    }
}

/// Restore a soft-deleted note
pub async fn restore_note(
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
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    match NotebookNote::restore(&conn, &note_id).await {
        Ok(true) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Note restored"
        }))),
        Ok(false) => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "message": "Note not found"
        }))),
        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "message": format!("Failed to restore note: {}", e)
        }))),
    }
}

/// Get deleted notes (trash)
pub async fn get_deleted_notes(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    match NotebookNote::find_deleted(&conn).await {
        Ok(notes) => Ok(HttpResponse::Ok().json(NoteListResponse {
            success: true,
            message: format!("Retrieved {} deleted notes", notes.len()),
            data: Some(notes),
        })),
        Err(e) => Ok(HttpResponse::InternalServerError().json(NoteListResponse {
            success: false,
            message: format!("Failed to get deleted notes: {}", e),
            data: None,
        })),
    }
}

/// Permanently delete a note (including all images from storage and database)
pub async fn permanent_delete_note(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<String>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = &claims.sub;
    let note_id = path.into_inner();

    let conn = app_state
        .get_user_db_connection(user_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    // First, delete all images associated with this note (from storage and database)
    let image_service = NotebookImageService::new(&supabase_config);
    match image_service.delete_note_images(&conn, user_id, &note_id).await {
        Ok(deleted_count) => {
            info!("[NotebookNotes] Deleted {} images for note {}", deleted_count, note_id);
        }
        Err(e) => {
            // Log but don't fail - we still want to delete the note
            error!("[NotebookNotes] Failed to delete images for note {}: {}", note_id, e);
        }
    }

    // Then permanently delete the note
    match NotebookNote::permanent_delete(&conn, &note_id).await {
        Ok(true) => {
            info!("[NotebookNotes] Note {} permanently deleted", note_id);
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "message": "Note permanently deleted"
            })))
        }
        Ok(false) => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "message": "Note not found"
        }))),
        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "message": format!("Failed to permanently delete note: {}", e)
        }))),
    }
}

/// Request to set tags on a note
#[derive(Debug, Deserialize)]
pub struct SetNoteTagsRequest {
    pub tag_ids: Vec<String>,
}

/// Set tags on a note
pub async fn set_note_tags(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<String>,
    payload: web::Json<SetNoteTagsRequest>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let note_id = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    // Convert tag_ids to JSON Value
    let tags_value: Value = serde_json::to_value(&payload.tag_ids)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid tags: {}", e)))?;

    // Update the note with the new tags
    let update_req = UpdateNoteRequest {
        title: None,
        content: None,
        content_plain_text: None,
        is_pinned: None,
        is_archived: None,
        is_deleted: None,
        folder_id: None,
        tags: Some(tags_value),
        y_state: None,
    };

    match NotebookNote::update(&conn, &note_id, update_req).await {
        Ok(note) => {
            info!("[NotebookNotes] Tags updated for note: {}", note_id);
            Ok(HttpResponse::Ok().json(NoteResponse {
                success: true,
                message: "Tags updated".to_string(),
                data: Some(note),
            }))
        }
        Err(e) => {
            error!("[NotebookNotes] Failed to update tags for note {}: {}", note_id, e);
            Ok(HttpResponse::InternalServerError().json(NoteResponse {
                success: false,
                message: format!("Failed to update tags: {}", e),
                data: None,
            }))
        }
    }
}

/// Configure notebook notes routes
pub fn configure_notebook_notes_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/notebook/notes")
            .route("", web::get().to(get_notes))
            .route("", web::post().to(create_note))
            .route("/trash", web::get().to(get_deleted_notes))
            .route("/{id}", web::get().to(get_note))
            .route("/{id}", web::put().to(update_note))
            .route("/{id}", web::delete().to(delete_note))
            .route("/{id}/restore", web::post().to(restore_note))
            .route("/{id}/permanent", web::delete().to(permanent_delete_note))
            .route("/{id}/tags", web::put().to(set_note_tags)),
    );
}
