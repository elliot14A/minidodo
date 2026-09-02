use axum::{extract::Extension, response::IntoResponse};
use minidodo_core::{DatabaseErrorCode, MinidodoError, Result};
use minidodo_infra::postgres::connection::ConnectionPool;

use crate::http::middleware::AuthContext;
use crate::http::routes::v1::JsonResponse;

#[utoipa::path(
    get,
    path = "/me",
    operation_id = "getCurrentBusiness",
    responses(
        (status = 200, description = "Current authenticated business", body = inline(JsonResponse<minidodo_core::Business>)),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("BearerAuth" = [])
    ),
    tag = "businesses"
)]
#[tracing::instrument(name = "get_current_business", skip(pool))]
pub async fn get(
    AuthContext { business_id }: AuthContext,
    Extension(pool): Extension<ConnectionPool>,
) -> Result<impl IntoResponse> {
    let business = minidodo_infra::postgres::actions::businesses::get_by_id(&pool, business_id)
        .await?
        .ok_or_else(|| MinidodoError::NotFound {
            details: "Business not found".to_string(),
            code: DatabaseErrorCode::RECORD_NOT_FOUND,
        })?;

    Ok(JsonResponse::success(business))
}
