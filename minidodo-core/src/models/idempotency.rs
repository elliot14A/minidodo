use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "recovery_point", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum RecoveryPoint {
    ChargePending,
    Finished,
}

impl std::fmt::Display for RecoveryPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ChargePending => write!(f, "charge_pending"),
            Self::Finished => write!(f, "finished"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct IdempotencyRecord {
    pub business_id: Uuid,
    pub idempotency_key: String,
    pub payload_hash: String,
    pub recovery_point: RecoveryPoint,
    pub locked_at: DateTime<Utc>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub response_code: Option<i32>,
    pub response_body: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

pub fn compute_payload_hash(business_id: Uuid, invoice_id: Uuid, card_token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(business_id.as_bytes());
    hasher.update(b":");
    hasher.update(invoice_id.as_bytes());
    hasher.update(b":");
    hasher.update(card_token.as_bytes());
    hex::encode(hasher.finalize())
}
