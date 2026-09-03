use minidodo_core::{MinidodoError, Result, SystemErrorCode};
use minidodo_infra::postgres::connection::ConnectionPool;
use sqlx::postgres::PgListener;
use uuid::Uuid;

use crate::completer::complete_attempt;

pub async fn run_listener(
    pool: ConnectionPool,
    psp_base_url: String,
) -> Result<()> {
    let mut listener = PgListener::connect_with(&pool)
        .await
        .map_err(|e| MinidodoError::Internal {
            message: format!("Failed to connect PgListener: {}", e),
            code: SystemErrorCode::INTERNAL_ERROR,
        })?;

    listener
        .listen("payments")
        .await
        .map_err(|e| MinidodoError::Internal {
            message: format!("Failed to listen on channel 'payments': {}", e),
            code: SystemErrorCode::INTERNAL_ERROR,
        })?;

    tracing::info!("Worker notification listener active on channel 'payments'");

    loop {
        match listener.recv().await {
            Ok(notification) => {
                let payload = notification.payload();
                if let Ok(attempt_id) = Uuid::parse_str(payload) {
                    tracing::info!(attempt_id = %attempt_id, "Received payment notification");

                    if let Ok(Some(attempt)) =
                        minidodo_infra::postgres::actions::payments::get_by_id(&pool, attempt_id).await
                    {
                        if attempt.status != minidodo_core::PaymentStatus::Pending {
                            continue;
                        }

                        let claimed = minidodo_infra::postgres::actions::payments::claim_in_flight(
                            &pool,
                            attempt.business_id,
                            &attempt.idempotency_key,
                        )
                        .await
                        .unwrap_or(false);

                        if claimed {
                            let pool = pool.clone();
                            let psp_base_url = psp_base_url.clone();

                            tokio::spawn(async move {
                                if let Err(e) =
                                    complete_attempt(&pool, &psp_base_url, &attempt).await
                                {
                                    tracing::error!(error = %e, attempt_id = %attempt.id, "Failed to complete payment");
                                }
                            });
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Listener error receiving notification");
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }
}
