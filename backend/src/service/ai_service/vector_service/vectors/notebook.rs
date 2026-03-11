#![allow(dead_code)]

use crate::models::notebook::NotebookNote;
use crate::service::ai_service::vector_service::client::VoyagerClient;
use crate::service::ai_service::vector_service::qdrant::QdrantDocumentClient;
use anyhow::{Context, Result};
use std::sync::Arc;

/// Notebook-specific vectorization functions
pub struct NotebookVectorization {
    voyager_client: Arc<VoyagerClient>,
    qdrant_client: Arc<QdrantDocumentClient>,
}

impl NotebookVectorization {
    pub fn new(
        voyager_client: Arc<VoyagerClient>,
        qdrant_client: Arc<QdrantDocumentClient>,
    ) -> Self {
        Self {
            voyager_client,
            qdrant_client,
        }
    }

    /// Vectorize a notebook note
    /// Formats content and stores in Qdrant for AI chat context
    pub async fn vectorize_notebook(&self, user_id: &str, note: &NotebookNote) -> Result<()> {
        log::info!(
            "Vectorizing notebook - user={}, note_id={}, title={}",
            user_id,
            note.id,
            note.title
        );

        // Format content for vectorization
        let content = format_notebook_content(note);

        log::debug!(
            "Formatted notebook content - user={}, note_id={}, content_length={}",
            user_id,
            note.id,
            content.len()
        );

        // Generate embedding
        let embedding = self
            .voyager_client
            .embed_text(&content)
            .await
            .context("Failed to generate embedding for notebook")?;

        log::info!(
            "Embedding generated - user={}, note_id={}, embedding_dim={}",
            user_id,
            note.id,
            embedding.len()
        );

        // Create vector ID: notebook-{note_id}
        let vector_id = format!("notebook-{}", note.id);

        // Store in Qdrant
        self.qdrant_client
            .upsert_notebook_vector(user_id, &vector_id, &content, &embedding)
            .await
            .context("Failed to store notebook vector in Qdrant")?;

        log::info!(
            "Successfully vectorized notebook - user={}, note_id={}, vector_id={}",
            user_id,
            note.id,
            vector_id
        );

        Ok(())
    }

    /// Delete a notebook vector from Qdrant
    pub async fn delete_notebook_vector(&self, user_id: &str, note_id: &str) -> Result<()> {
        let vector_id = format!("notebook-{}", note_id);

        self.qdrant_client
            .delete_notebook_vector(user_id, &vector_id)
            .await
            .context("Failed to delete notebook vector from Qdrant")?;

        log::info!(
            "Successfully deleted notebook vector - user={}, note_id={}, vector_id={}",
            user_id,
            note_id,
            vector_id
        );

        Ok(())
    }
}

/// Format notebook content for vectorization
/// Combines note title and content
fn format_notebook_content(note: &NotebookNote) -> String {
    let mut parts = Vec::new();

    // Note title
    parts.push(format!("Title: {}", note.title));

    // Note content (convert JSON Value to string)
    let content_str = match &note.content {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => {
            // Try to extract text from BlockNote format
            arr.iter()
                .filter_map(|v| {
                    if let serde_json::Value::Object(obj) = v
                        && let Some(serde_json::Value::String(text)) = obj.get("content")
                    {
                        return Some(text.clone());
                    }
                    if let serde_json::Value::Object(obj) = v
                        && let Some(serde_json::Value::Array(content_arr)) = obj.get("content")
                    {
                        let extracted: Vec<String> = content_arr
                            .iter()
                            .filter_map(|c| {
                                if let serde_json::Value::Object(c_obj) = c
                                    && let Some(serde_json::Value::String(text)) = c_obj.get("text")
                                {
                                    return Some(text.clone());
                                }
                                None
                            })
                            .collect();
                        return Some(extracted.join(" "));
                    }
                    None
                })
                .collect::<Vec<_>>()
                .join(" ")
        }
        _ => serde_json::to_string(&note.content).unwrap_or_default(),
    };

    if !content_str.trim().is_empty() {
        parts.push("Content:".to_string());
        parts.push(content_str);
    }

    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notebook_content_format() {
        let note = NotebookNote {
            id: "note-123".to_string(),
            parent_id: None,
            title: "Test Note".to_string(),
            content: serde_json::Value::String("This is a test notebook note.".to_string()),
            position: 0,
            is_deleted: false,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let content = format_notebook_content(&note);

        assert!(content.contains("Title: Test Note"));
        assert!(content.contains("This is a test notebook note."));
    }

    #[test]
    fn test_vector_id_format() {
        let note_id = "note-123";
        let vector_id = format!("notebook-{}", note_id);
        assert_eq!(vector_id, "notebook-note-123");
    }
}
