# Image Upload Service

This service provides image upload functionality for trade notes using Uploadcare as the storage backend.

## Features

- **Image Upload**: Upload images to Uploadcare with automatic validation
- **Image Management**: Store image metadata in the user's database
- **Trade Note Association**: Link images to specific trade notes
- **Image Transformations**: Generate CDN URLs with optional transformations
- **Soft Delete**: Mark images as deleted without removing from storage
- **File Validation**: Validate file type, size, and format

## Configuration

Add the following environment variables to your `.env` file:

```env
# Uploadcare Configuration
UPLOADCARE_PUBLIC_KEY=your_public_key_here
UPLOADCARE_SECRET_KEY=your_secret_key_here
```

## API Endpoints

### Upload Image
```
POST /api/images/upload
Content-Type: multipart/form-data

Form fields:
- file: The image file to upload
- trade_note_id: ID of the trade note to associate with
- alt_text: (optional) Alt text for accessibility
- caption: (optional) Image caption
- position_in_note: (optional) Position order in the note
```

### Get Images by Trade Note
```
GET /api/images/trade-note/{trade_note_id}
```

### Get All Images
```
GET /api/images?trade_note_id={id}&mime_type={type}&limit={n}&offset={n}
```

### Get Image by ID
```
GET /api/images/{image_id}
```

### Update Image
```
PUT /api/images/{image_id}
Content-Type: application/json

{
  "alt_text": "Updated alt text",
  "caption": "Updated caption",
  "position_in_note": 2,
  "width": 800,
  "height": 600
}
```

### Delete Image
```
DELETE /api/images/{image_id}
```

## Usage Example

```rust
use crate::service::image_upload::{ImageUploadService, UploadcareConfig};

// Initialize the service
let config = UploadcareConfig::from_env()?;
let upload_service = ImageUploadService::new(config)?;

// Upload an image
let file_data = std::fs::read("image.jpg")?;
let file_info = upload_service.upload_image(&file_data, "image.jpg", "image/jpeg").await?;

// Generate CDN URL with transformations
let thumbnail_url = upload_service.get_image_url(&file_info.uuid, Some(300), Some(300), Some(80));
```

## Supported Image Formats

- JPEG (.jpg, .jpeg)
- PNG (.png)
- GIF (.gif)
- WebP (.webp)
- BMP (.bmp)
- TIFF (.tiff)

## File Size Limits

- Maximum file size: 10MB
- Recommended: Under 5MB for optimal performance

## Image Transformations

The service supports various image transformations via Uploadcare's CDN:

- **Resize**: `resize/800/` or `resize/x600/`
- **Quality**: `quality/80/`
- **Format**: `format/webp/`
- **Crop**: `crop/400x300/`

Example:
```rust
let transformed_url = upload_service.get_cdn_url(&file_id, Some("resize/800/-/quality/80/"));
```
