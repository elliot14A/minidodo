use minidodo_core::models::idempotency::RecoveryPoint;
use minidodo_core::models::payment_attempt::{PaymentAttempt, PaymentStatus};
use snafu::ResultExt;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::postgres::error::{QueryFailedSnafu, Result};

pub async fn create(
    tx: &mut Transaction<'_, Postgres>,
    invoice_id: Uuid,
    business_id: Uuid,
    idempotency_key: &str,
    payload_hash: &str,
    card_token: &str,
) -> Result<PaymentAttempt> {
    let attempt = sqlx::query_as::<_, PaymentAttempt>(
        r#"
        insert into payment_attempts (
            invoice_id,
            business_id,
            idempotency_key,
            payload_hash,
            card_token,
            status
        )
        values ($1, $2, $3, $4, $5, $6)
        returning
            id,
            invoice_id,
            business_id,
            idempotency_key,
            payload_hash,
            card_token,
            status,
            psp_ref,
            psp_error_code,
            created_at
        "#,
    )
    .bind(invoice_id)
    .bind(business_id)
    .bind(idempotency_key)
    .bind(payload_hash)
    .bind(card_token)
    .bind(PaymentStatus::Pending)
    .fetch_one(&mut **tx)
    .await
    .context(QueryFailedSnafu)?;

    sqlx::query(
        r#"
        insert into idempotency_keys (
            business_id,
            idempotency_key,
            payload_hash,
            recovery_point,
            locked_at
        )
        values ($1, $2, $3, $4, current_timestamp)
        "#,
    )
    .bind(business_id)
    .bind(idempotency_key)
    .bind(payload_hash)
    .bind(RecoveryPoint::ChargePending)
    .execute(&mut **tx)
    .await
    .context(QueryFailedSnafu)?;

    Ok(attempt)
}
