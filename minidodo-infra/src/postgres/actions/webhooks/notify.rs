use snafu::ResultExt;
use uuid::Uuid;

use crate::postgres::connection::ConnectionPool;
use crate::postgres::error::{QueryFailedSnafu, Result};

pub async fn notify_webhook(pool: &ConnectionPool, delivery_id: Uuid) -> Result<()> {
    sqlx::query(r#"select pg_notify('webhooks', $1)"#)
        .bind(delivery_id.to_string())
        .execute(pool)
        .await
        .context(QueryFailedSnafu)?;

    Ok(())
}
