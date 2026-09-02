pub mod get;
pub mod routes;

pub use routes::routes;

#[derive(utoipa::OpenApi)]
#[openapi(
    paths(get::get),
    components(schemas(
        crate::http::routes::v1::JsonResponse<minidodo_core::Business>,
        minidodo_core::Business
    )),
    tags((name = "businesses", description = "Business entity endpoints"))
)]
pub struct BusinessesApiDoc;
