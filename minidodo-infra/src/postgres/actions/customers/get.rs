use minidodo_core::Customer;
use snafu::ResultExt;
use uuid::Uuid;

use crate::postgres::connection::ConnectionPool;
use crate::postgres::error::{QueryFailedSnafu, Result};

pub async fn get_by_id(
    pool: &ConnectionPool,
    business_id: Uuid,
    id: Uuid,
) -> Result<Option<Customer>> {
    sqlx::query_as::<_, Customer>(
        r#"
        select id, business_id, name, email, created_at
        from customers
        where business_id = $1 and id = $2
        "#,
    )
    .bind(business_id)
    .bind(id)
    .fetch_optional(pool)
    .await
    .context(QueryFailedSnafu)
}
