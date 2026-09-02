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
    operation_id = "getCustomerById",
    params(
        ("id" = Uuid, Path, description = "Customer UUID")
    ),
    responses(
        (status = 200, description = "Customer fetched successfully", body = inline(JsonResponse<minidodo_core::Customer>)),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Customer not found")
    ),
    security(
        ("BearerAuth" = [])
    ),
    tag = "customers"
)]
#[tracing::instrument(name = "get_customer_by_id", skip(pool), fields(business_id = %business_id, customer_id = %id))]
pub async fn get(
    AuthContext { business_id }: AuthContext,
    Path(id): Path<Uuid>,
    Extension(pool): Extension<ConnectionPool>,
) -> Result<impl IntoResponse> {
    let customer = minidodo_infra::postgres::actions::customers::get_by_id(&pool, business_id, id)
        .await?
        .ok_or_else(|| MinidodoError::NotFound {
            details: "Customer not found".to_string(),
            code: DatabaseErrorCode::RECORD_NOT_FOUND,
        })?;

    Ok(JsonResponse::success(customer))
}
