pub mod create;
pub mod get;
pub mod list;
pub mod routes;

pub use routes::routes;

#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        create::create,
        get::get,
        list::list,
    ),
    components(schemas(
        create::CreateCustomerRequest,
        crate::http::routes::v1::JsonResponse<minidodo_core::Customer>,
        crate::http::routes::v1::JsonResponse<minidodo_core::PaginationResult<minidodo_core::Customer>>,
        minidodo_core::Customer,
        minidodo_core::PaginationResult<minidodo_core::Customer>
    )),
    tags((name = "customers", description = "Customer management endpoints"))
)]
pub struct CustomersApiDoc;
