mod collaboration_service;
mod manager;
mod room;

pub use collaboration_service::{CollaborationService, CursorPosition, CollaboratorPresence};
pub use manager::{RoomManager, RoomManagerConfig};
pub use room::{
    AwarenessState, ClientConnection, CollabMessage, CollaborationRoom, CursorState,
    ParticipantInfo, UserInfo,
};
