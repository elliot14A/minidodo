use minidodo_core::{Customer, Pagination, PaginationResult};
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
    pagination: Pagination,
) -> Result<PaginationResult<Customer>> {
    let mut data_builder = build_list_query(business_id, &pagination);
    let customers_fut = data_builder.build_query_as::<Customer>().fetch_all(pool);

    let mut count_builder = build_count_query(business_id);
    let count_fut = count_builder.build_query_scalar::<i64>().fetch_one(pool);

    let (customers, count_row) = try_join!(customers_fut, count_fut).context(QueryFailedSnafu)?;

    Ok(PaginationResult::new(customers, count_row as u32, &pagination))
}

fn build_list_query<'a>(
    business_id: Uuid,
    pagination: &Pagination,
) -> QueryBuilder<'a, sqlx::Postgres> {
    let mut qb = QueryBuilder::<sqlx::Postgres>::new(
        r#"
        select id, business_id, name, email, created_at
        from customers
        where business_id = 
        "#,
    );

    qb.push_bind(business_id);
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
) -> QueryBuilder<'a, sqlx::Postgres> {
    let mut qb = QueryBuilder::<sqlx::Postgres>::new(
        r#"
        select count(*)
        from customers
        where business_id = 
        "#,
    );

    qb.push_bind(business_id);
    qb
}
