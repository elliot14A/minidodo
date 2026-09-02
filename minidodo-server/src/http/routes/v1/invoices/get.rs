use axum::{
    extract::{Extension, Path},
    response::IntoResponse,
};
use minidodo_core::{DatabaseErrorCode, MinidodoError, Result};
use minidodo_infra::postgres::connection::ConnectionPool;
use uuid::Uuid;

use crate::http::middleware::AuthContext;
use crate::http::routes::v1::JsonResponse;

#[utoipa::path(
    get,
    path = "/{id}",
    operation_id = "getInvoiceById",
    params(
        ("id" = Uuid, Path, description = "Invoice UUID")
    ),
    responses(
        (status = 200, description = "Invoice fetched successfully", body = inline(JsonResponse<minidodo_core::InvoiceWithLineItems>)),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Invoice not found")
    ),
    security(
        ("BearerAuth" = [])
    ),
    tag = "invoices"
)]
#[tracing::instrument(name = "get_invoice_by_id", skip(pool), fields(business_id = %business_id, invoice_id = %id))]
pub async fn get(
    AuthContext { business_id }: AuthContext,
    Path(id): Path<Uuid>,
    Extension(pool): Extension<ConnectionPool>,
) -> Result<impl IntoResponse> {
    let invoice = minidodo_infra::postgres::actions::invoices::get_by_id(&pool, business_id, id)
        .await?
        .ok_or_else(|| MinidodoError::NotFound {
            details: "Invoice not found".to_string(),
            code: DatabaseErrorCode::RECORD_NOT_FOUND,
        })?;

    Ok(JsonResponse::success(invoice))
}
