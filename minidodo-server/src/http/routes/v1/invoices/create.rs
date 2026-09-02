use axum::{extract::Extension, http::StatusCode, response::IntoResponse};
use chrono::NaiveDate;
use minidodo_core::{NewInvoice, NewLineItem, Result};
use minidodo_infra::postgres::connection::ConnectionPool;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::http::middleware::{AuthContext, ValidatedJson};
use crate::http::routes::v1::JsonResponse;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateLineItemRequest {
    #[validate(length(min = 1, message = "Description is required"))]
    pub description: String,
    #[validate(range(min = 1, message = "Quantity must be greater than 0"))]
    pub quantity: i32,
    #[validate(range(min = 0, message = "Unit amount cents must be greater than or equal to 0"))]
    pub unit_amount_cents: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateInvoiceRequest {
    pub customer_id: Uuid,
    pub due_date: NaiveDate,
    #[validate(length(min = 1, message = "Invoice must contain at least one line item"))]
    #[validate(nested)]
    pub line_items: Vec<CreateLineItemRequest>,
}

#[utoipa::path(
    post,
    path = "/",
    operation_id = "createInvoice",
    request_body = CreateInvoiceRequest,
    responses(
        (status = 201, description = "Invoice created successfully", body = inline(JsonResponse<minidodo_core::InvoiceWithLineItems>)),
        (status = 400, description = "Bad request - invalid input"),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("BearerAuth" = [])
    ),
    tag = "invoices"
)]
#[tracing::instrument(name = "create_invoice", skip(pool, body), fields(business_id = %business_id, customer_id = %body.customer_id))]
pub async fn create(
    AuthContext { business_id }: AuthContext,
    Extension(pool): Extension<ConnectionPool>,
    ValidatedJson(body): ValidatedJson<CreateInvoiceRequest>,
) -> Result<impl IntoResponse> {
    let new_invoice = NewInvoice {
        customer_id: body.customer_id,
        due_date: body.due_date,
        line_items: body
            .line_items
            .into_iter()
            .map(|item| NewLineItem {
                description: item.description,
                quantity: item.quantity,
                unit_amount_cents: item.unit_amount_cents,
            })
            .collect(),
    };

    let invoice = minidodo_infra::postgres::actions::invoices::create(&pool, business_id, &new_invoice).await?;

    Ok((
        StatusCode::CREATED,
        JsonResponse::with_message(invoice, "Invoice created successfully"),
    ))
}
