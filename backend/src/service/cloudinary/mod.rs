use anyhow::{Result, anyhow};
use cloudinary::upload::{OptionalParameters, Source, Upload, UploadResult};
use std::collections::{BTreeSet, HashSet};

#[derive(Clone)]
pub struct CloudinaryConfig {
    pub cloud_name: String,
    pub api_key: String,
    pub api_secret: String,
}

impl CloudinaryConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            cloud_name: std::env::var("CLOUDINARY_CLOUD_NAME")?,
            api_key: std::env::var("CLOUDINARY_API_KEY")?,
            api_secret: std::env::var("CLOUDINARY_API_SECRET")?,
        })
    }
}

pub struct CloudinaryClient {
    cloud_name: String,
    api_key: String,
    api_secret: String,
}

#[derive(Debug, Clone)]
pub struct UploadedNotebookImage {
    pub asset_id: String,
    pub public_id: String,
    pub secure_url: String,
    pub width: i64,
    pub height: i64,
    pub format: String,
    pub bytes: i64,
    pub original_filename: String,
}

impl CloudinaryClient {
    pub fn new(config: CloudinaryConfig) -> Self {
        Self {
            cloud_name: config.cloud_name,
            api_key: config.api_key,
            api_secret: config.api_secret,
        }
    }

    pub async fn upload_notebook_image(
        &self,
        data_url: String,
        public_id: String,
        display_name: String,
        tags: Vec<String>,
    ) -> Result<UploadedNotebookImage> {
        let upload = Upload::new(
            self.api_key.clone(),
            self.cloud_name.clone(),
            self.api_secret.clone(),
        );
        let mut options = BTreeSet::from([
            OptionalParameters::PublicId(public_id),
            OptionalParameters::DisplayName(display_name),
            OptionalParameters::Overwrite(false),
        ]);

        if !tags.is_empty() {
            options.insert(OptionalParameters::Tags(
                tags.into_iter().collect::<HashSet<_>>(),
            ));
        }

        let response = upload.image(Source::DataUrl(data_url), &options).await?;

        match response {
            UploadResult::Response(payload) => Ok(UploadedNotebookImage {
                asset_id: payload.asset_id,
                public_id: payload.public_id,
                secure_url: payload.secure_url,
                width: payload.width as i64,
                height: payload.height as i64,
                format: payload.format,
                bytes: payload.bytes as i64,
                original_filename: payload.original_filename.unwrap_or_default(),
            }),
            UploadResult::ResponseWithImageMetadata(payload) => Ok(UploadedNotebookImage {
                asset_id: payload.asset_id,
                public_id: payload.public_id,
                secure_url: payload.secure_url,
                width: payload.width as i64,
                height: payload.height as i64,
                format: payload.format,
                bytes: payload.bytes as i64,
                original_filename: payload.display_name,
            }),
            UploadResult::Error(payload) => Err(anyhow!(payload.error.message)),
        }
    }
}
