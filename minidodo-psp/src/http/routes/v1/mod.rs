use axum::routing::get;

pub mod charges;

pub fn routes() -> axum::Router {
    axum::Router::new()
        .route("/health", get(health_check))
        .nest("/charges", charges::routes())
}

#[tracing::instrument(name = "health_check")]
async fn health_check() -> &'static str {
    "OK"
}
