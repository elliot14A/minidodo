use minidodo_core::{Customer, NewCustomer};
use snafu::ResultExt;
use uuid::Uuid;

use crate::postgres::connection::ConnectionPool;
use crate::postgres::error::{QueryFailedSnafu, Result};

pub async fn create(
    pool: &ConnectionPool,
    business_id: Uuid,
    new_customer: &NewCustomer,
) -> Result<Customer> {
    sqlx::query_as::<_, Customer>(
        r#"
        insert into customers (
            business_id,
            name,
            email
        )
        values ($1, $2, $3)
        returning id, business_id, name, email, created_at
        "#,
    )
    .bind(business_id)
    .bind(&new_customer.name)
    .bind(&new_customer.email)
    .fetch_one(pool)
    .await
    .context(QueryFailedSnafu)
}
