use minidodo_core::config::load_psp_config;
use minidodo_core::Result;
use minidodo_psp::serve;
use tracing::info;

pub async fn run() -> Result<()> {
    info!("Loading Mock PSP configuration");
    let config = load_psp_config()?;

    info!(
        address = %format!("{}:{}", config.psp.host, config.psp.port),
        "Starting Mock PSP service"
    );

    serve(config.psp.host, config.psp.port).await
}
