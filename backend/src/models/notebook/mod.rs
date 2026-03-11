pub mod calendar_connections;
pub mod calendar_events;
pub mod calendar_sync_cursors;
pub mod note_collaborators;
pub mod note_invitations;
pub mod note_visibility;
pub mod notebook_folders;
pub mod notebook_images;
pub mod notebook_notes;
pub mod notebook_tags;

pub use calendar_connections::{
    CalendarConnection, CalendarProvider, CreateConnectionRequest, UpdateConnectionRequest,
};
pub use calendar_events::{
    CalendarEvent, CreateEventRequest, CreateSyncedEventRequest, EventQuery, EventSource,
    EventStatus, UpdateEventRequest,
};
pub use calendar_sync_cursors::CalendarSyncCursor;
pub use note_collaborators::{
    CollaboratorRole, CreateCollaboratorRequest, NoteCollaborator, UpdateCollaboratorRequest,
};
pub use note_invitations::{CreateInvitationRequest, NoteInvitation};
pub use note_visibility::{
    CreateShareSettingsRequest, NoteShareSettings, NoteVisibility, UpdateShareSettingsRequest,
};
pub use notebook_images::{CreateImageRequest, NotebookImage};
pub use notebook_notes::NotebookNote;
// vec![] is a macro in Rust that creates a new Vec (vector/dynamic array)
// Examples:
// let empty_vec: Vec<i32> = vec![]; // Creates an empty vector
// let numbers = vec![1, 2, 3, 4]; // Creates a vector with initial values
// let repeated = vec![0; 5]; // Creates a vector with 5 zeros
