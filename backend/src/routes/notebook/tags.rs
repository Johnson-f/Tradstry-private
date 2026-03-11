use actix_web::{HttpRequest, HttpResponse, Result, web};
use log::{error, info};
use serde::{Deserialize, Serialize};

use crate::models::notebook::notebook_tags::{
    CreateTagRequest, NotebookTag, UpdateTagRequest,
};
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
            error!("[NotebookTags] JWT validation failed: {}", e);
            actix_web::error::ErrorUnauthorized("Invalid or expired token")
        })
}

#[derive(Debug, Serialize)]
pub struct TagResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<NotebookTag>,
}

#[derive(Debug, Serialize)]
pub struct TagListResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<Vec<NotebookTag>>,
}

#[derive(Debug, Deserialize)]
pub struct SetTagsRequest {
    pub tag_ids: Vec<String>,
}


/// Get all tags
pub async fn get_tags(
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

    match NotebookTag::find_all(&conn).await {
        Ok(tags) => Ok(HttpResponse::Ok().json(TagListResponse {
            success: true,
            message: format!("Retrieved {} tags", tags.len()),
            data: Some(tags),
        })),
        Err(e) => Ok(HttpResponse::InternalServerError().json(TagListResponse {
            success: false,
            message: format!("Failed to get tags: {}", e),
            data: None,
        })),
    }
}

/// Get a single tag by ID
pub async fn get_tag(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<String>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let tag_id = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    match NotebookTag::find_by_id(&conn, &tag_id).await {
        Ok(tag) => Ok(HttpResponse::Ok().json(TagResponse {
            success: true,
            message: "Tag retrieved".to_string(),
            data: Some(tag),
        })),
        Err(e) => Ok(HttpResponse::NotFound().json(TagResponse {
            success: false,
            message: format!("Tag not found: {}", e),
            data: None,
        })),
    }
}

/// Create a new tag
pub async fn create_tag(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    payload: web::Json<CreateTagRequest>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    match NotebookTag::create(&conn, payload.into_inner()).await {
        Ok(tag) => {
            info!("[NotebookTags] Tag created: {}", tag.id);
            Ok(HttpResponse::Created().json(TagResponse {
                success: true,
                message: "Tag created".to_string(),
                data: Some(tag),
            }))
        }
        Err(e) => {
            error!("[NotebookTags] Failed to create tag: {}", e);
            Ok(HttpResponse::InternalServerError().json(TagResponse {
                success: false,
                message: format!("Failed to create tag: {}", e),
                data: None,
            }))
        }
    }
}


/// Update a tag
pub async fn update_tag(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<String>,
    payload: web::Json<UpdateTagRequest>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let tag_id = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    match NotebookTag::update(&conn, &tag_id, payload.into_inner()).await {
        Ok(tag) => Ok(HttpResponse::Ok().json(TagResponse {
            success: true,
            message: "Tag updated".to_string(),
            data: Some(tag),
        })),
        Err(e) => Ok(HttpResponse::InternalServerError().json(TagResponse {
            success: false,
            message: format!("Failed to update tag: {}", e),
            data: None,
        })),
    }
}

/// Delete a tag
pub async fn delete_tag(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<String>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let tag_id = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    match NotebookTag::delete(&conn, &tag_id).await {
        Ok(true) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Tag deleted"
        }))),
        Ok(false) => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "message": "Tag not found"
        }))),
        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "message": format!("Failed to delete tag: {}", e)
        }))),
    }
}

/// Get tags for a note
pub async fn get_note_tags(
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

    match NotebookTag::get_note_tags(&conn, &note_id).await {
        Ok(tags) => Ok(HttpResponse::Ok().json(TagListResponse {
            success: true,
            message: format!("Retrieved {} tags for note", tags.len()),
            data: Some(tags),
        })),
        Err(e) => Ok(HttpResponse::InternalServerError().json(TagListResponse {
            success: false,
            message: format!("Failed to get note tags: {}", e),
            data: None,
        })),
    }
}


/// Set tags for a note (replaces existing)
pub async fn set_note_tags(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<String>,
    payload: web::Json<SetTagsRequest>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let note_id = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    match NotebookTag::set_note_tags(&conn, &note_id, &payload.tag_ids).await {
        Ok(()) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Note tags updated"
        }))),
        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "message": format!("Failed to set note tags: {}", e)
        }))),
    }
}

/// Add a tag to a note
pub async fn tag_note(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<(String, String)>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let (note_id, tag_id) = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    match NotebookTag::tag_note(&conn, &note_id, &tag_id).await {
        Ok(()) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Tag added to note"
        }))),
        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "message": format!("Failed to tag note: {}", e)
        }))),
    }
}

/// Remove a tag from a note
pub async fn untag_note(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<(String, String)>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let (note_id, tag_id) = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    match NotebookTag::untag_note(&conn, &note_id, &tag_id).await {
        Ok(true) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Tag removed from note"
        }))),
        Ok(false) => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "message": "Tag association not found"
        }))),
        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "message": format!("Failed to untag note: {}", e)
        }))),
    }
}

/// Configure notebook tags routes
pub fn configure_notebook_tags_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/notebook/tags")
            .route("", web::get().to(get_tags))
            .route("", web::post().to(create_tag))
            .route("/{id}", web::get().to(get_tag))
            .route("/{id}", web::put().to(update_tag))
            .route("/{id}", web::delete().to(delete_tag)),
    );
    cfg.service(
        web::scope("/api/notebook/notes/{note_id}/tags")
            .route("", web::get().to(get_note_tags))
            .route("", web::put().to(set_note_tags))
            .route("/{tag_id}", web::post().to(tag_note))
            .route("/{tag_id}", web::delete().to(untag_note)),
    );
}
