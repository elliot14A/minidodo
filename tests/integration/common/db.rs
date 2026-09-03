use super::constants::DATABASE_URL;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

pub async fn db() -> PgPool {
    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(10))
        .connect(DATABASE_URL)
        .await
        .expect("connect to postgres; is the compose stack up?")
}

pub async fn count_succeeded_attempts(pool: &PgPool, invoice_id: &str) -> i64 {
    let iid = Uuid::parse_str(invoice_id).expect("uuid");
    sqlx::query_scalar(
        "select count(*) from payment_attempts where invoice_id = $1 and status = 'succeeded'",
    )
    .bind(iid)
    .fetch_one(pool)
    .await
    .expect("count succeeded")
}

pub async fn count_attempts(pool: &PgPool, invoice_id: &str) -> i64 {
    let iid = Uuid::parse_str(invoice_id).expect("uuid");
    sqlx::query_scalar("select count(*) from payment_attempts where invoice_id = $1")
        .bind(iid)
        .fetch_one(pool)
        .await
        .expect("count attempts")
}

pub async fn latest_attempt_status(pool: &PgPool, invoice_id: &str) -> Option<String> {
    let iid = Uuid::parse_str(invoice_id).expect("uuid");
    sqlx::query_scalar(
        "select status::text from payment_attempts where invoice_id = $1 \
         order by created_at desc limit 1",
    )
    .bind(iid)
    .fetch_optional(pool)
    .await
    .expect("latest attempt")
}

pub async fn wait_for_delivery(
    pool: &PgPool,
    invoice_id: &str,
    event_type: &str,
    want: &str,
    timeout_secs: u64,
) -> bool {
    let deadline = std::time::Instant::now() + Duration::from_secs(timeout_secs);
    let iid = Uuid::parse_str(invoice_id).expect("uuid");
    loop {
        let status: Option<String> = sqlx::query_scalar(
            "select status::text from webhook_deliveries \
             where event_type::text = $1 and (payload->>'invoice_id')::uuid = $2 limit 1",
        )
        .bind(event_type)
        .bind(iid)
        .fetch_optional(pool)
        .await
        .expect("query delivery");
        if status.as_deref() == Some(want) {
            return true;
        }
        if std::time::Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}
