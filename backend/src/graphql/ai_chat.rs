use async_graphql::{Context, InputObject, Object, Result, SimpleObject};
use clerk_rs::validators::authorizer::ClerkJwt;
use std::sync::Arc;

use crate::service::ai::ai_chat;
use crate::service::read_service::users::ensure_user;
use crate::service::turso::TursoClient;

async fn get_user_id_from_ctx(ctx: &Context<'_>) -> Result<String> {
    let jwt = ctx.data::<ClerkJwt>()?;
    let turso = ctx.data::<Arc<TursoClient>>()?;
    let conn = turso.get_connection()?;

    let full_name = jwt
        .other
        .get("full_name")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let email = jwt
        .other
        .get("email")
        .and_then(|value| value.as_str())
        .unwrap_or("");

    let user = ensure_user(&conn, &jwt.sub, full_name, email).await?;
    Ok(user.id)
}

#[derive(InputObject)]
#[graphql(rename_fields = "camelCase")]
pub struct AiChatInput {
    pub message: String,
    pub session_id: Option<String>,
}

#[derive(SimpleObject)]
#[graphql(rename_fields = "camelCase")]
pub struct AiChatResponse {
    pub request_id: String,
    pub session_id: String,
    pub text: String,
    pub promoted_memory_uris: Vec<String>,
}

#[derive(Default)]
pub struct AiChatMutation;

#[Object]
impl AiChatMutation {
    async fn ai_chat(&self, ctx: &Context<'_>, input: AiChatInput) -> Result<AiChatResponse> {
        let user_id = get_user_id_from_ctx(ctx).await?;
        let agents_client = ctx
            .data::<Option<crate::service::agents::AgentsClient>>()?
            .clone();

        let response = ai_chat::send_chat_message(
            &agents_client,
            &user_id,
            input.session_id,
            input.message,
        )
        .await?;

        Ok(AiChatResponse {
            request_id: response.request_id,
            session_id: response.session_id,
            text: response.text,
            promoted_memory_uris: response.promoted_memory_uris,
        })
    }
}
