mod graphql;
mod routes;
mod service;

use actix_web::{web, App, HttpServer};
use clerk_rs::validators::actix::ClerkMiddleware;
use log::info;
use service::auth::create_jwks_provider;
use service::turso::TursoConfig;
use service::turso::TursoClient;
use std::sync::Arc;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    env_logger::init();

    info!("Starting backend...");

    let config = TursoConfig::from_env()?;
    let turso_client = Arc::new(TursoClient::new(config).await?);

    turso_client.health_check().await?;
    info!("Database healthy and migrations applied");

    let clerk_secret = std::env::var("CLERK_SECRET_KEY")?;
    info!("Clerk authentication configured");

    let schema = graphql::build_schema();

    info!("Starting server on 0.0.0.0:8080");
    HttpServer::new(move || {
        let jwks_provider = create_jwks_provider(&clerk_secret);

        App::new()
            .wrap(ClerkMiddleware::new(jwks_provider, None, true))
            .app_data(web::Data::new(schema.clone()))
            .configure(routes::configure)
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await?;

    Ok(())
}
