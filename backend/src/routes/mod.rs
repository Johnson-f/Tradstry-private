mod graphql;

use actix_web::web;

pub use graphql::{graphiql, graphql_handler};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/graphql")
            .route(web::post().to(graphql_handler))
            .route(web::get().to(graphiql)),
    );
}
