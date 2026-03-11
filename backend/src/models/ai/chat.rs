#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Chat message role
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageRole::User => write!(f, "user"),
            MessageRole::Assistant => write!(f, "assistant"),
            MessageRole::System => write!(f, "system"),
        }
    }
}

/// Chat message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub session_id: String, // ADD THIS
    pub role: MessageRole,
    pub content: String,
    #[serde(rename = "created_at")]
    pub timestamp: DateTime<Utc>,
    pub context_vectors: Option<Vec<String>>, // Vector IDs used for context
    pub token_count: Option<u32>,
}

impl ChatMessage {
    pub fn new(session_id: String, role: MessageRole, content: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            session_id, // ADD THIS
            role,
            content,
            timestamp: Utc::now(),
            context_vectors: None,
            token_count: None,
        }
    }

    pub fn with_context(mut self, context_vectors: Vec<String>) -> Self {
        self.context_vectors = Some(context_vectors);
        self
    }

    pub fn with_token_count(mut self, token_count: u32) -> Self {
        self.token_count = Some(token_count);
        self
    }
}

/// Chat session structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: String,
    pub user_id: String,
    pub title: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: u32,
    pub last_message_at: Option<DateTime<Utc>>,
}

impl ChatSession {
    pub fn new(user_id: String, title: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            title,
            created_at: now,
            updated_at: now,
            message_count: 0,
            last_message_at: None,
        }
    }

    pub fn update_last_message(&mut self) {
        self.last_message_at = Some(Utc::now());
        self.updated_at = Utc::now();
        self.message_count += 1;
    }
}

/// Chat request structure
#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub session_id: Option<String>,
    pub include_context: Option<bool>,
    pub max_context_vectors: Option<usize>,
    pub use_tools: Option<bool>,
}

/// Chat response structure
#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub message: String,
    pub session_id: String,
    pub message_id: String,
    pub sources: Vec<ContextSource>,
    pub token_count: Option<u32>,
    pub processing_time_ms: u64,
}

/// Context source for chat responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSource {
    pub vector_id: String,
    pub data_type: String,
    pub entity_id: String,
    pub similarity_score: f32,
    pub snippet: String,
}

impl ContextSource {
    pub fn new(
        vector_id: String,
        data_type: String,
        entity_id: String,
        similarity_score: f32,
        snippet: String,
    ) -> Self {
        Self {
            vector_id,
            data_type,
            entity_id,
            similarity_score,
            snippet,
        }
    }

    /// Create ContextSource from Qdrant SearchResult
    pub fn from_search_result(
        result: &crate::service::ai_service::vector_service::qdrant::SearchResult,
    ) -> Self {
        // Extract entity_id from vector_id
        // Format: "trade-{id}-mistakes-notes" or "chat-{session_id}-qa"
        let entity_id = if result.id.starts_with("trade-") {
            // Extract trade ID: "trade-456-mistakes-notes" -> "456"
            result
                .id
                .strip_prefix("trade-")
                .and_then(|s| s.split('-').next())
                .unwrap_or(&result.id)
                .to_string()
        } else if result.id.starts_with("chat-") {
            // Extract session ID: "chat-session-123-qa" -> "session-123"
            result
                .id
                .strip_prefix("chat-")
                .and_then(|s| s.strip_suffix("-qa"))
                .unwrap_or(&result.id)
                .to_string()
        } else {
            result.id.clone()
        };

        // Determine data_type from result type or id prefix
        let data_type = result.r#type.clone().unwrap_or_else(|| {
            if result.id.starts_with("trade-") {
                "trade".to_string()
            } else if result.id.starts_with("chat-") {
                "chat".to_string()
            } else {
                "unknown".to_string()
            }
        });

        // Truncate content to snippet (max 200 chars)
        let snippet = if result.content.len() > 200 {
            format!("{}...", &result.content[..200])
        } else {
            result.content.clone()
        };

        Self {
            vector_id: result.id.clone(),
            data_type,
            entity_id,
            similarity_score: result.score,
            snippet,
        }
    }
}

/// Chat session list response
#[derive(Debug, Serialize)]
pub struct ChatSessionListResponse {
    pub sessions: Vec<ChatSessionSummary>,
    pub total_count: u32,
}

/// Chat session summary for list view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSessionSummary {
    pub id: String,
    pub title: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: u32,
    pub last_message_preview: Option<String>,
}

impl From<ChatSession> for ChatSessionSummary {
    fn from(session: ChatSession) -> Self {
        Self {
            id: session.id,
            title: session.title,
            created_at: session.created_at,
            updated_at: session.updated_at,
            message_count: session.message_count,
            last_message_preview: None, // Would be populated from last message
        }
    }
}

/// Chat session details response
#[derive(Debug, Serialize)]
pub struct ChatSessionDetailsResponse {
    pub session: ChatSession,
    pub messages: Vec<ChatMessage>,
    pub total_messages: u32,
}

/// Streaming chat response chunk
#[derive(Debug, Serialize)]
pub struct ChatStreamChunk {
    pub chunk_type: StreamChunkType,
    pub content: String,
    pub message_id: Option<String>,
    pub session_id: Option<String>,
    pub is_final: bool,
}

#[derive(Debug, Serialize)]
pub enum StreamChunkType {
    Token,   // Individual token/word
    Context, // Context information
    Sources, // Source information
    Error,   // Error message
}

impl ChatStreamChunk {
    pub fn token(content: String) -> Self {
        Self {
            chunk_type: StreamChunkType::Token,
            content,
            message_id: None,
            session_id: None,
            is_final: false,
        }
    }

    pub fn context(content: String) -> Self {
        Self {
            chunk_type: StreamChunkType::Context,
            content,
            message_id: None,
            session_id: None,
            is_final: false,
        }
    }

    pub fn sources(content: String) -> Self {
        Self {
            chunk_type: StreamChunkType::Sources,
            content,
            message_id: None,
            session_id: None,
            is_final: false,
        }
    }

    pub fn error(content: String) -> Self {
        Self {
            chunk_type: StreamChunkType::Error,
            content,
            message_id: None,
            session_id: None,
            is_final: true,
        }
    }

    pub fn final_chunk(message_id: String, session_id: String) -> Self {
        Self {
            chunk_type: StreamChunkType::Token,
            content: String::new(),
            message_id: Some(message_id),
            session_id: Some(session_id),
            is_final: true,
        }
    }
}

/// Chat statistics
#[derive(Debug, Serialize)]
pub struct ChatStats {
    pub total_sessions: u32,
    pub total_messages: u32,
    pub avg_messages_per_session: f64,
    pub most_active_session: Option<String>,
    pub last_activity: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_creation() {
        let message = ChatMessage::new(
            "session123".to_string(),
            MessageRole::User,
            "Hello".to_string(),
        );
        assert_eq!(message.session_id, "session123");
        assert_eq!(message.role, MessageRole::User);
        assert_eq!(message.content, "Hello");
        assert!(message.context_vectors.is_none());
    }

    #[test]
    fn test_chat_message_with_context() {
        let message = ChatMessage::new(
            "session123".to_string(),
            MessageRole::Assistant,
            "Response".to_string(),
        )
        .with_context(vec!["vec1".to_string(), "vec2".to_string()]);

        assert!(message.context_vectors.is_some());
        assert_eq!(message.context_vectors.unwrap().len(), 2);
    }

    #[test]
    fn test_chat_session_creation() {
        let session = ChatSession::new("user123".to_string(), Some("Test Session".to_string()));
        assert_eq!(session.user_id, "user123");
        assert_eq!(session.title, Some("Test Session".to_string()));
        assert_eq!(session.message_count, 0);
    }

    #[test]
    fn test_chat_session_update() {
        let mut session = ChatSession::new("user123".to_string(), None);
        let initial_count = session.message_count;

        session.update_last_message();

        assert_eq!(session.message_count, initial_count + 1);
        assert!(session.last_message_at.is_some());
    }

    #[test]
    fn test_stream_chunk_creation() {
        let token_chunk = ChatStreamChunk::token("Hello".to_string());
        assert_eq!(token_chunk.content, "Hello");
        assert!(!token_chunk.is_final);

        let error_chunk = ChatStreamChunk::error("Error occurred".to_string());
        assert_eq!(error_chunk.content, "Error occurred");
        assert!(error_chunk.is_final);
    }
}
