use minidodo_core::models::webhook::{NewWebhookEndpoint, WebhookEndpoint};
use snafu::ResultExt;
use uuid::Uuid;

use crate::postgres::connection::ConnectionPool;
use crate::postgres::error::{QueryFailedSnafu, Result};

pub async fn create_endpoint(
    pool: &ConnectionPool,
    business_id: Uuid,
    new_endpoint: &NewWebhookEndpoint,
) -> Result<WebhookEndpoint> {
    sqlx::query_as::<_, WebhookEndpoint>(
        r#"
        insert into webhooks (
            business_id,
            url,
            signing_secret,
            active
        )
        values ($1, $2, $3, true)
        returning id, business_id, url, signing_secret, active, created_at
        "#,
    )
    .bind(business_id)
    .bind(&new_endpoint.url)
    .bind(&new_endpoint.signing_secret)
    .fetch_one(pool)
    .await
    .context(QueryFailedSnafu)
}
