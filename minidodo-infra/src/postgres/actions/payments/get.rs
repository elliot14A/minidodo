use minidodo_core::models::payment_attempt::PaymentAttempt;
use snafu::ResultExt;

use crate::postgres::connection::ConnectionPool;
use crate::postgres::error::{QueryFailedSnafu, Result};

pub async fn find_stale(pool: &ConnectionPool) -> Result<Vec<PaymentAttempt>> {
    let attempts = sqlx::query_as::<_, PaymentAttempt>(
        r#"
        select
            pa.id,
            pa.invoice_id,
            pa.business_id,
            pa.idempotency_key,
            pa.payload_hash,
            pa.card_token,
            pa.status,
            pa.psp_ref,
            pa.psp_error_code,
            pa.created_at
        from idempotency_keys ik
        join payment_attempts pa
          on ik.business_id = pa.business_id
         and ik.idempotency_key = pa.idempotency_key
        where ik.recovery_point <> 'finished'
          and ik.locked_at < current_timestamp - interval '60 seconds'
        order by ik.locked_at asc
        limit 50
        "#,
    )
    .fetch_all(pool)
    .await
    .context(QueryFailedSnafu)?;

    Ok(attempts)
}
