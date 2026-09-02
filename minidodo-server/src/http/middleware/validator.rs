use axum::{
    extract::{FromRequest, Request},
    Json,
};
use minidodo_core::{MinidodoError, ValidationErrorCode};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use validator::{Validate, ValidationErrors};

#[derive(Debug)]
pub struct ValidatedJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate + Send,
    S: Send + Sync,
{
    type Rejection = MinidodoError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(data) = Json::<T>::from_request(req, state).await.map_err(|err| {
            tracing::error!(error = %err, "failed to parse JSON request body");
            MinidodoError::BadRequest {
                message: "invalid JSON format".to_string(),
                code: ValidationErrorCode::INVALID_JSON,
            }
        })?;

        data.validate().map_err(|err| {
            tracing::error!(validation_errors = ?err, "request validation failed");
            format_validation_error(err)
        })?;

        Ok(ValidatedJson(data))
    }
}

fn format_validation_error(errors: ValidationErrors) -> MinidodoError {
    let mut field_errors: HashMap<String, Vec<String>> = HashMap::new();

    for (field, field_errors_list) in errors.field_errors() {
        let messages: Vec<String> = field_errors_list
            .iter()
            .filter_map(|e| e.message.as_ref().map(|m| m.to_string()))
            .collect();

        if !messages.is_empty() {
            field_errors.insert(field.to_string(), messages);
        }
    }

    let message = if field_errors.is_empty() {
        "validation failed".to_string()
    } else {
        let field_messages: Vec<String> = field_errors
            .iter()
            .map(|(field, msgs)| format!("{}: {}", field, msgs.join(", ")))
            .collect();
        format!("validation failed: {}", field_messages.join("; "))
    };

    MinidodoError::BadRequest {
        message,
        code: ValidationErrorCode::INVALID_FIELD,
    }
}
