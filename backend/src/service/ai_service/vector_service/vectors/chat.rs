#![allow(dead_code)]

use crate::service::ai_service::vector_service::client::VoyagerClient;
use crate::service::ai_service::vector_service::qdrant::{QdrantDocumentClient, SearchResult};
use anyhow::{Context, Result};
use std::sync::Arc;
use uuid::Uuid;

/// Chat-specific vectorization functions
pub struct ChatVectorization {
    voyager_client: Arc<VoyagerClient>,
    qdrant_client: Arc<QdrantDocumentClient>,
}

impl ChatVectorization {
    pub fn new(
        voyager_client: Arc<VoyagerClient>,
        qdrant_client: Arc<QdrantDocumentClient>,
    ) -> Self {
        Self {
            voyager_client,
            qdrant_client,
        }
    }

    /// Vectorize a Q&A pair and store in Qdrant
    /// Formats content as "Q: {question}\nA: {answer}"
    pub async fn vectorize_qa_pair(
        &self,
        user_id: &str,
        session_id: &str,
        question: &str,
        answer: &str,
    ) -> Result<()> {
        log::info!(
            "Vectorizing Q&A pair - user={}, session={}, question_length={}, answer_length={}",
            user_id,
            session_id,
            question.len(),
            answer.len()
        );

        // Format as Q&A pair
        let content = format!("Q: {}\nA: {}", question, answer);

        log::debug!(
            "Formatted Q&A content - user={}, session={}, content_length={}",
            user_id,
            session_id,
            content.len()
        );

        // Generate embedding
        let embedding = self
            .voyager_client
            .embed_text(&content)
            .await
            .context("Failed to generate embedding for Q&A pair")?;

        log::info!(
            "Embedding generated - user={}, session={}, embedding_dim={}",
            user_id,
            session_id,
            embedding.len()
        );

        // Create meaningful ID for payload: chat-{session_id}-qa
        let meaningful_id = format!("chat-{}-qa", session_id);

        // Generate a proper UUID for Qdrant's point ID
        let qdrant_uuid = Uuid::new_v4().to_string();

        log::debug!(
            "Generated IDs - user={}, session={}, meaningful_id={}, qdrant_uuid={}",
            user_id,
            session_id,
            meaningful_id,
            qdrant_uuid
        );

        // Store in Qdrant
        match self
            .qdrant_client
            .upsert_chat_vector(user_id, &qdrant_uuid, &meaningful_id, &content, &embedding)
            .await
        {
            Ok(_) => {
                log::info!(
                    "Successfully stored chat vector in Qdrant - user={}, session={}, meaningful_id={}, qdrant_uuid={}",
                    user_id,
                    session_id,
                    meaningful_id,
                    qdrant_uuid
                );
            }
            Err(e) => {
                log::error!(
                    "Failed to store chat vector in Qdrant - user={}, session={}, meaningful_id={}, qdrant_uuid={}, content_length={}, embedding_dim={}, error={}, error_debug={:?}",
                    user_id,
                    session_id,
                    meaningful_id,
                    qdrant_uuid,
                    content.len(),
                    embedding.len(),
                    e,
                    e
                );
                return Err(e).context("Failed to store chat vector in Qdrant");
            }
        }

        log::info!(
            "Successfully vectorized Q&A pair - user={}, session={}, meaningful_id={}",
            user_id,
            session_id,
            meaningful_id
        );

        Ok(())
    }

    /// Search chat history using semantic similarity
    /// Returns only chat-type vectors
    pub async fn search_chat_history(
        &self,
        user_id: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        log::info!(
            "Searching chat history - user={}, query_preview='{}', limit={}",
            user_id,
            query.chars().take(50).collect::<String>(),
            limit
        );

        // Generate query embedding
        let query_embedding = self
            .voyager_client
            .embed_text(query)
            .await
            .context("Failed to generate query embedding")?;

        log::debug!(
            "Query embedding generated - user={}, embedding_dim={}",
            user_id,
            query_embedding.len()
        );

        // Search Qdrant filtered to chat type only
        let results = self
            .qdrant_client
            .search_by_embedding(user_id, &query_embedding, limit, Some("chat"))
            .await
            .context("Failed to search chat history in Qdrant")?;

        log::info!(
            "Chat history search completed - user={}, results={}",
            user_id,
            results.len()
        );

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qa_content_format() {
        let question = "Why do I keep losing money?";
        let answer = "Based on your trades, you're entering too early.";
        let content = format!("Q: {}\nA: {}", question, answer);

        assert!(content.starts_with("Q: "));
        assert!(content.contains("A: "));
        assert!(content.contains(question));
        assert!(content.contains(answer));
    }

    #[test]
    fn test_vector_id_format() {
        let session_id = "session-123";
        let meaningful_id = format!("chat-{}-qa", session_id);
        assert_eq!(meaningful_id, "chat-session-123-qa");

        // Verify UUID generation works
        let qdrant_uuid = Uuid::new_v4().to_string();
        assert_eq!(qdrant_uuid.len(), 36); // Standard UUID length
    }
}
