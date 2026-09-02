use minidodo_core::{load_server_config, Result};
use minidodo_infra::postgres::connection::establish_connection;
use tracing::info;

pub async fn run() -> Result<()> {
    let config = load_server_config()?;

    info!(address = %format!("{}:{}", config.server.host, config.server.port), "Connecting to database");
    let pool = establish_connection(&config.postgres).await?;

    info!("Starting API server");
    minidodo_server::serve(config.server.host, config.server.port, pool).await
}
