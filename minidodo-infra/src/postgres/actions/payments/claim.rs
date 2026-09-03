use snafu::ResultExt;
use uuid::Uuid;

use crate::postgres::connection::ConnectionPool;
use crate::postgres::error::{QueryFailedSnafu, Result};

pub async fn claim_in_flight(
    pool: &ConnectionPool,
    business_id: Uuid,
    idempotency_key: &str,
) -> Result<bool> {
    let result = sqlx::query(
        r#"
        with claimed as (
            select business_id, idempotency_key
            from idempotency_keys
            where business_id = $1
              and idempotency_key = $2
              and recovery_point = 'charge_pending'
            for update skip locked
        )
        update idempotency_keys ik
        set locked_at = current_timestamp
        from claimed
        where ik.business_id = claimed.business_id
          and ik.idempotency_key = claimed.idempotency_key
        "#,
    )
    .bind(business_id)
    .bind(idempotency_key)
    .execute(pool)
    .await
    .context(QueryFailedSnafu)?;

    Ok(result.rows_affected() > 0)
}
