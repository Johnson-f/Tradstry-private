use actix_web::{HttpMessage, HttpResponse, Scope, web};
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::models::alerts::{CreateAlertRuleRequest, UpdateAlertRuleRequest};
use crate::service::notifications::{
    alert_worker::{PricePoint, evaluate_price_rules_for_user},
    alerts::AlertRuleService,
    push::{PushPayload, PushService, SaveSubscriptionRequest},
};
use crate::turso::AppState;
use crate::turso::config::WebPushConfig;

fn get_user_id_from_ext(req: &actix_web::HttpRequest) -> Option<String> {
    req.extensions()
        .get::<crate::turso::config::SupabaseClaims>()
        .map(|c| c.sub.clone())
}

pub fn configure_alert_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(api_scope());
}

fn api_scope() -> Scope {
    web::scope("/api/alerts")
        .route("/subscribe", web::post().to(subscribe))
        .route("/unsubscribe", web::post().to(unsubscribe))
        .route("/test", web::post().to(send_test))
        .route("/evaluate/price", web::post().to(evaluate_price_alerts))
        .route("/rules", web::get().to(list_rules))
        .route("/rules", web::post().to(create_rule))
        .route("/rules/{id}", web::get().to(get_rule))
        .route("/rules/{id}", web::put().to(update_rule))
        .route("/rules/{id}", web::delete().to(delete_rule))
}

#[derive(Deserialize)]
struct PriceEvaluationInput {
    symbol: String,
    price: f64,
    direction: Option<String>,
    percent_change: Option<f64>,
    timestamp: Option<String>,
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
    Ok(HttpResponse::Ok().json(serde_json::json!({ "id": id })))
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
    Ok(HttpResponse::Ok().json(serde_json::json!({ "removed": ok })))
}

async fn send_test(
    app: web::Data<AppState>,
    req: actix_web::HttpRequest,
) -> actix_web::Result<HttpResponse> {
    let user_id = get_user_id_from_ext(&req)
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("Unauthorized"))?;
    let tag = format!("alerts-test-{}", chrono::Utc::now().timestamp_millis());
    let ts = chrono::Utc::now().timestamp();
    let payload = PushPayload {
        title: "Tradstry".to_string(),
        body: Some("Test push notification".to_string()),
        icon: Some("/app/icon.png".to_string()),
        url: Some("/app/alerts".to_string()),
        tag: Some(tag.clone()),
        data: Some(serde_json::json!({ "t": "system", "eid": tag, "ts": ts })),
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
    Ok(HttpResponse::Ok().json(serde_json::json!({ "ok": true })))
}

async fn evaluate_price_alerts(
    app: web::Data<AppState>,
    req: actix_web::HttpRequest,
    body: web::Json<Vec<PriceEvaluationInput>>,
) -> actix_web::Result<HttpResponse> {
    let user_id = get_user_id_from_ext(&req)
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("Unauthorized"))?;
    let conn = app
        .turso_client
        .get_user_database_connection(&user_id)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?
        .ok_or_else(|| actix_web::error::ErrorForbidden("No database"))?;

    let points: Vec<PricePoint> = body
        .iter()
        .map(|p| {
            let ts: DateTime<Utc> = p
                .timestamp
                .as_deref()
                .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);
            PricePoint {
                symbol: p.symbol.clone(),
                price: p.price,
                direction: p.direction.clone(),
                percent_change: p.percent_change,
                timestamp: ts,
            }
        })
        .collect();

    let dispatched = evaluate_price_rules_for_user(&conn, &app.config.web_push, &user_id, &points)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "dispatched": dispatched })))
}

async fn list_rules(
    app: web::Data<AppState>,
    req: actix_web::HttpRequest,
) -> actix_web::Result<HttpResponse> {
    let user_id = get_user_id_from_ext(&req)
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("Unauthorized"))?;
    let conn = app
        .turso_client
        .get_user_database_connection(&user_id)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?
        .ok_or_else(|| actix_web::error::ErrorForbidden("No database"))?;
    let service = AlertRuleService::new(&conn);
    let rules = service
        .list_rules(&user_id)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(HttpResponse::Ok().json(rules))
}

async fn create_rule(
    app: web::Data<AppState>,
    req: actix_web::HttpRequest,
    body: web::Json<CreateAlertRuleRequest>,
) -> actix_web::Result<HttpResponse> {
    let user_id = get_user_id_from_ext(&req)
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("Unauthorized"))?;
    let conn = app
        .turso_client
        .get_user_database_connection(&user_id)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?
        .ok_or_else(|| actix_web::error::ErrorForbidden("No database"))?;
    let service = AlertRuleService::new(&conn);
    let created = service
        .create_rule(&user_id, body.into_inner())
        .await
        .map_err(actix_web::error::ErrorBadRequest)?;
    Ok(HttpResponse::Created().json(created))
}

async fn get_rule(
    app: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
) -> actix_web::Result<HttpResponse> {
    let user_id = get_user_id_from_ext(&req)
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("Unauthorized"))?;
    let conn = app
        .turso_client
        .get_user_database_connection(&user_id)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?
        .ok_or_else(|| actix_web::error::ErrorForbidden("No database"))?;
    let id = path.into_inner();
    let service = AlertRuleService::new(&conn);
    match service
        .get_rule(&user_id, &id)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?
    {
        Some(rule) => Ok(HttpResponse::Ok().json(rule)),
        None => Ok(HttpResponse::NotFound().finish()),
    }
}

async fn update_rule(
    app: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    body: web::Json<UpdateAlertRuleRequest>,
) -> actix_web::Result<HttpResponse> {
    let user_id = get_user_id_from_ext(&req)
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("Unauthorized"))?;
    let conn = app
        .turso_client
        .get_user_database_connection(&user_id)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?
        .ok_or_else(|| actix_web::error::ErrorForbidden("No database"))?;
    let id = path.into_inner();
    let service = AlertRuleService::new(&conn);
    let updated = service
        .update_rule(&user_id, &id, body.into_inner())
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    match updated {
        Some(rule) => Ok(HttpResponse::Ok().json(rule)),
        None => Ok(HttpResponse::NotFound().finish()),
    }
}

async fn delete_rule(
    app: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
) -> actix_web::Result<HttpResponse> {
    let user_id = get_user_id_from_ext(&req)
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("Unauthorized"))?;
    let conn = app
        .turso_client
        .get_user_database_connection(&user_id)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?
        .ok_or_else(|| actix_web::error::ErrorForbidden("No database"))?;
    let id = path.into_inner();
    let service = AlertRuleService::new(&conn);
    let deleted = service
        .delete_rule(&user_id, &id)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    if deleted {
        Ok(HttpResponse::NoContent().finish())
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}
