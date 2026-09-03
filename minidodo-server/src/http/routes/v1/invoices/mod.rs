pub mod create;
pub mod get;
pub mod list;
pub mod pay;
pub mod routes;
pub mod update;

pub use routes::routes;

#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        create::create,
        get::get,
        list::list,
        update::update_state,
        pay::pay,
    ),
    components(schemas(
        create::CreateInvoiceRequest,
        create::CreateLineItemRequest,
        update::UpdateInvoiceStateRequest,
        pay::PayInvoiceRequest,
        pay::PayInvoiceResponse,
        minidodo_core::Invoice,
        minidodo_core::InvoiceState,
        minidodo_core::LineItem,
        minidodo_core::InvoiceWithLineItems,
        minidodo_core::UpdateInvoiceStateTarget,
        minidodo_core::PaginationResult<minidodo_core::Invoice>,
        crate::http::routes::v1::JsonResponse<minidodo_core::Invoice>,
        crate::http::routes::v1::JsonResponse<minidodo_core::InvoiceWithLineItems>,
        crate::http::routes::v1::JsonResponse<pay::PayInvoiceResponse>,
        crate::http::routes::v1::JsonResponse<minidodo_core::PaginationResult<minidodo_core::Invoice>>
    )),
    tags((name = "invoices", description = "Invoice & Line Item management endpoints"))
)]
pub struct InvoicesApiDoc;
