use minidodo_core::{Invoice, InvoiceWithLineItems, LineItem};
use snafu::ResultExt;
use uuid::Uuid;

use crate::postgres::connection::ConnectionPool;
use crate::postgres::error::{QueryFailedSnafu, Result};

pub async fn get_by_id(
    pool: &ConnectionPool,
    business_id: Uuid,
    id: Uuid,
) -> Result<Option<InvoiceWithLineItems>> {
    let invoice = sqlx::query_as::<_, Invoice>(
        r#"
        select id, business_id, customer_id, state, total_cents, due_date, created_at
        from invoices
        where business_id = $1 and id = $2
        "#,
    )
    .bind(business_id)
    .bind(id)
    .fetch_optional(pool)
    .await
    .context(QueryFailedSnafu)?;

    let Some(invoice) = invoice else {
        return Ok(None);
    };

    let line_items = sqlx::query_as::<_, LineItem>(
        r#"
        select id, invoice_id, description, quantity, unit_amount_cents, created_at
        from line_items
        where invoice_id = $1
        order by created_at asc
        "#,
    )
    .bind(invoice.id)
    .fetch_all(pool)
    .await
    .context(QueryFailedSnafu)?;

    Ok(Some(InvoiceWithLineItems { invoice, line_items }))
}
