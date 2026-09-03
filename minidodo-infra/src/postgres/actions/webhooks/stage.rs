use minidodo_core::models::webhook::WebhookEventType;
use serde_json::Value;
use snafu::ResultExt;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::postgres::error::{QueryFailedSnafu, Result};

pub async fn stage_deliveries(
    tx: &mut Transaction<'_, Postgres>,
    business_id: Uuid,
    event_type: WebhookEventType,
    payload: Value,
) -> Result<Vec<Uuid>> {
    let endpoints = sqlx::query_scalar::<_, Uuid>(
        r#"
        select id
        from webhooks
        where business_id = $1 and active = true
        "#,
    )
    .bind(business_id)
    .fetch_all(&mut **tx)
    .await
    .context(QueryFailedSnafu)?;

    let mut delivery_ids = Vec::with_capacity(endpoints.len());

    for endpoint_id in endpoints {
        let delivery_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            insert into webhook_deliveries (
                endpoint_id,
                business_id,
                event_type,
                payload,
                status,
                attempts
            )
            values ($1, $2, $3, $4, 'pending', 0)
            returning id
            "#,
        )
        .bind(endpoint_id)
        .bind(business_id)
        .bind(event_type)
        .bind(&payload)
        .fetch_one(&mut **tx)
        .await
        .context(QueryFailedSnafu)?;

        delivery_ids.push(delivery_id);
    }

    Ok(delivery_ids)
}
