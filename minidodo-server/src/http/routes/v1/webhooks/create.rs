use axum::{extract::Extension, http::StatusCode, response::IntoResponse};
use minidodo_core::models::webhook::{NewWebhookEndpoint, WebhookEndpoint};
use minidodo_core::Result;
use minidodo_infra::postgres::connection::ConnectionPool;
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::http::middleware::{AuthContext, ValidatedJson};
use crate::http::routes::v1::JsonResponse;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateWebhookRequest {
    #[validate(url(message = "Invalid URL"))]
    pub url: String,
}

#[utoipa::path(
    post,
    path = "/",
    operation_id = "createWebhookEndpoint",
    request_body = CreateWebhookRequest,
    responses(
        (status = 201, description = "Webhook endpoint registered; signing_secret is returned once", body = inline(JsonResponse<WebhookEndpoint>)),
        (status = 400, description = "Bad request - invalid input"),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("BearerAuth" = [])
    ),
    tag = "webhooks"
)]
#[tracing::instrument(name = "create_webhook_endpoint", skip(pool, body), fields(business_id = %business_id))]
pub async fn create(
    AuthContext { business_id }: AuthContext,
    Extension(pool): Extension<ConnectionPool>,
    ValidatedJson(body): ValidatedJson<CreateWebhookRequest>,
) -> Result<impl IntoResponse> {
    let signing_secret = format!("whsec_{}", Uuid::new_v4().simple());

    let new_endpoint = NewWebhookEndpoint {
        url: body.url,
        signing_secret,
    };

    let endpoint = minidodo_infra::postgres::actions::webhooks::create_endpoint(&pool, business_id, &new_endpoint).await?;

    Ok((
        StatusCode::CREATED,
        JsonResponse::with_message(endpoint, "Webhook endpoint registered successfully"),
    ))
}
