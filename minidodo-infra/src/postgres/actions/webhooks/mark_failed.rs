use snafu::ResultExt;
use uuid::Uuid;

use crate::postgres::connection::ConnectionPool;
use crate::postgres::error::{QueryFailedSnafu, Result};

pub async fn mark_failed(
    pool: &ConnectionPool,
    delivery_id: Uuid,
    attempts: i32,
    error: &str,
) -> Result<bool> {
    let result = sqlx::query(
        r#"
        update webhook_deliveries
        set
            status = 'failed',
            attempts = $1,
            last_error = $2,
            last_attempt_at = current_timestamp
        where id = $3 and status = 'pending'
        "#,
    )
    .bind(attempts)
    .bind(error)
    .bind(delivery_id)
    .execute(pool)
    .await
    .context(QueryFailedSnafu)?;

    Ok(result.rows_affected() > 0)
}
