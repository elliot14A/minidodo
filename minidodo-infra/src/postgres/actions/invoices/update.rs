use minidodo_core::{
    DatabaseErrorCode, Invoice, InvoiceErrorCode, InvoiceState, MinidodoError, Result,
    UpdateInvoiceStateTarget,
};
use snafu::ResultExt;
use uuid::Uuid;

use crate::postgres::connection::ConnectionPool;
use crate::postgres::error::QueryFailedSnafu;

pub async fn update_state(
    pool: &ConnectionPool,
    business_id: Uuid,
    id: Uuid,
    target_state: UpdateInvoiceStateTarget,
) -> Result<Invoice> {
    let (required_current_state, new_state) = match target_state {
        UpdateInvoiceStateTarget::Open => (InvoiceState::Draft, InvoiceState::Open),
        UpdateInvoiceStateTarget::Void => (InvoiceState::Open, InvoiceState::Void),
        UpdateInvoiceStateTarget::Uncollectible => {
            (InvoiceState::Open, InvoiceState::Uncollectible)
        }
    };

    let updated = sqlx::query_as::<_, Invoice>(
        r#"
        update invoices
        set state = $1
        where id = $2 and business_id = $3 and state = $4
        returning id, business_id, customer_id, state, total_cents, due_date, created_at
        "#,
    )
    .bind(new_state)
    .bind(id)
    .bind(business_id)
    .bind(required_current_state)
    .fetch_optional(pool)
    .await
    .context(QueryFailedSnafu)?;

    if let Some(invoice) = updated {
        return Ok(invoice);
    }

    let existing = sqlx::query_as::<_, Invoice>(
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

    match existing {
        None => Err(MinidodoError::NotFound {
            details: "Invoice not found".to_string(),
            code: DatabaseErrorCode::RECORD_NOT_FOUND,
        }),
        Some(invoice) => {
            let is_terminal = matches!(
                invoice.state,
                InvoiceState::Paid | InvoiceState::Void | InvoiceState::Uncollectible
            );
            let state_desc = if is_terminal {
                format!("terminal state '{}'", invoice.state)
            } else {
                format!("state '{}'", invoice.state)
            };

            Err(MinidodoError::Conflict {
                message: format!(
                    "cannot mark invoice as '{}': invoice is in {}",
                    target_state, state_desc
                ),
                code: InvoiceErrorCode::INVALID_STATE_TRANSITION,
            })
        }
    }
}
