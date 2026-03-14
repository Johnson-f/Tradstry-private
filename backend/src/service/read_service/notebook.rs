use anyhow::Result;

use crate::service::turso::client::UserDb;
use crate::service::turso::schema::tables::notebook_table::{
    self, CreateNotebookNoteInput, NotebookNote, UpdateNotebookNoteInput,
};

pub async fn list_notebook_notes(
    user_db: &UserDb,
    account_id: Option<&str>,
) -> Result<Vec<NotebookNote>> {
    notebook_table::list_notebook_notes(user_db.conn(), user_db.user_id(), account_id).await
}

pub async fn get_notebook_note(user_db: &UserDb, id: &str) -> Result<Option<NotebookNote>> {
    notebook_table::find_notebook_note(user_db.conn(), id, user_db.user_id()).await
}

pub async fn create_notebook_note(
    user_db: &UserDb,
    input: CreateNotebookNoteInput,
) -> Result<NotebookNote> {
    notebook_table::create_notebook_note(user_db.conn(), user_db.user_id(), input).await
}

pub async fn update_notebook_note(
    user_db: &UserDb,
    id: &str,
    input: UpdateNotebookNoteInput,
) -> Result<NotebookNote> {
    notebook_table::update_notebook_note(user_db.conn(), id, user_db.user_id(), input).await
}

pub async fn delete_notebook_note(user_db: &UserDb, id: &str) -> Result<bool> {
    notebook_table::delete_notebook_note(user_db.conn(), id, user_db.user_id()).await
}
