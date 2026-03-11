use actix_web::{HttpRequest, HttpResponse, Result, web};
use log::{error, info};
use serde::Serialize;

use crate::models::notebook::notebook_images::{
    CreateImageRequest, NotebookImage, UpdateImageRequest,
};
use crate::service::notebook_engine::images::{NotebookImageService, UploadImageRequest};
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
            error!("[NotebookImages] JWT validation failed: {}", e);
            actix_web::error::ErrorUnauthorized("Invalid or expired token")
        })
}

#[derive(Debug, Serialize)]
pub struct ImageResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<NotebookImage>,
}

#[derive(Debug, Serialize)]
pub struct ImageListResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<Vec<NotebookImage>>,
}


/// Get all images for a note
pub async fn get_note_images(
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

    match NotebookImage::find_by_note(&conn, &note_id).await {
        Ok(images) => Ok(HttpResponse::Ok().json(ImageListResponse {
            success: true,
            message: format!("Retrieved {} images", images.len()),
            data: Some(images),
        })),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ImageListResponse {
            success: false,
            message: format!("Failed to get images: {}", e),
            data: None,
        })),
    }
}

/// Get a single image by ID
pub async fn get_image(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<String>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let image_id = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    match NotebookImage::find_by_id(&conn, &image_id).await {
        Ok(image) => Ok(HttpResponse::Ok().json(ImageResponse {
            success: true,
            message: "Image retrieved".to_string(),
            data: Some(image),
        })),
        Err(e) => Ok(HttpResponse::NotFound().json(ImageResponse {
            success: false,
            message: format!("Image not found: {}", e),
            data: None,
        })),
    }
}

/// Upload an image to Supabase Storage and save metadata
pub async fn upload_image(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    payload: web::Json<UploadImageRequest>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = &claims.sub;

    let conn = app_state
        .get_user_db_connection(user_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    // Create image service
    let image_service = NotebookImageService::new(&supabase_config);

    match image_service.upload_and_save(&conn, user_id, payload.into_inner()).await {
        Ok(image) => {
            info!("[NotebookImages] Image uploaded: {}", image.id);
            Ok(HttpResponse::Created().json(ImageResponse {
                success: true,
                message: "Image uploaded".to_string(),
                data: Some(image),
            }))
        }
        Err(e) => {
            error!("[NotebookImages] Failed to upload image: {}", e);
            Ok(HttpResponse::InternalServerError().json(ImageResponse {
                success: false,
                message: format!("Failed to upload image: {}", e),
                data: None,
            }))
        }
    }
}


/// Create image metadata (for external URLs, not uploaded)
pub async fn create_image(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    payload: web::Json<CreateImageRequest>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    match NotebookImage::create(&conn, payload.into_inner()).await {
        Ok(image) => {
            info!("[NotebookImages] Image created: {}", image.id);
            Ok(HttpResponse::Created().json(ImageResponse {
                success: true,
                message: "Image created".to_string(),
                data: Some(image),
            }))
        }
        Err(e) => {
            error!("[NotebookImages] Failed to create image: {}", e);
            Ok(HttpResponse::InternalServerError().json(ImageResponse {
                success: false,
                message: format!("Failed to create image: {}", e),
                data: None,
            }))
        }
    }
}

/// Update image metadata
pub async fn update_image(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<String>,
    payload: web::Json<UpdateImageRequest>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let image_id = path.into_inner();

    let conn = app_state
        .get_user_db_connection(&claims.sub)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    match NotebookImage::update(&conn, &image_id, payload.into_inner()).await {
        Ok(image) => Ok(HttpResponse::Ok().json(ImageResponse {
            success: true,
            message: "Image updated".to_string(),
            data: Some(image),
        })),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ImageResponse {
            success: false,
            message: format!("Failed to update image: {}", e),
            data: None,
        })),
    }
}

/// Delete an image (from storage and database)
pub async fn delete_image(
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    path: web::Path<String>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let user_id = &claims.sub;
    let image_id = path.into_inner();

    let conn = app_state
        .get_user_db_connection(user_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

    // Create image service for storage deletion
    let image_service = NotebookImageService::new(&supabase_config);

    match image_service.delete_image(&conn, user_id, &image_id).await {
        Ok(true) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Image deleted"
        }))),
        Ok(false) => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "message": "Image not found"
        }))),
        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "message": format!("Failed to delete image: {}", e)
        }))),
    }
}

/// Configure notebook images routes
pub fn configure_notebook_images_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/notebook/images")
            .route("", web::post().to(create_image))
            .route("/upload", web::post().to(upload_image))
            .route("/{id}", web::get().to(get_image))
            .route("/{id}", web::put().to(update_image))
            .route("/{id}", web::delete().to(delete_image)),
    );
    cfg.service(
        web::resource("/api/notebook/notes/{note_id}/images")
            .route(web::get().to(get_note_images)),
    );
}
