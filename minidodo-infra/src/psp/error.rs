use minidodo_core::{MinidodoError, PaymentErrorCode, SystemErrorCode};
use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Failed to send request to PSP: {source}"))]
    RequestFailed { source: reqwest::Error },

    #[snafu(display("Failed to parse PSP JSON response: {source}"))]
    ParseFailed { source: reqwest::Error },

    #[snafu(display("PSP request timed out"))]
    Timeout,
}

impl From<Error> for MinidodoError {
    fn from(err: Error) -> Self {
        match err {
            Error::RequestFailed { .. } | Error::Timeout => MinidodoError::ServiceUnavailable {
                message: "Payment processor unavailable".to_string(),
                code: PaymentErrorCode::PSP_COMMUNICATION_FAILURE,
            },
            Error::ParseFailed { .. } => MinidodoError::Internal {
                message: "Failed to parse payment processor response".to_string(),
                code: SystemErrorCode::INTERNAL_ERROR,
            },
        }
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
