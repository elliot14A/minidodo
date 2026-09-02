use axum::{http::StatusCode, response::IntoResponse, Json};
use minidodo_core::http::ValidatedJson;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;
use validator::Validate;

use crate::http::state::{PspChargeResponse, PspChargeStatus};

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateChargeRequest {
    #[validate(range(min = 1, message = "Amount in cents must be greater than 0"))]
    pub amount_cents: i64,
    #[validate(length(min = 1, message = "Card token is required"))]
    pub card_token: String,
}

#[tracing::instrument(name = "create_charge", skip(body))]
pub async fn create(ValidatedJson(body): ValidatedJson<CreateChargeRequest>) -> axum::response::Response {
    let (status_code, response) = match body.card_token.as_str() {
        "tok_timeout" => {
            tokio::time::sleep(Duration::from_secs(30)).await;
            (
                StatusCode::OK,
                PspChargeResponse {
                    status: PspChargeStatus::Succeeded,
                    psp_ref: Some(Uuid::new_v4()),
                    code: None,
                },
            )
        }
        "tok_network_error" => (
            StatusCode::INTERNAL_SERVER_ERROR,
            PspChargeResponse {
                status: PspChargeStatus::Failed,
                psp_ref: None,
                code: Some("network_error".to_string()),
            },
        ),
        "tok_card_declined" => (
            StatusCode::BAD_REQUEST,
            PspChargeResponse {
                status: PspChargeStatus::Failed,
                psp_ref: None,
                code: Some("card_declined".to_string()),
            },
        ),
        "tok_insufficient_funds" => (
            StatusCode::BAD_REQUEST,
            PspChargeResponse {
                status: PspChargeStatus::Failed,
                psp_ref: None,
                code: Some("insufficient_funds".to_string()),
            },
        ),
        _ => (
            StatusCode::OK,
            PspChargeResponse {
                status: PspChargeStatus::Succeeded,
                psp_ref: Some(Uuid::new_v4()),
                code: None,
            },
        ),
    };

    (status_code, Json(response)).into_response()
}
