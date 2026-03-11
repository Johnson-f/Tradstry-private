use actix_web::{HttpRequest, HttpResponse, Result, web};
use log::{error, info};
use serde::Serialize;

use crate::models::notebook::notebook_folders::{
    CreateFolderRequest, NotebookFolder, UpdateFolderRequest,
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
            error!("[NotebookFolders] JWT validation failed: {}", e);
            actix_web::error::ErrorUnauthorized("Invalid or expired token")
        })
}

#[derive(Debug, Serialize)]
pub struct FolderResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<NotebookFolder>,
}

#[derive(Debug, Serialize)]
pub struct FolderListResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<Vec<NotebookFolder>>,
}


/// Get all folders
pub async fn get_folders(
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

    match NotebookFolder::find_all(&conn).await {
        Ok(folders) => Ok(HttpResponse::Ok().json(FolderListResponse {
            success: true,
            message: format!("Retrieved {} folders", folders.len()),
            data: Some(folders),
        })),
        Err(e) => Ok(HttpResponse::InternalServerError().json(FolderListResponse {
            success: false,
            message: format!("Failed to get folders: {}", e),
            data: None,
        })),
    }
}

/// Get a single folder by ID
pub async fn get_folder(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<String>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let folder_id = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    match NotebookFolder::find_by_id(&conn, &folder_id).await {
        Ok(folder) => Ok(HttpResponse::Ok().json(FolderResponse {
            success: true,
            message: "Folder retrieved".to_string(),
            data: Some(folder),
        })),
        Err(e) => Ok(HttpResponse::NotFound().json(FolderResponse {
            success: false,
            message: format!("Folder not found: {}", e),
            data: None,
        })),
    }
}

/// Create a new folder
pub async fn create_folder(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    payload: web::Json<CreateFolderRequest>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    match NotebookFolder::create(&conn, payload.into_inner()).await {
        Ok(folder) => {
            info!("[NotebookFolders] Folder created: {}", folder.id);
            Ok(HttpResponse::Created().json(FolderResponse {
                success: true,
                message: "Folder created".to_string(),
                data: Some(folder),
            }))
        }
        Err(e) => {
            error!("[NotebookFolders] Failed to create folder: {}", e);
            Ok(HttpResponse::InternalServerError().json(FolderResponse {
                success: false,
                message: format!("Failed to create folder: {}", e),
                data: None,
            }))
        }
    }
}


/// Update a folder
pub async fn update_folder(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<String>,
    payload: web::Json<UpdateFolderRequest>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let folder_id = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    match NotebookFolder::update(&conn, &folder_id, payload.into_inner()).await {
        Ok(folder) => Ok(HttpResponse::Ok().json(FolderResponse {
            success: true,
            message: "Folder updated".to_string(),
            data: Some(folder),
        })),
        Err(e) => Ok(HttpResponse::InternalServerError().json(FolderResponse {
            success: false,
            message: format!("Failed to update folder: {}", e),
            data: None,
        })),
    }
}

/// Delete a folder (cascades to notes)
pub async fn delete_folder(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<String>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let folder_id = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    match NotebookFolder::delete(&conn, &folder_id).await {
        Ok(true) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Folder deleted"
        }))),
        Ok(false) => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "message": "Folder not found"
        }))),
        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "message": format!("Failed to delete folder: {}", e)
        }))),
    }
}

/// Get favorite folders
pub async fn get_favorite_folders(
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

    match NotebookFolder::find_favorites(&conn).await {
        Ok(folders) => Ok(HttpResponse::Ok().json(FolderListResponse {
            success: true,
            message: format!("Retrieved {} favorite folders", folders.len()),
            data: Some(folders),
        })),
        Err(e) => Ok(HttpResponse::InternalServerError().json(FolderListResponse {
            success: false,
            message: format!("Failed to get favorite folders: {}", e),
            data: None,
        })),
    }
}

/// Configure notebook folders routes
pub fn configure_notebook_folders_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/notebook/folders")
            .route("", web::get().to(get_folders))
            .route("", web::post().to(create_folder))
            .route("/favorites", web::get().to(get_favorite_folders))
            .route("/{id}", web::get().to(get_folder))
            .route("/{id}", web::put().to(update_folder))
            .route("/{id}", web::delete().to(delete_folder)),
    );
}
