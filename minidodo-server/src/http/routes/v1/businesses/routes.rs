use axum::{routing::get, Router};

pub fn routes() -> Router {
    Router::new().route("/me", get(super::get::get))
}
