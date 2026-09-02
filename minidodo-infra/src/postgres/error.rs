use minidodo_core::{DatabaseErrorCode, MinidodoError, QueryErrorCode};
use snafu::Snafu;
use sqlx::migrate::MigrateError;
use tracing::error;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("could not get connection from sqlx pool: {}", source))]
    SqlxConnection { source: sqlx::Error },

    #[snafu(display("could not run database migrations: {}", source))]
    SqlxMigrations { source: MigrateError },

    #[snafu(display("database query failed: {}", source))]
    QueryFailed { source: sqlx::Error },

    #[snafu(display("database transaction failed: {}", source))]
    TransactionFailed { source: sqlx::Error },

    #[snafu(display("invalid input: {}", details))]
    InvalidInput { details: String },
}

impl From<Error> for MinidodoError {
    fn from(err: Error) -> Self {
        match err {
            Error::SqlxConnection { source } => {
                error!(error = %source, "database connection failed");
                MinidodoError::DatabaseConnection {
                    message: "failed to establish database connection".to_string(),
                    code: DatabaseErrorCode::CONNECTION_FAILED,
                }
            }
            Error::SqlxMigrations { source } => {
                error!(error = %source, "database migrations failed");
                MinidodoError::DatabaseConnection {
                    message: format!("failed to run database migrations: {}", source),
                    code: DatabaseErrorCode::MIGRATION_FAILED,
                }
            }
            Error::TransactionFailed { source } => {
                error!(error = %source, "database transaction failed");
                MinidodoError::DatabaseError {
                    message: "transaction failed due to a database error".to_string(),
                    code: DatabaseErrorCode::OPERATION_FAILED,
                }
            }
            Error::InvalidInput { details } => MinidodoError::BadRequest {
                message: details,
                code: QueryErrorCode::INVALID_PARAMETERS,
            },
            Error::QueryFailed { source } => {
                use sqlx::Error as SqlxError;
                match source {
                    SqlxError::RowNotFound => MinidodoError::NotFound {
                        details: "requested record was not found".to_string(),
                        code: DatabaseErrorCode::RECORD_NOT_FOUND,
                    },
                    SqlxError::Database(ref db_err) => {
                        let code = db_err.code().unwrap_or("".into());
                        let msg = db_err.message();

                        error!(error_code = %code, error_message = %msg, "database query failed");

                        match code.as_ref() {
                            // unique violation
                            "23505" => MinidodoError::Duplicate {
                                details: "a record with this unique value already exists".to_string(),
                                code: DatabaseErrorCode::DUPLICATE_RECORD,
                            },
                            // foreign key violation
                            "23503" => MinidodoError::ConstraintViolation {
                                resource: "foreign key".to_string(),
                                details: "referenced record does not exist".to_string(),
                                code: DatabaseErrorCode::FOREIGN_KEY_VIOLATION,
                            },
                            // not null violation
                            "23502" => MinidodoError::ConstraintViolation {
                                resource: "required field".to_string(),
                                details: "a required field is missing".to_string(),
                                code: DatabaseErrorCode::CHECK_CONSTRAINT_VIOLATION,
                            },
                            // check constraint violation
                            "23514" => MinidodoError::ConstraintViolation {
                                resource: "check constraint".to_string(),
                                details: "the provided value violates a data constraint".to_string(),
                                code: DatabaseErrorCode::CHECK_CONSTRAINT_VIOLATION,
                            },
                            // serialization failure / deadlock
                            "40001" => {
                                error!("transaction serialization conflict");
                                MinidodoError::DatabaseError {
                                    message: "database operation failed due to concurrent access, please retry".to_string(),
                                    code: DatabaseErrorCode::TRANSACTION_CONFLICT,
                                }
                            }
                            _ => MinidodoError::DatabaseError {
                                message: "a database error occurred".to_string(),
                                code: DatabaseErrorCode::OPERATION_FAILED,
                            },
                        }
                    }
                    SqlxError::Io(ref io_err) => {
                        error!(io_error = %io_err, "database i/o error");
                        MinidodoError::DatabaseConnection {
                            message: "failed to communicate with database".to_string(),
                            code: DatabaseErrorCode::CONNECTION_FAILED,
                        }
                    }
                    _ => {
                        error!(error = %source, "database query execution failed");
                        MinidodoError::DatabaseError {
                            message: "database query execution failed".to_string(),
                            code: DatabaseErrorCode::QUERY_EXECUTION_FAILED,
                        }
                    }
                }
            }
        }
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
