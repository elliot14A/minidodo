use minidodo_core::{MinidodoError, SystemErrorCode, WebhookErrorCode};
use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Failed to serialize webhook payload: {message}"))]
    SerializationFailed { message: String },

    #[snafu(display("Webhook client initialization error: {source}"))]
    ClientInitFailed { source: reqwest::Error },

    #[snafu(display("Webhook delivery HTTP request failed: {source}"))]
    RequestFailed { source: reqwest::Error },

    #[snafu(display("Webhook delivery exhausted max attempts: {last_error}"))]
    DeliveryExhausted { last_error: String },
}

impl From<Error> for MinidodoError {
    fn from(err: Error) -> Self {
        match err {
            Error::SerializationFailed { message } => MinidodoError::Internal {
                message,
                code: SystemErrorCode::INTERNAL_ERROR,
            },
            Error::ClientInitFailed { source } => MinidodoError::Internal {
                message: format!("Webhook HTTP client error: {}", source),
                code: SystemErrorCode::INTERNAL_ERROR,
            },
            Error::RequestFailed { source } => MinidodoError::ServiceUnavailable {
                message: format!("Webhook endpoint unreachable: {}", source),
                code: WebhookErrorCode::DELIVERY_FAILED,
            },
            Error::DeliveryExhausted { last_error } => MinidodoError::ServiceUnavailable {
                message: format!("Webhook delivery exhausted retries: {}", last_error),
                code: WebhookErrorCode::DELIVERY_FAILED,
            },
        }
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
