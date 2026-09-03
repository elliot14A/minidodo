use snafu::ResultExt;
use uuid::Uuid;

use crate::postgres::connection::ConnectionPool;
use crate::postgres::error::{QueryFailedSnafu, Result};

pub async fn mark_delivered(
    pool: &ConnectionPool,
    delivery_id: Uuid,
    attempts: i32,
) -> Result<bool> {
    let result = sqlx::query(
        r#"
        update webhook_deliveries
        set
            status = 'delivered',
            attempts = $1,
            last_attempt_at = current_timestamp
        where id = $2 and status = 'pending'
        "#,
    )
    .bind(attempts)
    .bind(delivery_id)
    .execute(pool)
    .await
    .context(QueryFailedSnafu)?;

    Ok(result.rows_affected() > 0)
}
