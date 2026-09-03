use minidodo_core::config::load_psp_config;
use minidodo_core::Result;
use tracing::info;

use crate::http::state::PspState;

pub async fn serve() -> Result<()> {
    let config = load_psp_config()?;
    let state = PspState {
        webhook_signing_secret: config.psp.webhook_signing_secret,
    };
    let (addr, server) = crate::http::server::build_http_server(config.psp.host, config.psp.port, state).await?;
    info!(address = %addr, "mock PSP server ready and listening");
    server.await
}
