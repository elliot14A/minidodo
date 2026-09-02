use axum::{routing::post, Router};

pub fn routes() -> Router {
    Router::new().route("/", post(super::create::create))
}
