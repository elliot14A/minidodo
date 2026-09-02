use axum::{
    extract::{rejection::JsonRejection, FromRequest, Request},
    Json,
};
use validator::Validate;

use crate::{MinidodoError, ValidationErrorCode};

#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: serde::de::DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = MinidodoError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(data) = Json::<T>::from_request(req, state).await.map_err(|err| {
            tracing::error!(error = %err, "failed to parse JSON request body");
            match err {
                JsonRejection::JsonDataError(_) => MinidodoError::BadRequest {
                    message: "request body contains an invalid or unsupported value".to_string(),
                    code: ValidationErrorCode::INVALID_FIELD,
                },
                _ => MinidodoError::BadRequest {
                    message: "invalid JSON format".to_string(),
                    code: ValidationErrorCode::INVALID_JSON,
                },
            }
        })?;

        data.validate().map_err(|err| {
            tracing::error!(error = %err, "request validation failed");
            MinidodoError::BadRequest {
                message: format!("validation error: {}", err),
                code: ValidationErrorCode::INVALID_PARAMETERS,
            }
        })?;

        Ok(ValidatedJson(data))
    }
}
