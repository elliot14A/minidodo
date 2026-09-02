use axum::{extract::Extension, http::StatusCode, response::IntoResponse};
use minidodo_core::{NewCustomer, Result};
use minidodo_infra::postgres::connection::ConnectionPool;
use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

use crate::http::middleware::{AuthContext, ValidatedJson};
use crate::http::routes::v1::JsonResponse;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateCustomerRequest {
    #[validate(length(min = 1, max = 255, message = "Name must be between 1 and 255 characters"))]
    pub name: String,
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
}

#[utoipa::path(
    post,
    path = "/",
    operation_id = "createCustomer",
    request_body = CreateCustomerRequest,
    responses(
        (status = 201, description = "Customer created successfully", body = inline(JsonResponse<minidodo_core::Customer>)),
        (status = 400, description = "Bad request - invalid input"),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("BearerAuth" = [])
    ),
    tag = "customers"
)]
#[tracing::instrument(name = "create_customer", skip(pool, body), fields(business_id = %business_id, customer_email = %body.email))]
pub async fn create(
    AuthContext { business_id }: AuthContext,
    Extension(pool): Extension<ConnectionPool>,
    ValidatedJson(body): ValidatedJson<CreateCustomerRequest>,
) -> Result<impl IntoResponse> {
    let new_customer = NewCustomer {
        name: body.name,
        email: body.email,
    };

    let customer = minidodo_infra::postgres::actions::customers::create(&pool, business_id, &new_customer).await?;

    Ok((
        StatusCode::CREATED,
        JsonResponse::with_message(customer, "Customer created successfully"),
    ))
}
