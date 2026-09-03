use axum::{routing::post, Router};

pub fn routes() -> Router {
    Router::new().route("/sink", post(super::sink::sink))
}
