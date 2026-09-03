use axum::{
    extract::{Extension, Path},
    http::{HeaderMap, StatusCode},
};
use minidodo_core::{
    compute_payload_hash, DatabaseErrorCode, MinidodoError, PaymentErrorCode, Result,
};
use minidodo_core::http::ValidatedJson;
use minidodo_infra::postgres::connection::ConnectionPool;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::http::middleware::AuthContext;
use crate::http::routes::v1::JsonResponse;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct PayInvoiceRequest {
    #[validate(length(min = 1, message = "Card token is required"))]
    pub card_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PayInvoiceResponse {
    pub attempt_id: Uuid,
    pub invoice_id: Uuid,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub psp_ref: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
}

#[utoipa::path(
    post,
    path = "/{id}/pay",
    operation_id = "payInvoice",
    params(
        ("id" = Uuid, Path, description = "Invoice UUID"),
        ("Idempotency-Key" = String, Header, description = "Idempotency key for safe payment retries")
    ),
    request_body = PayInvoiceRequest,
    responses(
        (status = 202, description = "Payment claim accepted and processing", body = inline(JsonResponse<PayInvoiceResponse>)),
        (status = 400, description = "Missing or invalid idempotency key / request body"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Invoice not found"),
        (status = 409, description = "Invoice not payable in current state"),
        (status = 422, description = "Idempotency key conflict with different payload")
    ),
    security(
        ("BearerAuth" = [])
    ),
    tag = "invoices"
)]
#[tracing::instrument(name = "pay_invoice", skip(pool, headers, body), fields(business_id = %business_id, invoice_id = %id))]
pub async fn pay(
    AuthContext { business_id }: AuthContext,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Extension(pool): Extension<ConnectionPool>,
    ValidatedJson(body): ValidatedJson<PayInvoiceRequest>,
) -> Result<(StatusCode, JsonResponse<PayInvoiceResponse>)> {
    let idempotency_key = headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| MinidodoError::BadRequest {
            message: "Idempotency-Key header is required".to_string(),
            code: "VALIDATION_REQUIRED_HEADER_MISSING",
        })?;

    let payload_hash = compute_payload_hash(business_id, id, &body.card_token);

    let existing_record = minidodo_infra::postgres::actions::payments::lookup(
        &pool,
        business_id,
        idempotency_key,
    )
    .await?;

    if let Some(record) = existing_record {
        if record.payload_hash != payload_hash {
            return Err(MinidodoError::UnprocessableEntity {
                message: "idempotency key reused with a different payload".to_string(),
                code: PaymentErrorCode::IDEMPOTENCY_KEY_CONFLICT,
            });
        }

        if let (Some(code), Some(body_json)) = (record.response_code, record.response_body) {
            let status = StatusCode::from_u16(code as u16).unwrap_or(StatusCode::OK);
            let response_data: PayInvoiceResponse = serde_json::from_value(body_json).map_err(|e| {
                MinidodoError::Internal {
                    message: format!("Failed to parse stored idempotency body: {}", e),
                    code: minidodo_core::SystemErrorCode::INTERNAL_ERROR,
                }
            })?;

            let message = if response_data.status == "paid" {
                "Payment succeeded"
            } else {
                "Payment failed"
            };

            return Ok((
                status,
                JsonResponse::with_message(response_data, message),
            ));
        }

        let response_data = PayInvoiceResponse {
            attempt_id: Uuid::nil(),
            invoice_id: id,
            status: "processing".to_string(),
            psp_ref: None,
            error_code: None,
        };
        return Ok((
            StatusCode::ACCEPTED,
            JsonResponse::with_message(response_data, "Payment attempt is currently processing"),
        ));
    }

    let mut tx = pool.begin().await.map_err(|e| MinidodoError::Internal {
        message: format!("Failed to begin transaction: {}", e),
        code: minidodo_core::SystemErrorCode::INTERNAL_ERROR,
    })?;

    let claimed_rows = minidodo_infra::postgres::actions::payments::claim(&mut tx, business_id, id).await?;

    if claimed_rows == 0 {
        let invoice = minidodo_infra::postgres::actions::invoices::get_by_id(&pool, business_id, id).await?;
        return match invoice {
            None => Err(MinidodoError::NotFound {
                details: "Invoice not found".to_string(),
                code: DatabaseErrorCode::RECORD_NOT_FOUND,
            }),
            Some(inv) => Err(MinidodoError::Conflict {
                message: format!("invoice not payable in state '{}'", inv.invoice.state),
                code: PaymentErrorCode::INVOICE_NOT_PAYABLE,
            }),
        };
    }

    let attempt = minidodo_infra::postgres::actions::payments::create(
        &mut tx,
        id,
        business_id,
        idempotency_key,
        &payload_hash,
        &body.card_token,
    )
    .await?;

    tx.commit().await.map_err(|e| MinidodoError::Internal {
        message: format!("Failed to commit transaction: {}", e),
        code: minidodo_core::SystemErrorCode::INTERNAL_ERROR,
    })?;

    let _ = minidodo_infra::postgres::actions::payments::notify_payment(&pool, attempt.id).await;

    let response_data = PayInvoiceResponse {
        attempt_id: attempt.id,
        invoice_id: id,
        status: "processing".to_string(),
        psp_ref: None,
        error_code: None,
    };

    Ok((
        StatusCode::ACCEPTED,
        JsonResponse::with_message(response_data, "Payment processing started"),
    ))
}
