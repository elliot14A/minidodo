use std::time::Duration;
use chrono::Utc;
use minidodo_core::models::webhook::{
    backoff_secs, sign_payload, WebhookDelivery, WebhookEndpoint, WebhookEventPayload,
    WEBHOOK_MAX_ATTEMPTS,
};
use minidodo_core::Result;
use reqwest::Client;
use tracing::{error, info, warn};

use crate::postgres::connection::ConnectionPool;

fn get_http_client() -> Client {
    Client::builder()
        .pool_max_idle_per_host(10)
        .build()
        .unwrap_or_default()
}

pub async fn deliver(
    pool: &ConnectionPool,
    delivery: WebhookDelivery,
    endpoint: WebhookEndpoint,
) -> Result<()> {
    let client = get_http_client();
    let event_payload = WebhookEventPayload {
        event_id: delivery.id,
        event_type: delivery.event_type,
        created_at: delivery.created_at,
        data: delivery.payload,
    };

    let body_bytes = match serde_json::to_vec(&event_payload) {
        Ok(b) => b,
        Err(e) => {
            error!(error = %e, delivery_id = %delivery.id, "Failed to serialize webhook event payload");
            crate::postgres::actions::webhooks::mark_failed(
                pool,
                delivery.id,
                1,
                &format!("serialization error: {}", e),
            )
            .await?;
            return Ok(());
        }
    };

    let mut last_error = String::from("unknown error");

    for attempt in 1..=WEBHOOK_MAX_ATTEMPTS {
        let timestamp = Utc::now().timestamp();
        let signature = sign_payload(&endpoint.signing_secret, timestamp, &body_bytes);

        info!(
            delivery_id = %delivery.id,
            endpoint_url = %endpoint.url,
            attempt = attempt,
            "Delivering webhook HTTP POST via infra"
        );

        let response = client
            .post(&endpoint.url)
            .header("content-type", "application/json")
            .header("x-webhook-signature", format!("sha256={}", signature))
            .header("x-webhook-timestamp", timestamp.to_string())
            .body(body_bytes.clone())
            .timeout(Duration::from_secs(10))
            .send()
            .await;

        match response {
            Ok(res) if res.status().is_success() => {
                info!(
                    delivery_id = %delivery.id,
                    attempt = attempt,
                    "Webhook delivery succeeded"
                );
                crate::postgres::actions::webhooks::mark_delivered(
                    pool,
                    delivery.id,
                    attempt,
                )
                .await?;
                return Ok(());
            }
            Ok(res) => {
                let status = res.status();
                last_error = format!("HTTP {}", status);
                warn!(
                    delivery_id = %delivery.id,
                    attempt = attempt,
                    status = %status,
                    "Webhook delivery received non-2xx status"
                );
            }
            Err(e) => {
                last_error = format!("transport error: {}", e);
                warn!(
                    delivery_id = %delivery.id,
                    attempt = attempt,
                    error = %e,
                    "Webhook delivery network error"
                );
            }
        }

        if attempt < WEBHOOK_MAX_ATTEMPTS {
            let delay = backoff_secs(attempt);
            tokio::time::sleep(Duration::from_secs(delay)).await;
        }
    }

    error!(
        delivery_id = %delivery.id,
        attempts = WEBHOOK_MAX_ATTEMPTS,
        last_error = %last_error,
        code = minidodo_core::error_codes::WebhookErrorCode::DELIVERY_FAILED,
        "Webhook delivery exhausted all attempts and failed"
    );

    crate::postgres::actions::webhooks::mark_failed(
        pool,
        delivery.id,
        WEBHOOK_MAX_ATTEMPTS,
        &last_error,
    )
    .await?;

    Ok(())
}
