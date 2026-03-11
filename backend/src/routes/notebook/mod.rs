pub mod ai;
pub mod calendar;
pub mod collaboration;
pub mod folders;
pub mod images;
pub mod notes;
pub mod tags;

pub use ai::configure_notebook_ai_routes;
pub use calendar::configure_calendar_routes;
pub use collaboration::configure_collaboration_routes;
pub use folders::configure_notebook_folders_routes;
pub use images::configure_notebook_images_routes;
pub use notes::configure_notebook_notes_routes;
pub use tags::configure_notebook_tags_routes;
