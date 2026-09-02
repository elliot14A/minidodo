use axum::{
    extract::{FromRequestParts, OptionalFromRequestParts},
    http::{header::AUTHORIZATION, request::Parts},
};
use minidodo_core::{AuthErrorCode, MinidodoError, Result};
use minidodo_infra::postgres::connection::ConnectionPool;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Authentication context extracted from Bearer API Key
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthContext {
    pub business_id: Uuid,
}

impl AuthContext {
    pub fn new(business_id: Uuid) -> Self {
        Self { business_id }
    }
}

impl<S> FromRequestParts<S> for AuthContext
where
    S: Send + Sync,
{
    type Rejection = MinidodoError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| {
                tracing::debug!("Authorization header missing");
                MinidodoError::Unauthorized {
                    message: "Missing Authorization header".to_string(),
                    code: AuthErrorCode::UNAUTHORIZED,
                }
            })?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                tracing::debug!("Authorization header is not a Bearer token");
                MinidodoError::Unauthorized {
                    message: "Invalid Authorization header format".to_string(),
                    code: AuthErrorCode::INVALID_KEY,
                }
            })?;

        let pool = parts.extensions.get::<ConnectionPool>().ok_or_else(|| {
            tracing::error!("ConnectionPool missing from request extensions");
            MinidodoError::Internal {
                message: "Internal server error".to_string(),
                code: minidodo_core::SystemErrorCode::INTERNAL_ERROR,
            }
        })?;

        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        let token_hash = hasher.finalize();

        let business_id = minidodo_infra::postgres::actions::apikeys::verify_api_key(pool, &token_hash)
            .await?
            .ok_or_else(|| {
                tracing::debug!("Invalid or non-existent API key");
                MinidodoError::Unauthorized {
                    message: "Invalid API key".to_string(),
                    code: AuthErrorCode::INVALID_KEY,
                }
            })?;

        Ok(AuthContext::new(business_id))
    }
}

impl<S> OptionalFromRequestParts<S> for AuthContext
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> std::result::Result<Option<Self>, Self::Rejection> {
        Ok(<AuthContext as FromRequestParts<S>>::from_request_parts(parts, state).await.ok())
    }
}
