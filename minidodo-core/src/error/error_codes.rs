pub struct DatabaseErrorCode;
impl DatabaseErrorCode {
    pub const CONNECTION_FAILED: &'static str = "DB_CONNECTION_FAILED";
    pub const MIGRATION_FAILED: &'static str = "DB_MIGRATION_FAILED";
    pub const DUPLICATE_RECORD: &'static str = "DB_DUPLICATE_RECORD";
    pub const RECORD_NOT_FOUND: &'static str = "DB_RECORD_NOT_FOUND";
    pub const FOREIGN_KEY_VIOLATION: &'static str = "DB_FOREIGN_KEY_VIOLATION";
    pub const CHECK_CONSTRAINT_VIOLATION: &'static str = "DB_CHECK_CONSTRAINT_VIOLATION";
    pub const OPERATION_FAILED: &'static str = "DB_OPERATION_FAILED";
    pub const QUERY_EXECUTION_FAILED: &'static str = "DB_QUERY_EXECUTION_FAILED";
    pub const TRANSACTION_CONFLICT: &'static str = "DB_TRANSACTION_CONFLICT";
}

pub struct ValidationErrorCode;
impl ValidationErrorCode {
    pub const INVALID_FIELD: &'static str = "VALIDATION_INVALID_FIELD";
    pub const INVALID_JSON: &'static str = "VALIDATION_INVALID_JSON";
    pub const REQUIRED_FIELD_MISSING: &'static str = "VALIDATION_REQUIRED_FIELD_MISSING";
    pub const INVALID_PARAMETERS: &'static str = "VALIDATION_INVALID_PARAMETERS";
}

pub struct QueryErrorCode;
impl QueryErrorCode {
    pub const INVALID_PARAMETERS: &'static str = "QUERY_INVALID_PARAMETERS";
    pub const RESULT_TOO_LARGE: &'static str = "QUERY_RESULT_TOO_LARGE";
}

pub struct AuthErrorCode;
impl AuthErrorCode {
    pub const UNAUTHORIZED: &'static str = "AUTH_UNAUTHORIZED";
    pub const INVALID_KEY: &'static str = "AUTH_INVALID_KEY";
    pub const REVOKED_KEY: &'static str = "AUTH_REVOKED_KEY";
    pub const FORBIDDEN: &'static str = "AUTH_FORBIDDEN";
}

pub struct InvoiceErrorCode;
impl InvoiceErrorCode {
    pub const INVALID_STATE_TRANSITION: &'static str = "INVOICE_INVALID_STATE_TRANSITION";
}

pub struct PaymentErrorCode;
impl PaymentErrorCode {
    pub const INVOICE_NOT_PAYABLE: &'static str = "PAYMENT_INVOICE_NOT_PAYABLE";
    pub const CONCURRENT_PAYMENT_CONFLICT: &'static str = "PAYMENT_CONCURRENT_CONFLICT";
    pub const IDEMPOTENCY_KEY_CONFLICT: &'static str = "PAYMENT_IDEMPOTENCY_KEY_CONFLICT";
    pub const PSP_COMMUNICATION_FAILURE: &'static str = "PAYMENT_PSP_FAILURE";
}

pub struct SystemErrorCode;
impl SystemErrorCode {
    pub const INTERNAL_ERROR: &'static str = "SYSTEM_INTERNAL_ERROR";
    pub const SERVICE_UNAVAILABLE: &'static str = "SYSTEM_SERVICE_UNAVAILABLE";
    pub const CONFIG_ERROR: &'static str = "SYSTEM_CONFIG_ERROR";
}
