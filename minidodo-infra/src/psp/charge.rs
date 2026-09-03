use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

fn get_http_client() -> Client {
    Client::builder()
        .pool_max_idle_per_host(10)
        .build()
        .unwrap_or_default()
}

#[derive(Debug, Clone, Serialize)]
struct CreateChargePayload<'a> {
    amount_cents: i64,
    card_token: &'a str,
    idempotency_key: Option<&'a str>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PspResponse {
    pub status: String,
    pub psp_ref: Option<Uuid>,
    pub code: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PspOutcome {
    Success { psp_ref: Uuid },
    DefinitiveFailure { error_code: Option<String> },
    Unknown,
}

pub async fn charge(
    base_url: &str,
    amount_cents: i64,
    card_token: &str,
    derived_key: &str,
) -> PspOutcome {
    let client = get_http_client();
    let url = format!("{}/v1/charges", base_url.trim_end_matches('/'));

    let payload = CreateChargePayload {
        amount_cents,
        card_token,
        idempotency_key: Some(derived_key),
    };

    let response = client
        .post(&url)
        .header("idempotency-key", derived_key)
        .json(&payload)
        .timeout(Duration::from_secs(45))
        .send()
        .await;

    match response {
        Ok(res) => {
            let status = res.status();
            if status.is_success() {
                match res.json::<PspResponse>().await {
                    Ok(body) if body.status == "succeeded" => match body.psp_ref {
                        Some(psp_ref) => PspOutcome::Success { psp_ref },
                        None => {
                            tracing::error!("PSP returned succeeded but omitted psp_ref");
                            PspOutcome::Unknown
                        }
                    },
                    Ok(body) => PspOutcome::DefinitiveFailure {
                        error_code: body.code,
                    },
                    Err(_) => PspOutcome::Unknown,
                }
            } else if status.is_client_error() {
                let code = match res.json::<PspResponse>().await {
                    Ok(body) => body.code,
                    Err(_) => Some("client_error".to_string()),
                };
                PspOutcome::DefinitiveFailure { error_code: code }
            } else {
                let code = match res.json::<PspResponse>().await {
                    Ok(body) => body.code,
                    Err(_) => None,
                };
                if code.as_deref() == Some("network_error") {
                    PspOutcome::DefinitiveFailure { error_code: code }
                } else {
                    PspOutcome::Unknown
                }
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "PSP HTTP call failed (network error or timeout)");
            PspOutcome::Unknown
        }
    }
}
