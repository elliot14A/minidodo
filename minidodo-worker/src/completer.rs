use minidodo_core::models::payment_attempt::PaymentAttempt;
use minidodo_core::Result;
use minidodo_infra::postgres::actions::payments::{SettleFailureParams, SettleSuccessParams};
use minidodo_infra::postgres::connection::ConnectionPool;
use minidodo_infra::psp::{charge, PspOutcome};
use serde_json::json;
use tracing::{info, warn};

pub async fn complete_attempt(
    pool: &ConnectionPool,
    psp_base_url: &str,
    attempt: &PaymentAttempt,
) -> Result<()> {
    let invoice_with_items = minidodo_infra::postgres::actions::invoices::get_by_id(
        pool,
        attempt.business_id,
        attempt.invoice_id,
    )
    .await?;

    let Some(invoice_data) = invoice_with_items else {
        tracing::warn!(invoice_id = %attempt.invoice_id, "invoice not found for attempt");
        return Ok(());
    };

    let amount_cents = invoice_data.invoice.total_cents;
    let derived_key = format!("invsvc-{}", attempt.id);

    info!(
        attempt_id = %attempt.id,
        invoice_id = %attempt.invoice_id,
        derived_key = %derived_key,
        card_token = %attempt.card_token,
        amount_cents = amount_cents,
        "Worker initiating charge to PSP via infra"
    );

    let outcome = charge(
        psp_base_url,
        amount_cents,
        &attempt.card_token,
        &derived_key,
    )
    .await;

    match outcome {
        PspOutcome::Success { psp_ref } => {
            info!(attempt_id = %attempt.id, psp_ref = %psp_ref, "Settling payment as success");
            let response_body = json!({
                "attempt_id": attempt.id,
                "invoice_id": attempt.invoice_id,
                "status": "paid",
                "psp_ref": psp_ref
            });

            let staged_delivery_ids = minidodo_infra::postgres::actions::payments::settle_success(
                pool,
                SettleSuccessParams {
                    attempt_id: attempt.id,
                    invoice_id: attempt.invoice_id,
                    business_id: attempt.business_id,
                    idempotency_key: &attempt.idempotency_key,
                    psp_ref,
                    response_code: 200,
                    response_body,
                },
            )
            .await?;

            if let Some(delivery_ids) = staged_delivery_ids {
                for delivery_id in delivery_ids {
                    if let Err(e) = minidodo_infra::postgres::actions::webhooks::notify_webhook(pool, delivery_id).await {
                        warn!(error = %e, delivery_id = %delivery_id, "Failed to send pg_notify for webhook delivery");
                    }
                }
            }
        }
        PspOutcome::DefinitiveFailure { error_code } => {
            info!(attempt_id = %attempt.id, error_code = ?error_code, "Settling payment as failed");
            let response_body = json!({
                "attempt_id": attempt.id,
                "invoice_id": attempt.invoice_id,
                "status": "failed",
                "error_code": error_code
            });

            let staged_delivery_ids = minidodo_infra::postgres::actions::payments::settle_failure(
                pool,
                SettleFailureParams {
                    attempt_id: attempt.id,
                    invoice_id: attempt.invoice_id,
                    business_id: attempt.business_id,
                    idempotency_key: &attempt.idempotency_key,
                    psp_error_code: error_code,
                    response_code: 400,
                    response_body,
                },
            )
            .await?;

            if let Some(delivery_ids) = staged_delivery_ids {
                for delivery_id in delivery_ids {
                    if let Err(e) = minidodo_infra::postgres::actions::webhooks::notify_webhook(pool, delivery_id).await {
                        warn!(error = %e, delivery_id = %delivery_id, "Failed to send pg_notify for webhook delivery");
                    }
                }
            }
        }
        PspOutcome::Unknown => {
            tracing::warn!(
                attempt_id = %attempt.id,
                invoice_id = %attempt.invoice_id,
                "PSP outcome indeterminate; leaving in processing for recovery sweep"
            );
        }
    }

    Ok(())
}
