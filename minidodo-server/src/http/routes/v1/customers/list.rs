use axum::{extract::Extension, response::IntoResponse};
use minidodo_core::{Pagination, Result};
use minidodo_infra::postgres::connection::ConnectionPool;

use crate::http::middleware::AuthContext;
use crate::http::routes::v1::JsonResponse;

#[utoipa::path(
    get,
    path = "/",
    operation_id = "listCustomers",
    params(
        ("page" = Option<u32>, Query, description = "Page number for pagination (minimum 1)", example = 1),
        ("limit" = Option<u32>, Query, description = "Number of items per page (minimum 1, maximum 100)", example = 10),
        ("sort_order" = Option<String>, Query, description = "Sort order (asc or desc)", example = "desc")
    ),
    responses(
        (status = 200, description = "Customers fetched successfully", body = inline(JsonResponse<minidodo_core::PaginationResult<minidodo_core::Customer>>)),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("BearerAuth" = [])
    ),
    tag = "customers"
)]
#[tracing::instrument(name = "list_customers", skip(pool, pagination), fields(business_id = %business_id))]
pub async fn list(
    AuthContext { business_id }: AuthContext,
    pagination: Pagination,
    Extension(pool): Extension<ConnectionPool>,
) -> Result<impl IntoResponse> {
    let result = minidodo_infra::postgres::actions::customers::list_by_business(&pool, business_id, pagination).await?;

    Ok(JsonResponse::success(result))
}
