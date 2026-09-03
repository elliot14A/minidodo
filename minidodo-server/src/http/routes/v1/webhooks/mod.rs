pub mod create;
pub mod get;
pub mod list;
pub mod routes;

pub use routes::routes;

#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        create::create,
        get::get,
        list::list,
    ),
    components(schemas(
        create::CreateWebhookRequest,
        crate::http::routes::v1::JsonResponse<minidodo_core::models::webhook::WebhookEndpoint>,
        crate::http::routes::v1::JsonResponse<Vec<minidodo_core::models::webhook::WebhookEndpoint>>,
        minidodo_core::models::webhook::WebhookEndpoint
    )),
    tags((name = "webhooks", description = "Webhook endpoint registration"))
)]
pub struct WebhooksApiDoc;
