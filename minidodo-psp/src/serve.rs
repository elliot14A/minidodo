use minidodo_core::Result;
use tracing::info;

pub async fn serve(host: String, port: u16) -> Result<()> {
    let (addr, server) = crate::http::server::build_http_server(host, port).await?;
    info!(address = %addr, "mock PSP server ready and listening");
    server.await
}
