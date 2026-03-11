use anyhow::Result;
use base64::{engine::general_purpose::STANDARD, Engine};
use log::{debug, error, info};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::notebook::{CreateImageRequest, NotebookImage};
use crate::turso::config::SupabaseConfig;

const NOTEBOOK_IMAGES_BUCKET: &str = "notebook-images";

/// Supabase Storage service for notebook images
#[derive(Clone)]
pub struct NotebookImageStorage {
    client: Client,
    supabase_url: String,
    service_role_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadImageRequest {
    pub note_id: String,
    pub file_data: String, // Base64 encoded image data
    pub file_name: String,
    pub content_type: String,
    pub alt_text: Option<String>,
    pub caption: Option<String>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub position: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadedImage {
    pub id: String,
    pub note_id: String,
    pub src: String,
    pub alt_text: Option<String>,
    pub caption: Option<String>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub position: i64,
    pub storage_path: String,
}

#[derive(Debug, Deserialize)]
struct SupabaseUploadResponse {
    #[serde(rename = "Key")]
    key: Option<String>,
    #[serde(rename = "Id")]
    id: Option<String>,
}

impl NotebookImageStorage {
    pub fn new(config: &SupabaseConfig) -> Self {
        Self {
            client: Client::new(),
            supabase_url: config.project_url.clone(),
            service_role_key: config.service_role_key.clone(),
        }
    }

    pub fn from_env() -> Result<Self> {
        // Load .env file if present (no-op if already loaded or missing)
        dotenvy::dotenv().ok();

        let supabase_url = std::env::var("SUPABASE_URL")
            .map_err(|_| anyhow::anyhow!("SUPABASE_URL not set"))?;
        let service_role_key = std::env::var("SUPABASE_SERVICE_ROLE_KEY")
            .map_err(|_| anyhow::anyhow!("SUPABASE_SERVICE_ROLE_KEY not set"))?;

        Ok(Self {
            client: Client::new(),
            supabase_url,
            service_role_key,
        })
    }

    /// Build the storage path: {user_id}/{note_id}/{image_id}.{ext}
    fn build_storage_path(user_id: &str, note_id: &str, image_id: &str, file_name: &str) -> String {
        let extension = file_name
            .rsplit('.')
            .next()
            .unwrap_or("png");
        format!("{}/{}/{}.{}", user_id, note_id, image_id, extension)
    }

    /// Upload an image to Supabase Storage
    pub async fn upload_image(
        &self,
        user_id: &str,
        req: UploadImageRequest,
    ) -> Result<UploadedImage> {
        let image_id = Uuid::new_v4().to_string();
        let storage_path = Self::build_storage_path(user_id, &req.note_id, &image_id, &req.file_name);

        info!(
            "Uploading image to Supabase Storage: bucket={}, path={}",
            NOTEBOOK_IMAGES_BUCKET, storage_path
        );

        // Decode base64 image data
        let file_bytes = STANDARD
            .decode(&req.file_data)
            .map_err(|e| anyhow::anyhow!("Failed to decode base64 image data: {}", e))?;

        // Upload to Supabase Storage
        let upload_url = format!(
            "{}/storage/v1/object/{}/{}",
            self.supabase_url, NOTEBOOK_IMAGES_BUCKET, storage_path
        );

        debug!("Upload URL: {}", upload_url);

        let response = self
            .client
            .post(&upload_url)
            .header("Authorization", format!("Bearer {}", self.service_role_key))
            .header("Content-Type", &req.content_type)
            .header("x-upsert", "true") // Overwrite if exists
            .body(file_bytes)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to upload image to Supabase: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!(
                "Supabase Storage upload failed: status={}, error={}",
                status, error_text
            );
            return Err(anyhow::anyhow!(
                "Failed to upload image to Supabase Storage: {} - {}",
                status,
                error_text
            ));
        }

        info!("Image uploaded successfully to: {}", storage_path);

        // Build the public URL for the image
        let public_url = self.get_public_url(&storage_path);

        Ok(UploadedImage {
            id: image_id,
            note_id: req.note_id,
            src: public_url,
            alt_text: req.alt_text,
            caption: req.caption,
            width: req.width,
            height: req.height,
            position: req.position.unwrap_or(0),
            storage_path,
        })
    }

    /// Get the public URL for an image
    pub fn get_public_url(&self, storage_path: &str) -> String {
        format!(
            "{}/storage/v1/object/public/{}/{}",
            self.supabase_url, NOTEBOOK_IMAGES_BUCKET, storage_path
        )
    }

    /// Get a signed URL for private access (if bucket is private)
    pub async fn get_signed_url(&self, storage_path: &str, expires_in: u64) -> Result<String> {
        let url = format!(
            "{}/storage/v1/object/sign/{}/{}",
            self.supabase_url, NOTEBOOK_IMAGES_BUCKET, storage_path
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.service_role_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({ "expiresIn": expires_in }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Failed to get signed URL: {}", error_text));
        }

        #[derive(Deserialize)]
        struct SignedUrlResponse {
            #[serde(rename = "signedURL")]
            signed_url: String,
        }

        let signed_response: SignedUrlResponse = response.json().await?;
        Ok(format!("{}{}", self.supabase_url, signed_response.signed_url))
    }

    /// Delete an image from Supabase Storage
    pub async fn delete_image(&self, storage_path: &str) -> Result<bool> {
        info!("Deleting image from Supabase Storage: {}", storage_path);

        let url = format!(
            "{}/storage/v1/object/{}/{}",
            self.supabase_url, NOTEBOOK_IMAGES_BUCKET, storage_path
        );

        let response = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", self.service_role_key))
            .send()
            .await?;

        if response.status().is_success() || response.status().as_u16() == 404 {
            info!("Image deleted successfully: {}", storage_path);
            Ok(true)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            error!("Failed to delete image: {}", error_text);
            Err(anyhow::anyhow!("Failed to delete image: {}", error_text))
        }
    }

    /// Delete all images for a note
    pub async fn delete_note_images(&self, user_id: &str, note_id: &str) -> Result<u64> {
        let folder_path = format!("{}/{}", user_id, note_id);
        info!("Deleting all images in folder: {}", folder_path);

        // List all files in the folder
        let list_url = format!(
            "{}/storage/v1/object/list/{}",
            self.supabase_url, NOTEBOOK_IMAGES_BUCKET
        );

        let response = self
            .client
            .post(&list_url)
            .header("Authorization", format!("Bearer {}", self.service_role_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "prefix": folder_path,
                "limit": 1000
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Failed to list images: {}", error_text));
        }

        #[derive(Deserialize)]
        struct FileObject {
            name: String,
        }

        let files: Vec<FileObject> = response.json().await.unwrap_or_default();
        let mut deleted_count = 0u64;

        for file in files {
            let file_path = format!("{}/{}", folder_path, file.name);
            if self.delete_image(&file_path).await.is_ok() {
                deleted_count += 1;
            }
        }

        info!("Deleted {} images from folder: {}", deleted_count, folder_path);
        Ok(deleted_count)
    }

    /// Delete all images for a user
    pub async fn delete_user_images(&self, user_id: &str) -> Result<u64> {
        info!("Deleting all images for user: {}", user_id);

        let list_url = format!(
            "{}/storage/v1/object/list/{}",
            self.supabase_url, NOTEBOOK_IMAGES_BUCKET
        );

        let response = self
            .client
            .post(&list_url)
            .header("Authorization", format!("Bearer {}", self.service_role_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "prefix": user_id,
                "limit": 10000
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Failed to list user images: {}", error_text));
        }

        #[derive(Deserialize)]
        struct FileObject {
            name: String,
        }

        let files: Vec<FileObject> = response.json().await.unwrap_or_default();
        let mut deleted_count = 0u64;

        for file in files {
            let file_path = format!("{}/{}", user_id, file.name);
            if self.delete_image(&file_path).await.is_ok() {
                deleted_count += 1;
            }
        }

        info!("Deleted {} images for user: {}", deleted_count, user_id);
        Ok(deleted_count)
    }
}

/// Service for managing notebook images (combines storage + database)
pub struct NotebookImageService {
    storage: NotebookImageStorage,
}

impl NotebookImageService {
    pub fn new(config: &SupabaseConfig) -> Self {
        Self {
            storage: NotebookImageStorage::new(config),
        }
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self {
            storage: NotebookImageStorage::from_env()?,
        })
    }

    /// Upload image and save to database
    pub async fn upload_and_save(
        &self,
        conn: &libsql::Connection,
        user_id: &str,
        req: UploadImageRequest,
    ) -> Result<NotebookImage> {
        // Upload to Supabase Storage
        let uploaded = self.storage.upload_image(user_id, req).await?;

        // Save to database
        let create_req = CreateImageRequest {
            note_id: uploaded.note_id,
            src: uploaded.src,
            storage_path: Some(uploaded.storage_path),
            alt_text: uploaded.alt_text,
            caption: uploaded.caption,
            width: uploaded.width,
            height: uploaded.height,
            position: Some(uploaded.position),
        };

        NotebookImage::create(conn, create_req).await
    }

    /// Get all images for a note (fetches from database, URLs point to Supabase)
    pub async fn get_note_images(
        &self,
        conn: &libsql::Connection,
        note_id: &str,
    ) -> Result<Vec<NotebookImage>> {
        NotebookImage::find_by_note(conn, note_id).await
    }

    /// Delete image from storage and database
    pub async fn delete_image(
        &self,
        conn: &libsql::Connection,
        _user_id: &str,
        image_id: &str,
    ) -> Result<bool> {
        // Get image from database first
        let image = NotebookImage::find_by_id(conn, image_id).await?;

        // Extract storage path from src URL
        let storage_path = self.extract_storage_path(&image.src);

        // Delete from storage
        if let Some(path) = storage_path {
            self.storage.delete_image(&path).await.ok();
        }

        // Delete from database
        NotebookImage::delete(conn, image_id).await
    }

    /// Delete all images for a note (storage + database)
    pub async fn delete_note_images(
        &self,
        conn: &libsql::Connection,
        user_id: &str,
        note_id: &str,
    ) -> Result<u64> {
        // Delete from storage
        self.storage.delete_note_images(user_id, note_id).await.ok();

        // Delete from database
        NotebookImage::delete_by_note(conn, note_id).await
    }

    /// Extract storage path from public URL
    fn extract_storage_path(&self, src: &str) -> Option<String> {
        // URL format: {supabase_url}/storage/v1/object/public/{bucket}/{path}
        let marker = format!("/storage/v1/object/public/{}/", NOTEBOOK_IMAGES_BUCKET);
        if let Some(idx) = src.find(&marker) {
            Some(src[idx + marker.len()..].to_string())
        } else {
            None
        }
    }

    /// Get storage service for direct access
    pub fn storage(&self) -> &NotebookImageStorage {
        &self.storage
    }
}
