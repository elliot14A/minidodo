use minidodo_core::Result;
use minidodo_psp::serve;
use tracing::info;

pub async fn run() -> Result<()> {
    info!("Starting Mock PSP service");
    serve().await
}
