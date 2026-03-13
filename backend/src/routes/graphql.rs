use actix_web::{web, HttpMessage, HttpRequest, HttpResponse, Result};
use async_graphql::http::GraphiQLSource;
use async_graphql_actix_web::{GraphQLRequest, GraphQLResponse};
use clerk_rs::validators::authorizer::ClerkJwt;
use log::{error, info};
use std::sync::Arc;

use crate::graphql::AppSchema;
use crate::service::turso::TursoClient;

fn infer_operation_name(query: &str) -> &str {
    query
        .split_whitespace()
        .collect::<Vec<_>>()
        .windows(2)
        .find_map(|parts| match parts {
            ["query" | "mutation" | "subscription", name] => Some(*name),
            _ => None,
        })
        .unwrap_or("anonymous")
}

pub async fn graphql_handler(
    schema: web::Data<AppSchema>,
    http_req: HttpRequest,
    turso: web::Data<Arc<TursoClient>>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let mut request = req.into_inner();
    let operation_name = request
        .operation_name
        .clone()
        .unwrap_or_else(|| infer_operation_name(&request.query).to_string());
    let query_preview = request.query.lines().next().unwrap_or("").trim().to_string();
    let auth = http_req.extensions().get::<ClerkJwt>().cloned();
    let auth_subject = auth
        .as_ref()
        .map(|jwt| jwt.sub.clone())
        .unwrap_or_else(|| "anonymous".to_string());

    info!(
        "GraphQL request: operation={} method={} path={} auth_subject={} query_preview={:?}",
        operation_name,
        http_req.method(),
        http_req.path(),
        auth_subject,
        query_preview
    );

    if let Some(jwt) = auth {
        request = request.data(jwt);
    }
    request = request.data(turso.get_ref().clone());

    let response = schema.execute(request).await;

    if response.errors.is_empty() {
        info!(
            "GraphQL success: operation={} auth_subject={} error_count=0",
            operation_name, auth_subject
        );
    } else {
        error!(
            "GraphQL failure: operation={} auth_subject={} errors={:?}",
            operation_name, auth_subject, response.errors
        );
    }

    response.into()
}

pub async fn graphiql() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(GraphiQLSource::build().endpoint("/graphql").finish()))
}
