mod cli;
mod commands;

use clap::Parser;
use cli::{MinidodoCli, MinidodoCommands};
use commands::{migrate, psp, server};
use minidodo_core::Result;
use tracing::error;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    let cli = MinidodoCli::parse();

    let result = match cli.command.unwrap_or_default() {
        MinidodoCommands::Server => server::run().await,
        MinidodoCommands::Migrate => migrate::run().await,
        MinidodoCommands::Psp => psp::run().await,
    };

    if let Err(e) = &result {
        error!(error = %e, "Command execution failed");
        std::process::exit(1);
    }

    Ok(())
}
