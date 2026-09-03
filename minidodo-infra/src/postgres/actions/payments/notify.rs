use snafu::ResultExt;
use uuid::Uuid;

use crate::postgres::connection::ConnectionPool;
use crate::postgres::error::{QueryFailedSnafu, Result};

pub async fn notify_payment(pool: &ConnectionPool, attempt_id: Uuid) -> Result<()> {
    sqlx::query(
        r#"
        select pg_notify('payments', $1)
        "#,
    )
    .bind(attempt_id.to_string())
    .execute(pool)
    .await
    .context(QueryFailedSnafu)?;

    Ok(())
}
