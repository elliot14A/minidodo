use minidodo_core::{MinidodoError, Result, SystemErrorCode};
use minidodo_infra::postgres::connection::ConnectionPool;
use sqlx::postgres::PgListener;
use uuid::Uuid;

pub async fn run_webhook_listener(pool: ConnectionPool) -> Result<()> {
    let mut listener = PgListener::connect_with(&pool)
        .await
        .map_err(|e| MinidodoError::Internal {
            message: format!("Failed to connect PgListener for webhooks: {}", e),
            code: SystemErrorCode::INTERNAL_ERROR,
        })?;

    listener
        .listen("webhooks")
        .await
        .map_err(|e| MinidodoError::Internal {
            message: format!("Failed to listen on channel 'webhooks': {}", e),
            code: SystemErrorCode::INTERNAL_ERROR,
        })?;

    tracing::info!("Worker notification listener active on channel 'webhooks'");

    loop {
        match listener.recv().await {
            Ok(notification) => {
                let payload = notification.payload();
                if let Ok(delivery_id) = Uuid::parse_str(payload) {
                    tracing::info!(delivery_id = %delivery_id, "Received webhook delivery notification");

                    if let Ok(Some((delivery, endpoint))) =
                        minidodo_infra::postgres::actions::webhooks::claim(&pool, delivery_id).await
                    {
                        let pool = pool.clone();
                        tokio::spawn(async move {
                            if let Err(e) = minidodo_infra::webhooks::deliver(&pool, delivery, endpoint).await {
                                tracing::error!(error = %e, delivery_id = %delivery_id, "Webhook deliverer task error");
                            }
                        });
                    }
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Listener error receiving webhook notification");
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }
}
