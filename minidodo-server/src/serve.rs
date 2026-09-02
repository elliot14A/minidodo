use minidodo_core::Result;
use minidodo_infra::postgres::connection::ConnectionPool;

pub async fn serve(host: String, port: u16, pg_pool: ConnectionPool) -> Result<()> {
    let (addr, server) = crate::http::server::build_http_server(host, port, pg_pool).await?;
    tracing::info!(address = %addr, "server ready and listening");
    server.await
}
