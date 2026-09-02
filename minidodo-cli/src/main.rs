mod cli;
mod commands;

use clap::Parser;
use cli::MinidodoCli;
use commands::execute_command;
use minidodo_core::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::from_filename("minidodo.env");
    let _ = dotenvy::dotenv();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = MinidodoCli::parse();
    let command = cli.command.unwrap_or_default();

    let result = execute_command(command).await;

    if let Err(ref err) = result {
        tracing::error!(error = %err, "Command execution failed");
    }

    result
}
