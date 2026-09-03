use minidodo_core::models::webhook::WebhookEndpoint;
use snafu::ResultExt;
use uuid::Uuid;

use crate::postgres::connection::ConnectionPool;
use crate::postgres::error::{QueryFailedSnafu, Result};

pub async fn list_endpoints_by_business(
    pool: &ConnectionPool,
    business_id: Uuid,
) -> Result<Vec<WebhookEndpoint>> {
    sqlx::query_as::<_, WebhookEndpoint>(
        r#"
        select id, business_id, url, signing_secret, active, created_at
        from webhooks
        where business_id = $1
        order by created_at desc
        "#,
    )
    .bind(business_id)
    .fetch_all(pool)
    .await
    .context(QueryFailedSnafu)
}
