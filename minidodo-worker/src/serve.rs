use minidodo_core::config::WorkerConfig;
use minidodo_core::Result;
use minidodo_infra::postgres::connection::establish_connection;
use tracing::info;

use crate::listener::run_listener;
use crate::sweep::run_sweep;
use crate::webhook_listener::run_webhook_listener;

pub async fn serve(config: WorkerConfig) -> Result<()> {
    info!("Connecting to database for worker");
    let pool = establish_connection(&config.postgres).await?;

    let psp_base_url = config.worker.psp_base_url;
    let sweep_interval = config.worker.sweep_interval_secs;

    info!(
        psp_base_url = %psp_base_url,
        sweep_interval_secs = sweep_interval,
        "Starting minidodo worker"
    );

    let payment_listener_handle = tokio::spawn(run_listener(
        pool.clone(),
        psp_base_url.clone(),
    ));

    let payment_sweep_handle = tokio::spawn(run_sweep(
        pool.clone(),
        psp_base_url.clone(),
        sweep_interval,
    ));

    let webhook_listener_handle = tokio::spawn(run_webhook_listener(
        pool.clone(),
    ));

    tokio::select! {
        res = payment_listener_handle => {
            tracing::error!("Payment notification listener exited: {:?}", res);
        }
        res = payment_sweep_handle => {
            tracing::error!("Payment recovery sweep exited: {:?}", res);
        }
        res = webhook_listener_handle => {
            tracing::error!("Webhook notification listener exited: {:?}", res);
        }
    }

    Ok(())
}
