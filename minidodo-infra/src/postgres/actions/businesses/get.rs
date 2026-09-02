use minidodo_core::Business;
use snafu::ResultExt;
use uuid::Uuid;

use crate::postgres::connection::ConnectionPool;
use crate::postgres::error::{QueryFailedSnafu, Result};

pub async fn get_by_id(
    pool: &ConnectionPool,
    id: Uuid,
) -> Result<Option<Business>> {
    sqlx::query_as::<_, Business>(
        r#"
        select id, name, created_at
        from businesses
        where id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context(QueryFailedSnafu)
}
