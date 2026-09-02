use minidodo_core::{Invoice, InvoiceState, Pagination, PaginationResult};
use snafu::ResultExt;
use sqlx::QueryBuilder;
use tokio::try_join;
use uuid::Uuid;

use crate::postgres::connection::ConnectionPool;
use crate::postgres::error::{QueryFailedSnafu, Result};

#[tracing::instrument(skip(pool, pagination))]
pub async fn list_by_business(
    pool: &ConnectionPool,
    business_id: Uuid,
    state_filter: Option<InvoiceState>,
    customer_filter: Option<Uuid>,
    pagination: Pagination,
) -> Result<PaginationResult<Invoice>> {
    let mut data_builder = build_list_query(business_id, state_filter, customer_filter, &pagination);
    let invoices_fut = data_builder.build_query_as::<Invoice>().fetch_all(pool);

    let mut count_builder = build_count_query(business_id, state_filter, customer_filter);
    let count_fut = count_builder.build_query_scalar::<i64>().fetch_one(pool);

    let (invoices, count_row) = try_join!(invoices_fut, count_fut).context(QueryFailedSnafu)?;

    Ok(PaginationResult::new(invoices, count_row as u32, &pagination))
}

fn build_list_query<'a>(
    business_id: Uuid,
    state_filter: Option<InvoiceState>,
    customer_filter: Option<Uuid>,
    pagination: &Pagination,
) -> QueryBuilder<'a, sqlx::Postgres> {
    let mut qb = QueryBuilder::<sqlx::Postgres>::new(
        r#"
        select id, business_id, customer_id, state, total_cents, due_date, created_at
        from invoices
        where business_id = 
        "#,
    );

    qb.push_bind(business_id);

    if let Some(state) = state_filter {
        qb.push(" and state = ");
        qb.push_bind(state);
    }

    if let Some(customer_id) = customer_filter {
        qb.push(" and customer_id = ");
        qb.push_bind(customer_id);
    }

    qb.push(" order by created_at ");
    qb.push(pagination.sort_order());
    qb.push(" limit ");
    qb.push_bind(pagination.limit() as i64);
    qb.push(" offset ");
    qb.push_bind(pagination.offset());

    qb
}

fn build_count_query<'a>(
    business_id: Uuid,
    state_filter: Option<InvoiceState>,
    customer_filter: Option<Uuid>,
) -> QueryBuilder<'a, sqlx::Postgres> {
    let mut qb = QueryBuilder::<sqlx::Postgres>::new(
        r#"
        select count(*)
        from invoices
        where business_id = 
        "#,
    );

    qb.push_bind(business_id);

    if let Some(state) = state_filter {
        qb.push(" and state = ");
        qb.push_bind(state);
    }

    if let Some(customer_id) = customer_filter {
        qb.push(" and customer_id = ");
        qb.push_bind(customer_id);
    }

    qb
}
