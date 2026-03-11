use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Event types for WebSocket messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    // Stock events
    StockCreated,
    StockUpdated,
    StockDeleted,

    // Option events
    OptionCreated,
    OptionUpdated,
    OptionDeleted,

    // Note events
    NoteCreated,
    NoteUpdated,
    NoteDeleted,

    // Playbook events
    PlaybookCreated,
    PlaybookUpdated,
    PlaybookDeleted,

    // Trade-Playbook association events
    TradeTagged,
    TradeUntagged,

    // AI Chat events
    ChatMessageCreated,
    ChatMessageUpdated,
    ChatSessionCreated,
    ChatSessionUpdated,
    ChatSessionDeleted,

    // System events
    Connected,
    Disconnected,
    Error,

    // Market data events
    MarketQuote,
    MarketUpdate,

    // AI analysis events
    AiAnalysisProgress,
    AiAnalysisSection,
    AiAnalysisComplete,

    // AI report events
    AiReportProgress,
    AiReportChunk,
    AiReportComplete,

    // Notebook AI events
    NotebookAiProgress,
    NotebookAiChunk,
    NotebookAiComplete,

    // Collaboration events
    CollaboratorJoined,
    CollaboratorLeft,
    CollaboratorCursorMove,
    CollaboratorSelection,
    NoteContentUpdate,
    NoteInvitationReceived,
    NoteInvitationAccepted,
    NoteInvitationDeclined,
    NoteVisibilityChanged,
}

/// WebSocket message envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsMessage {
    pub event: EventType,
    pub data: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

impl WsMessage {
    pub fn new(event: EventType, data: serde_json::Value) -> Self {
        Self {
            event,
            data,
            timestamp: Utc::now(),
        }
    }
}

/// WebSocket client authentication
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub user_id: String,
    pub client_id: String,
}

/// WebSocket error message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMessage {
    pub error: String,
    pub code: String,
}

impl ErrorMessage {
    #[allow(dead_code)]
    pub fn authentication_error() -> Self {
        Self {
            error: "Authentication failed".to_string(),
            code: "AUTH_ERROR".to_string(),
        }
    }

    #[allow(dead_code)]
    pub fn authorization_error() -> Self {
        Self {
            error: "Authorization failed".to_string(),
            code: "AUTHZ_ERROR".to_string(),
        }
    }

    #[allow(dead_code)]
    pub fn generic_error(message: String) -> Self {
        Self {
            error: message,
            code: "GENERIC_ERROR".to_string(),
        }
    }
}
