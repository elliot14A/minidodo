use minidodo_core::PostgresConfig;
use snafu::ResultExt;
use sqlx::{PgPool, Postgres, Transaction, postgres::PgPoolOptions};
use tracing::{debug, info};

use crate::postgres::error::{Result, SqlxConnectionSnafu, SqlxMigrationsSnafu};

pub type ConnectionPool = sqlx::PgPool;
pub type PgTransaction<'a> = Transaction<'a, Postgres>;

#[tracing::instrument(skip(config), fields(
    db_host = %config.host,
    db_name = %config.database
))]
pub async fn establish_connection(config: &PostgresConfig) -> Result<ConnectionPool> {
    info!("establishing database connection pool");

    let connection_string = config.to_connection_string();
    let pool = PgPoolOptions::new()
        .max_connections(config.pool_size)
        .connect(&connection_string)
        .await
        .context(SqlxConnectionSnafu)?;

    info!("database connection pool established successfully");
    Ok(pool)
}

pub async fn begin_transaction(pool: &ConnectionPool) -> Result<PgTransaction<'_>> {
    pool.begin().await.context(SqlxConnectionSnafu)
}

#[tracing::instrument(skip(pool))]
pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    debug!("running postgres database migrations");
    sqlx::migrate!("./src/postgres/migrations")
        .run(pool)
        .await
        .context(SqlxMigrationsSnafu)?;

    info!("database migrations applied successfully");
    Ok(())
}
