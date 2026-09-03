use axum::{extract::Extension, response::IntoResponse};
use minidodo_core::models::webhook::WebhookEndpoint;
use minidodo_core::Result;
use minidodo_infra::postgres::connection::ConnectionPool;

use crate::http::middleware::AuthContext;
use crate::http::routes::v1::JsonResponse;

#[utoipa::path(
    get,
    path = "/",
    operation_id = "listWebhookEndpoints",
    responses(
        (status = 200, description = "Webhook endpoints fetched successfully", body = inline(JsonResponse<Vec<WebhookEndpoint>>)),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("BearerAuth" = [])
    ),
    tag = "webhooks"
)]
#[tracing::instrument(name = "list_webhook_endpoints", skip(pool), fields(business_id = %business_id))]
pub async fn list(
    AuthContext { business_id }: AuthContext,
    Extension(pool): Extension<ConnectionPool>,
) -> Result<impl IntoResponse> {
    let endpoints = minidodo_infra::postgres::actions::webhooks::list_endpoints_by_business(&pool, business_id).await?;

    Ok(JsonResponse::success(endpoints))
}
