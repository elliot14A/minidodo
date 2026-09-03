use minidodo_core::Result;
use minidodo_infra::postgres::connection::ConnectionPool;
use std::time::Duration;

use crate::completer::complete_attempt;

pub async fn run_sweep(
    pool: ConnectionPool,
    psp_base_url: String,
    interval_secs: u64,
) -> Result<()> {
    tracing::info!(interval_secs = interval_secs, "Worker recovery sweep started");

    loop {
        match minidodo_infra::postgres::actions::payments::find_stale(&pool).await {
            Ok(stale_attempts) => {
                for attempt in stale_attempts {
                    let reclaimed = minidodo_infra::postgres::actions::payments::reclaim_stale(
                        &pool,
                        attempt.business_id,
                        &attempt.idempotency_key,
                    )
                    .await
                    .unwrap_or(false);

                    if reclaimed {
                        tracing::info!(attempt_id = %attempt.id, "Reclaimed stale in-flight payment");
                        let pool = pool.clone();
                        let psp_base_url = psp_base_url.clone();

                        tokio::spawn(async move {
                            if let Err(e) =
                                complete_attempt(&pool, &psp_base_url, &attempt).await
                            {
                                tracing::error!(error = %e, attempt_id = %attempt.id, "Failed to complete reclaimed attempt");
                            }
                        });
                    }
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Recovery sweep failed to query stale keys");
            }
        }

        tokio::time::sleep(Duration::from_secs(interval_secs)).await;
    }
}
