use minidodo_core::models::payment_attempt::PaymentAttempt;
use snafu::ResultExt;
use uuid::Uuid;

use crate::postgres::connection::ConnectionPool;
use crate::postgres::error::{QueryFailedSnafu, Result};

pub async fn get_by_id(
    pool: &ConnectionPool,
    attempt_id: Uuid,
) -> Result<Option<PaymentAttempt>> {
    let attempt = sqlx::query_as::<_, PaymentAttempt>(
        r#"
        select
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
        from payment_attempts
        where id = $1
        "#,
    )
    .bind(attempt_id)
    .fetch_optional(pool)
    .await
    .context(QueryFailedSnafu)?;

    Ok(attempt)
}
