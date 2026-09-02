use axum::{
    extract::{Extension, Path},
    response::IntoResponse,
};
use minidodo_core::{Result, UpdateInvoiceStateTarget};
use minidodo_infra::postgres::connection::ConnectionPool;
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::http::middleware::{AuthContext, ValidatedJson};
use crate::http::routes::v1::JsonResponse;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateInvoiceStateRequest {
    pub state: UpdateInvoiceStateTarget,
}

#[utoipa::path(
    patch,
    path = "/{id}",
    operation_id = "updateInvoiceState",
    params(
        ("id" = Uuid, Path, description = "Invoice UUID")
    ),
    request_body = UpdateInvoiceStateRequest,
    responses(
        (status = 200, description = "Invoice state updated successfully", body = inline(JsonResponse<minidodo_core::Invoice>)),
        (status = 400, description = "Bad request - invalid state"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Invoice not found"),
        (status = 409, description = "Conflict - invalid state transition")
    ),
    security(
        ("BearerAuth" = [])
    ),
    tag = "invoices"
)]
#[tracing::instrument(name = "update_invoice_state", skip(pool, body), fields(business_id = %business_id, invoice_id = %id, target_state = %body.state))]
pub async fn update_state(
    AuthContext { business_id }: AuthContext,
    Path(id): Path<Uuid>,
    Extension(pool): Extension<ConnectionPool>,
    ValidatedJson(body): ValidatedJson<UpdateInvoiceStateRequest>,
) -> Result<impl IntoResponse> {
    let invoice = minidodo_infra::postgres::actions::invoices::update_state(
        &pool,
        business_id,
        id,
        body.state,
    )
    .await?;

    Ok(JsonResponse::with_message(invoice, "Invoice state updated successfully"))
}
