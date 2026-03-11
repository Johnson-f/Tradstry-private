use actix_web::{HttpMessage, HttpResponse, Scope, web};
use serde::Deserialize;

use crate::service::notifications::push::{PushPayload, PushService, SaveSubscriptionRequest};
use crate::turso::AppState;
use crate::turso::config::WebPushConfig;

fn get_user_id_from_ext(req: &actix_web::HttpRequest) -> Option<String> {
    // Simplified: in this codebase, claims are inserted in extensions
    req.extensions()
        .get::<crate::turso::config::SupabaseClaims>()
        .map(|c| c.sub.clone())
}

pub fn configure_push_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(api_scope());
}

fn api_scope() -> Scope {
    web::scope("/api/push")
        .route("/subscribe", web::post().to(subscribe))
        .route("/unsubscribe", web::post().to(unsubscribe))
        .route("/test", web::post().to(send_test))
}

async fn subscribe(
    app: web::Data<AppState>,
    req: actix_web::HttpRequest,
    body: web::Json<SaveSubscriptionRequest>,
) -> actix_web::Result<HttpResponse> {
    let user_id = get_user_id_from_ext(&req)
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("Unauthorized"))?;
    let conn = app
        .turso_client
        .get_user_database_connection(&user_id)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?
        .ok_or_else(|| actix_web::error::ErrorForbidden("No database"))?;

    let push_cfg: &WebPushConfig = &app.config.web_push;
    let service = PushService::new(&conn, push_cfg);
    let id = service
        .upsert_subscription(&user_id, body.into_inner())
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(HttpResponse::Ok().json(serde_json::json!({"id": id})))
}

#[derive(Deserialize)]
struct UnsubReq {
    endpoint: String,
}

async fn unsubscribe(
    app: web::Data<AppState>,
    req: actix_web::HttpRequest,
    body: web::Json<UnsubReq>,
) -> actix_web::Result<HttpResponse> {
    let user_id = get_user_id_from_ext(&req)
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("Unauthorized"))?;
    let conn = app
        .turso_client
        .get_user_database_connection(&user_id)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?
        .ok_or_else(|| actix_web::error::ErrorForbidden("No database"))?;
    let push_cfg: &WebPushConfig = &app.config.web_push;
    let service = PushService::new(&conn, push_cfg);
    let ok = service
        .remove_subscription(&user_id, &body.endpoint)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(HttpResponse::Ok().json(serde_json::json!({"removed": ok})))
}

async fn send_test(
    app: web::Data<AppState>,
    req: actix_web::HttpRequest,
) -> actix_web::Result<HttpResponse> {
    let user_id = get_user_id_from_ext(&req)
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("Unauthorized"))?;
    let payload = PushPayload {
        title: "Tradstry".to_string(),
        body: Some("Test push notification".to_string()),
        icon: Some("/app/icon.png".to_string()),
        url: Some("/app".to_string()),
        tag: Some("test".to_string()),
        data: None,
    };
    let conn = app
        .turso_client
        .get_user_database_connection(&user_id)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?
        .ok_or_else(|| actix_web::error::ErrorForbidden("No database"))?;
    let push_cfg: &WebPushConfig = &app.config.web_push;
    let service = PushService::new(&conn, push_cfg);
    service
        .send_to_user(&user_id, &payload)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(HttpResponse::Ok().json(serde_json::json!({"ok": true})))
}
