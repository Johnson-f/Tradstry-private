use async_graphql::{Context, Object, Result};
use clerk_rs::validators::authorizer::ClerkJwt;
use std::sync::Arc;

use crate::service::read_service::users::ensure_user;
use crate::service::turso::TursoClient;
use crate::service::turso::schema::tables::users_table::User;

#[derive(Default)]
pub struct UserQuery;

#[Object]
impl UserQuery {
    /// Returns the current authenticated user.
    /// Creates the user record and a default account on first access.
    async fn me(&self, ctx: &Context<'_>) -> Result<User> {
        let jwt = ctx.data::<ClerkJwt>()?;
        let turso = ctx.data::<Arc<TursoClient>>()?;
        let conn = turso.get_connection()?;

        let full_name = jwt
            .other
            .get("full_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let email = jwt
            .other
            .get("email")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let user = ensure_user(&conn, &jwt.sub, full_name, email).await?;
        Ok(user)
    }
}
