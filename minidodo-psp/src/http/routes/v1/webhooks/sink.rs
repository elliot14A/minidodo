use axum::{
    body::Bytes,
    extract::Extension,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use minidodo_core::models::webhook::verify_signature;

use crate::http::state::PspState;

pub async fn sink(
    Extension(state): Extension<PspState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let signature = headers
        .get("x-webhook-signature")
        .and_then(|v| v.to_str().ok());

    let timestamp_str = headers
        .get("x-webhook-timestamp")
        .and_then(|v| v.to_str().ok());

    let (Some(sig), Some(ts_raw)) = (signature, timestamp_str) else {
        tracing::warn!("Webhook sink received request missing signature or timestamp headers");
        return (StatusCode::UNAUTHORIZED, "missing signature headers").into_response();
    };

    let Ok(timestamp) = ts_raw.parse::<i64>() else {
        tracing::warn!("Webhook sink received invalid timestamp header format");
        return (StatusCode::UNAUTHORIZED, "invalid timestamp").into_response();
    };

    let valid = verify_signature(&state.webhook_signing_secret, timestamp, &body, sig);

    if !valid {
        tracing::warn!("Webhook sink signature verification failed");
        return (StatusCode::UNAUTHORIZED, "invalid signature").into_response();
    }

    tracing::info!("Webhook received and verified successfully at mock sink");
    (StatusCode::OK, "OK").into_response()
}
