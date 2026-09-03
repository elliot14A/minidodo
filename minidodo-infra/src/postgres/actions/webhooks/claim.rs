use minidodo_core::models::webhook::{WebhookDelivery, WebhookEndpoint};
use snafu::ResultExt;
use uuid::Uuid;

use crate::postgres::connection::ConnectionPool;
use crate::postgres::error::{QueryFailedSnafu, Result};

pub async fn claim(
    pool: &ConnectionPool,
    delivery_id: Uuid,
) -> Result<Option<(WebhookDelivery, WebhookEndpoint)>> {
    let row = sqlx::query_as::<
        _,
        (
            Uuid,
            Uuid,
            Uuid,
            minidodo_core::models::webhook::WebhookEventType,
            serde_json::Value,
            minidodo_core::models::webhook::WebhookDeliveryStatus,
            i32,
            Option<String>,
            Option<chrono::DateTime<chrono::Utc>>,
            chrono::DateTime<chrono::Utc>,
            Uuid,
            Uuid,
            String,
            String,
            bool,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        r#"
        with locked as (
            select
                id,
                endpoint_id,
                business_id,
                event_type,
                payload,
                status,
                attempts,
                last_error,
                last_attempt_at,
                created_at
            from webhook_deliveries
            where id = $1 and status = 'pending'
            for update skip locked
        )
        select
            l.id,
            l.endpoint_id,
            l.business_id,
            l.event_type,
            l.payload,
            l.status,
            l.attempts,
            l.last_error,
            l.last_attempt_at,
            l.created_at,
            w.id,
            w.business_id,
            w.url,
            w.signing_secret,
            w.active,
            w.created_at
        from locked l
        join webhooks w on l.endpoint_id = w.id
        "#,
    )
    .bind(delivery_id)
    .fetch_optional(pool)
    .await
    .context(QueryFailedSnafu)?;

    let Some(r) = row else {
        return Ok(None);
    };

    let delivery = WebhookDelivery {
        id: r.0,
        endpoint_id: r.1,
        business_id: r.2,
        event_type: r.3,
        payload: r.4,
        status: r.5,
        attempts: r.6,
        last_error: r.7,
        last_attempt_at: r.8,
        created_at: r.9,
    };

    let endpoint = WebhookEndpoint {
        id: r.10,
        business_id: r.11,
        url: r.12,
        signing_secret: r.13,
        active: r.14,
        created_at: r.15,
    };

    Ok(Some((delivery, endpoint)))
}
