use snafu::ResultExt;
use uuid::Uuid;

use crate::postgres::connection::ConnectionPool;
use crate::postgres::error::{QueryFailedSnafu, Result};

pub async fn verify_api_key(
    pool: &ConnectionPool,
    token_hash: &[u8],
) -> Result<Option<Uuid>> {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        select business_id
        from api_keys
        where token_hash = $1
        "#,
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await
    .context(QueryFailedSnafu)
}
