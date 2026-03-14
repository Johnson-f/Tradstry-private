mod graphql;
mod notebook_images;

use actix_web::web;

pub use graphql::{graphiql, graphql_handler, graphql_ws_handler};
pub use notebook_images::{delete_notebook_image, get_notebook_image, upload_notebook_image};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/graphql")
            .route(web::post().to(graphql_handler))
            .route(web::get().to(graphiql)),
    )
    .service(web::resource("/graphql/ws").route(web::get().to(graphql_ws_handler)))
    .service(
        web::scope("/notebook/images")
            .route("/upload", web::post().to(upload_notebook_image))
            .route("/{id}", web::delete().to(delete_notebook_image))
            .route("/{id}", web::get().to(get_notebook_image)),
    );
}
