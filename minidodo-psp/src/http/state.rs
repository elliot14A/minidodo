use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PspState {
    pub webhook_signing_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PspChargeStatus {
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PspChargeResponse {
    pub status: PspChargeStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub psp_ref: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}
