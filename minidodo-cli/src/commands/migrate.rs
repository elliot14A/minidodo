use minidodo_core::{load_database_config, Result};
use minidodo_infra::postgres::connection::{establish_connection, run_migrations};
use tracing::info;

pub async fn run() -> Result<()> {
    let config = load_database_config()?;

    info!("Connecting to database for migrations");
    let pool = establish_connection(&config).await?;

    info!("Running PostgreSQL migrations");
    run_migrations(&pool).await?;

    info!("Migrations completed successfully");
    Ok(())
}
