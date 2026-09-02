use axum::{routing::get, Router};

pub fn routes() -> Router {
    Router::new()
        .route("/", get(super::list::list).post(super::create::create))
        .route(
            "/{id}",
            get(super::get::get).patch(super::update::update_state),
        )
}
