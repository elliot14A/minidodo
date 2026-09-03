use minidodo_core::models::idempotency::RecoveryPoint;
use minidodo_core::models::payment_attempt::PaymentStatus;
use serde_json::Value;
use snafu::ResultExt;
use uuid::Uuid;

use crate::postgres::connection::ConnectionPool;
use crate::postgres::error::{QueryFailedSnafu, Result, TransactionFailedSnafu};

pub struct SettleFailureParams<'a> {
    pub attempt_id: Uuid,
    pub invoice_id: Uuid,
    pub business_id: Uuid,
    pub idempotency_key: &'a str,
    pub psp_error_code: Option<String>,
    pub response_code: i32,
    pub response_body: Value,
}

pub async fn settle_failure(pool: &ConnectionPool, params: SettleFailureParams<'_>) -> Result<bool> {
    let mut tx = pool.begin().await.context(TransactionFailedSnafu)?;

    let attempt_result = sqlx::query(
        r#"
        update payment_attempts
        set status = $1, psp_error_code = $2
        where id = $3 and business_id = $4 and status = 'pending'
        "#,
    )
    .bind(PaymentStatus::Failed)
    .bind(params.psp_error_code)
    .bind(params.attempt_id)
    .bind(params.business_id)
    .execute(&mut *tx)
    .await
    .context(QueryFailedSnafu)?;

    if attempt_result.rows_affected() == 0 {
        return Ok(false);
    }

    sqlx::query(
        r#"
        update invoices
        set state = 'open'
        where id = $1 and business_id = $2 and state = 'processing'
        "#,
    )
    .bind(params.invoice_id)
    .bind(params.business_id)
    .execute(&mut *tx)
    .await
    .context(QueryFailedSnafu)?;

    sqlx::query(
        r#"
        update idempotency_keys
        set
            recovery_point = $1,
            last_run_at = current_timestamp,
            response_code = $2,
            response_body = $3
        where business_id = $4 and idempotency_key = $5
        "#,
    )
    .bind(RecoveryPoint::Finished)
    .bind(params.response_code)
    .bind(params.response_body)
    .bind(params.business_id)
    .bind(params.idempotency_key)
    .execute(&mut *tx)
    .await
    .context(QueryFailedSnafu)?;

    tx.commit().await.context(TransactionFailedSnafu)?;

    Ok(true)
}
