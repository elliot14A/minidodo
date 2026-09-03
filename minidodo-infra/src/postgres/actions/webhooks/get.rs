use minidodo_core::models::webhook::WebhookEndpoint;
use snafu::ResultExt;
use uuid::Uuid;

use crate::postgres::connection::ConnectionPool;
use crate::postgres::error::{QueryFailedSnafu, Result};

pub async fn get_endpoint_by_id(
    pool: &ConnectionPool,
    business_id: Uuid,
    id: Uuid,
) -> Result<Option<WebhookEndpoint>> {
    sqlx::query_as::<_, WebhookEndpoint>(
        r#"
        select id, business_id, url, signing_secret, active, created_at
        from webhooks
        where business_id = $1 and id = $2
        "#,
    )
    .bind(business_id)
    .bind(id)
    .fetch_optional(pool)
    .await
    .context(QueryFailedSnafu)
}
