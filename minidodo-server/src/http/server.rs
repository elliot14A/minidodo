use axum::{extract::Request, Router, ServiceExt};
use minidodo_core::{MinidodoError, Result, SystemErrorCode};
use minidodo_infra::postgres::connection::ConnectionPool;
use std::{future::Future, pin::Pin};
use tower_http::{
    cors::{Any, CorsLayer},
    normalize_path::NormalizePathLayer,
    trace::TraceLayer,
};
use tower_layer::Layer;
use tracing::{error, info};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;

use crate::http;

pub type ServerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

pub async fn build_http_server(
    host: String,
    port: u16,
    pg_pool: ConnectionPool,
) -> Result<(String, ServerFuture)> {
    let openapi = http::routes::v1::ApiDoc::openapi();

    let app = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi))
        .merge(http::routes::routes())
        .layer(axum::Extension(pg_pool));

    let app = initialize_tracing(app);
    let app = app.layer(
        CorsLayer::new()
            .allow_origin(Any)
            .allow_headers(Any)
            .allow_methods(Any),
    );
    let app = NormalizePathLayer::trim_trailing_slash().layer(app);

    let addr = format!("{}:{}", host, port);
    info!(address = %addr, "binding listener");

    let listener = tokio::net::TcpListener::bind(&addr).await.map_err(|e| {
        error!("failed to bind to address {}: {}", addr, e);
        MinidodoError::Internal {
            message: "failed to bind to server address".to_string(),
            code: SystemErrorCode::INTERNAL_ERROR,
        }
    })?;

    let server_addr = addr.clone();
    let server = Box::pin(async move {
        axum::serve(
            listener,
            ServiceExt::<Request>::into_make_service_with_connect_info::<std::net::SocketAddr>(app),
        )
        .await
        .map_err(|e| {
            error!("server error on {}: {}", server_addr, e);
            MinidodoError::Internal {
                message: "server encountered an error".to_string(),
                code: SystemErrorCode::INTERNAL_ERROR,
            }
        })
    });

    Ok((addr, server))
}

fn initialize_tracing(app: Router) -> Router {
    app.layer(
        TraceLayer::new_for_http()
            .make_span_with(|request: &axum::http::Request<_>| {
                let req_id = Uuid::new_v4();
                tracing::info_span!(
                    "http_request",
                    request_id = %req_id,
                    method = %request.method(),
                    uri = %request.uri().to_string(),
                    version = ?request.version(),
                )
            })
            .on_request(|request: &axum::http::Request<_>, _span: &tracing::Span| {
                tracing::info!(
                    method = %request.method(),
                    uri = %request.uri().to_string(),
                    "request received"
                );
            })
            .on_response(
                |response: &axum::http::Response<_>,
                 latency: std::time::Duration,
                 _span: &tracing::Span| {
                    let status = response.status();
                    if status.is_server_error() {
                        tracing::error!(
                            status = %status,
                            latency_us = latency.as_micros(),
                            "request completed"
                        );
                    } else {
                        tracing::info!(
                            status = %status,
                            latency_us = latency.as_micros(),
                            "request completed"
                        );
                    }
                },
            ),
    )
}
