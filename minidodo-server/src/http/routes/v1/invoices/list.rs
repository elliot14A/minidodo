use axum::{
    extract::{Extension, Query},
    response::IntoResponse,
};
use minidodo_core::{InvoiceState, Pagination, Result};
use minidodo_infra::postgres::connection::ConnectionPool;
use serde::Deserialize;
use uuid::Uuid;

use crate::http::middleware::AuthContext;
use crate::http::routes::v1::JsonResponse;

#[derive(Debug, Deserialize)]
pub struct ListInvoicesQuery {
    pub state: Option<InvoiceState>,
    pub customer_id: Option<Uuid>,
}

#[utoipa::path(
    get,
    path = "/",
    operation_id = "listInvoices",
    params(
        ("state" = Option<InvoiceState>, Query, description = "Filter by invoice state"),
        ("customer_id" = Option<Uuid>, Query, description = "Filter by customer ID"),
        ("page" = Option<u32>, Query, description = "Page number for pagination (minimum 1)", example = 1),
        ("limit" = Option<u32>, Query, description = "Number of items per page (minimum 1, maximum 100)", example = 10),
        ("sort_order" = Option<String>, Query, description = "Sort order (asc or desc)", example = "desc")
    ),
    responses(
        (status = 200, description = "Invoices fetched successfully", body = inline(JsonResponse<minidodo_core::PaginationResult<minidodo_core::Invoice>>)),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("BearerAuth" = [])
    ),
    tag = "invoices"
)]
#[tracing::instrument(name = "list_invoices", skip(pool, pagination), fields(business_id = %business_id))]
pub async fn list(
    AuthContext { business_id }: AuthContext,
    Query(filters): Query<ListInvoicesQuery>,
    pagination: Pagination,
    Extension(pool): Extension<ConnectionPool>,
) -> Result<impl IntoResponse> {
    let result = minidodo_infra::postgres::actions::invoices::list_by_business(
        &pool,
        business_id,
        filters.state,
        filters.customer_id,
        pagination,
    )
    .await?;

    Ok(JsonResponse::success(result))
}
