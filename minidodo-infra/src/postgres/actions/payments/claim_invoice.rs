use snafu::ResultExt;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::postgres::error::{QueryFailedSnafu, Result};

pub async fn claim(
    tx: &mut Transaction<'_, Postgres>,
    business_id: Uuid,
    invoice_id: Uuid,
) -> Result<u64> {
    let result = sqlx::query(
        r#"
        update invoices
        set state = 'processing'
        where id = $1 and business_id = $2 and state = 'open'
        "#,
    )
    .bind(invoice_id)
    .bind(business_id)
    .execute(&mut **tx)
    .await
    .context(QueryFailedSnafu)?;

    Ok(result.rows_affected())
}
