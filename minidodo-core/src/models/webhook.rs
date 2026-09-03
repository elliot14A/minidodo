use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Sha256;
use utoipa::ToSchema;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

pub const WEBHOOK_MAX_ATTEMPTS: i32 = 5;
pub const WEBHOOK_BACKOFF_BASE_SECS: u64 = 1;
pub const WEBHOOK_BACKOFF_CAP_SECS: u64 = 30;

pub fn backoff_secs(attempt: i32) -> u64 {
    if attempt <= 1 {
        return WEBHOOK_BACKOFF_BASE_SECS;
    }
    let shift = (attempt - 1).min(10) as u32;
    let multiplier = 1u64.checked_shl(shift).unwrap_or(u64::MAX);
    let delay = WEBHOOK_BACKOFF_BASE_SECS.saturating_mul(multiplier);
    delay.min(WEBHOOK_BACKOFF_CAP_SECS)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "webhook_delivery_status", rename_all = "lowercase")]
pub enum WebhookDeliveryStatus {
    Pending,
    Delivered,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "webhook_event_type")]
pub enum WebhookEventType {
    #[serde(rename = "invoice.paid")]
    #[sqlx(rename = "invoice.paid")]
    InvoicePaid,
    #[serde(rename = "invoice.payment_failed")]
    #[sqlx(rename = "invoice.payment_failed")]
    InvoicePaymentFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct WebhookEndpoint {
    pub id: Uuid,
    pub business_id: Uuid,
    pub url: String,
    pub signing_secret: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct WebhookDelivery {
    pub id: Uuid,
    pub endpoint_id: Uuid,
    pub business_id: Uuid,
    pub event_type: WebhookEventType,
    pub payload: Value,
    pub status: WebhookDeliveryStatus,
    pub attempts: i32,
    pub last_error: Option<String>,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WebhookEventPayload {
    pub event_id: Uuid,
    pub event_type: WebhookEventType,
    pub created_at: DateTime<Utc>,
    pub data: Value,
}

pub fn sign_payload(secret: &str, timestamp: i64, body: &[u8]) -> String {
    match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(mut mac) => {
            mac.update(timestamp.to_string().as_bytes());
            mac.update(b".");
            mac.update(body);
            let result = mac.finalize();
            hex::encode(result.into_bytes())
        }
        Err(_) => String::new(),
    }
}

pub fn verify_signature(secret: &str, timestamp: i64, body: &[u8], signature_header: &str) -> bool {
    let signature = signature_header.strip_prefix("sha256=").unwrap_or(signature_header);
    let expected = sign_payload(secret, timestamp, body);
    if expected.is_empty() || signature.is_empty() {
        return false;
    }
    subtle::ConstantTimeEq::ct_eq(expected.as_bytes(), signature.as_bytes()).into()
}
