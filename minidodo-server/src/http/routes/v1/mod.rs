use axum::{Json, response::IntoResponse, routing::get};
use utoipa::{
    Modify, OpenApi, ToSchema,
    openapi::Components,
    openapi::security::{Http, HttpAuthScheme, SecurityScheme},
};

pub fn routes() -> axum::Router {
    axum::Router::new()
        .route("/health", get(health_check))
}

#[utoipa::path(
    get,
    path = "/health",
    operation_id = "healthCheck",
    responses(
        (status = 200, description = "Service is healthy", body = String, example = "OK")
    ),
    security(),
    tag = "health"
)]
#[tracing::instrument(name = "health_check")]
async fn health_check() -> &'static str {
    "OK"
}

#[derive(serde::Serialize, ToSchema)]
pub struct JsonResponse<T> {
    #[schema(inline)]
    pub data: T,
    pub message: Option<String>,
}

impl<T> JsonResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            data,
            message: None,
        }
    }

    pub fn with_message(data: T, message: impl Into<String>) -> Self {
        Self {
            data,
            message: Some(message.into()),
        }
    }
}

impl<T> IntoResponse for JsonResponse<T>
where
    T: serde::Serialize,
{
    fn into_response(self) -> axum::response::Response {
        Json(self).into_response()
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(
        health_check,
    ),
    security(
        ("BearerAuth" = [])
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "health", description = "Health check endpoints"),
    )
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Components::new);
        components.add_security_scheme(
            "BearerAuth",
            SecurityScheme::Http(
                Http::builder()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("dodo_key")
                    .build(),
            ),
        );
    }
}
