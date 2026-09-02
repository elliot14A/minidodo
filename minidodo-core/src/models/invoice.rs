use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::models::line_item::{LineItem, NewLineItem};
use crate::{MinidodoError, Result, ValidationErrorCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "invoice_state", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum InvoiceState {
    Draft,
    Open,
    Processing,
    Paid,
    Void,
    Uncollectible,
}

impl std::fmt::Display for InvoiceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Draft => write!(f, "draft"),
            Self::Open => write!(f, "open"),
            Self::Processing => write!(f, "processing"),
            Self::Paid => write!(f, "paid"),
            Self::Void => write!(f, "void"),
            Self::Uncollectible => write!(f, "uncollectible"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct Invoice {
    pub id: Uuid,
    pub business_id: Uuid,
    pub customer_id: Uuid,
    pub state: InvoiceState,
    pub total_cents: i64,
    pub due_date: NaiveDate,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InvoiceWithLineItems {
    #[serde(flatten)]
    pub invoice: Invoice,
    pub line_items: Vec<LineItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewInvoice {
    pub customer_id: Uuid,
    pub due_date: NaiveDate,
    pub line_items: Vec<NewLineItem>,
}

impl NewInvoice {
    pub fn calculate_total_cents(&self) -> Result<i64> {
        let mut total: i64 = 0;
        for item in &self.line_items {
            let line_total = i64::from(item.quantity)
                .checked_mul(item.unit_amount_cents)
                .and_then(|line| total.checked_add(line))
                .ok_or_else(|| MinidodoError::BadRequest {
                    message: "Invoice total exceeds the maximum supported amount".to_string(),
                    code: ValidationErrorCode::INVALID_FIELD,
                })?;
            total = line_total;
        }
        Ok(total)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum UpdateInvoiceStateTarget {
    Open,
    Void,
    Uncollectible,
}

impl std::fmt::Display for UpdateInvoiceStateTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open => write!(f, "open"),
            Self::Void => write!(f, "void"),
            Self::Uncollectible => write!(f, "uncollectible"),
        }
    }
}
