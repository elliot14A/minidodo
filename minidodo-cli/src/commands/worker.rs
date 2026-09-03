use minidodo_core::config::load_worker_config;
use minidodo_core::Result;
use tracing::info;

pub async fn run() -> Result<()> {
    info!("Loading worker configuration");
    let config = load_worker_config()?;

    info!(
        psp_base_url = %config.worker.psp_base_url,
        sweep_interval_secs = config.worker.sweep_interval_secs,
        "Starting worker service"
    );

    minidodo_worker::serve(config).await
}
