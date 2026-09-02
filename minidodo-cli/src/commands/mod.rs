pub mod migrate;
pub mod server;

use minidodo_core::Result;
use crate::cli::MinidodoCommands;

pub async fn execute_command(command: MinidodoCommands) -> Result<()> {
    match command {
        MinidodoCommands::Server => server::run().await,
        MinidodoCommands::Migrate => migrate::run().await,
    }
}
