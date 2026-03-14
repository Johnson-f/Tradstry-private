use anyhow::Result;

use crate::service::turso::client::UserDb;
use crate::service::turso::schema::tables::notebook_images::{
    self, CreateNotebookImageInput, NotebookImage,
};

pub async fn create_notebook_image(
    user_db: &UserDb,
    input: CreateNotebookImageInput,
) -> Result<NotebookImage> {
    notebook_images::create_notebook_image(user_db.conn(), user_db.user_id(), input).await
}

pub async fn get_notebook_image(user_db: &UserDb, id: &str) -> Result<Option<NotebookImage>> {
    notebook_images::find_notebook_image(user_db.conn(), id, user_db.user_id()).await
}
