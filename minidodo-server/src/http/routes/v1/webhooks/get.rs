use axum::{
    extract::{Extension, Path},
    response::IntoResponse,
};
use minidodo_core::models::webhook::WebhookEndpoint;
use minidodo_core::{DatabaseErrorCode, MinidodoError, Result};
use minidodo_infra::postgres::connection::ConnectionPool;
use uuid::Uuid;

use crate::http::middleware::AuthContext;
use crate::http::routes::v1::JsonResponse;

#[utoipa::path(
    get,
    path = "/{id}",
    operation_id = "getWebhookEndpointById",
    params(
        ("id" = Uuid, Path, description = "Webhook endpoint UUID")
    ),
    responses(
        (status = 200, description = "Webhook endpoint fetched successfully", body = inline(JsonResponse<WebhookEndpoint>)),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Webhook endpoint not found")
    ),
    security(
        ("BearerAuth" = [])
    ),
    tag = "webhooks"
)]
#[tracing::instrument(name = "get_webhook_endpoint_by_id", skip(pool), fields(business_id = %business_id, endpoint_id = %id))]
pub async fn get(
    AuthContext { business_id }: AuthContext,
    Path(id): Path<Uuid>,
    Extension(pool): Extension<ConnectionPool>,
) -> Result<impl IntoResponse> {
    let endpoint = minidodo_infra::postgres::actions::webhooks::get_endpoint_by_id(&pool, business_id, id)
        .await?
        .ok_or_else(|| MinidodoError::NotFound {
            details: "Webhook endpoint not found".to_string(),
            code: DatabaseErrorCode::RECORD_NOT_FOUND,
        })?;

    Ok(JsonResponse::success(endpoint))
}
