use minidodo_core::models::idempotency::IdempotencyRecord;
use snafu::ResultExt;
use uuid::Uuid;

use crate::postgres::connection::ConnectionPool;
use crate::postgres::error::{QueryFailedSnafu, Result};

pub async fn lookup(
    pool: &ConnectionPool,
    business_id: Uuid,
    idempotency_key: &str,
) -> Result<Option<IdempotencyRecord>> {
    let record = sqlx::query_as::<_, IdempotencyRecord>(
        r#"
        select
            business_id,
            idempotency_key,
            payload_hash,
            recovery_point,
            locked_at,
            last_run_at,
            response_code,
            response_body,
            created_at
        from idempotency_keys
        where business_id = $1 and idempotency_key = $2
        "#,
    )
    .bind(business_id)
    .bind(idempotency_key)
    .fetch_optional(pool)
    .await
    .context(QueryFailedSnafu)?;

    Ok(record)
}
