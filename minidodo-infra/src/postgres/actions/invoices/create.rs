use minidodo_core::{
    DatabaseErrorCode, Invoice, InvoiceState, InvoiceWithLineItems, LineItem, MinidodoError,
    NewInvoice, Result,
};
use snafu::ResultExt;
use uuid::Uuid;

use crate::postgres::connection::ConnectionPool;
use crate::postgres::error::{QueryFailedSnafu, TransactionFailedSnafu};

pub async fn create(
    pool: &ConnectionPool,
    business_id: Uuid,
    new_invoice: &NewInvoice,
) -> Result<InvoiceWithLineItems> {
    let mut tx = pool.begin().await.context(TransactionFailedSnafu)?;

    let total_cents = new_invoice.calculate_total_cents()?;

    let invoice = sqlx::query_as::<_, Invoice>(
        r#"
        insert into invoices (
            business_id,
            customer_id,
            state,
            total_cents,
            due_date
        )
        select $1, $2, $3, $4, $5
        where exists (
            select 1 from customers where id = $2 and business_id = $1
        )
        returning id, business_id, customer_id, state, total_cents, due_date, created_at
        "#,
    )
    .bind(business_id)
    .bind(new_invoice.customer_id)
    .bind(InvoiceState::Draft)
    .bind(total_cents)
    .bind(new_invoice.due_date)
    .fetch_optional(&mut *tx)
    .await
    .context(QueryFailedSnafu)?;

    let Some(invoice) = invoice else {
        return Err(MinidodoError::NotFound {
            details: "Customer not found".to_string(),
            code: DatabaseErrorCode::RECORD_NOT_FOUND,
        });
    };

    let mut line_items = Vec::with_capacity(new_invoice.line_items.len());

    for item in &new_invoice.line_items {
        let line_item = sqlx::query_as::<_, LineItem>(
            r#"
            insert into line_items (
                invoice_id,
                description,
                quantity,
                unit_amount_cents
            )
            values ($1, $2, $3, $4)
            returning id, invoice_id, description, quantity, unit_amount_cents, created_at
            "#,
        )
        .bind(invoice.id)
        .bind(&item.description)
        .bind(item.quantity)
        .bind(item.unit_amount_cents)
        .fetch_one(&mut *tx)
        .await
        .context(QueryFailedSnafu)?;

        line_items.push(line_item);
    }

    tx.commit().await.context(TransactionFailedSnafu)?;

    Ok(InvoiceWithLineItems { invoice, line_items })
}
