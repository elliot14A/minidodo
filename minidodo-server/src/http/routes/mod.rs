pub mod v1;

pub use v1::ApiDoc;

pub fn routes() -> axum::Router {
    axum::Router::new().nest("/v1", v1::routes())
}
