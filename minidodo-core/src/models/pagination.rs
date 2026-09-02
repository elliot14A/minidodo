use std::collections::HashMap;

use axum::{
    extract::{FromRequestParts, Query},
    http::request::Parts,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{MinidodoError, ValidationErrorCode};

/// Struct representing paginated result for any entity
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct PaginationResult<T>
where
    T: Serialize,
{
    pub items: Vec<T>,
    pub total_items: u32,
    pub page: u32,
    pub limit: u32,
    pub total_pages: u32,
    pub has_next_page: bool,
    pub has_prev_page: bool,
}

impl<T> PaginationResult<T>
where
    T: Serialize,
{
    /// Create a new PaginationResult
    pub fn new(items: Vec<T>, total_items: u32, pagination: &Pagination) -> Self {
        let total_pages = pagination.total_pages(total_items);
        let has_next_page = pagination.has_next_page(total_items);
        let has_prev_page = pagination.has_prev_page();
        Self {
            items,
            total_items,
            page: pagination.page(),
            limit: pagination.limit(),
            total_pages,
            has_next_page,
            has_prev_page,
        }
    }
}

/// Pagination struct for handling page-based pagination
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pagination {
    pub page: u32,
    pub limit: u32,
    pub sort_order: String,
    pub query_params: Option<HashMap<String, String>>,
}

impl Default for Pagination {
    fn default() -> Self {
        Self::new(
            Self::DEFAULT_PAGE,
            Self::DEFAULT_LIMIT,
            Self::DEFAULT_SORT_ORDER.to_string(),
            None,
        )
    }
}

impl Pagination {
    pub const DEFAULT_PAGE: u32 = 1;
    pub const DEFAULT_LIMIT: u32 = 10;
    pub const MAX_LIMIT: u32 = 100;
    pub const DEFAULT_SORT_ORDER: &'static str = "asc";

    pub fn new(
        page: u32,
        limit: u32,
        sort_order: String,
        query_params: Option<HashMap<String, String>>,
    ) -> Self {
        Self {
            page: page.max(1),
            limit: limit.clamp(1, Self::MAX_LIMIT),
            sort_order,
            query_params,
        }
    }

    pub fn offset(&self) -> i64 {
        (i64::from(self.page) - 1) * i64::from(self.limit)
    }

    pub fn page(&self) -> u32 {
        self.page
    }

    pub fn limit(&self) -> u32 {
        self.limit
    }

    pub fn sort_order(&self) -> &str {
        match self.sort_order.as_str() {
            "asc" => "asc",
            "desc" => "desc",
            _ => Self::DEFAULT_SORT_ORDER,
        }
    }

    pub fn query_params(&self) -> Option<&HashMap<String, String>> {
        self.query_params.as_ref()
    }

    pub fn total_pages(&self, total_items: u32) -> u32 {
        if total_items == 0 {
            return 0;
        }
        total_items.div_ceil(self.limit)
    }

    pub fn has_next_page(&self, total_items: u32) -> bool {
        self.page < self.total_pages(total_items)
    }

    pub fn has_prev_page(&self) -> bool {
        self.page > 1
    }
}

impl<S> FromRequestParts<S> for Pagination
where
    S: Send + Sync,
{
    type Rejection = MinidodoError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(params) = Query::<HashMap<String, String>>::from_request_parts(parts, state)
            .await
            .map_err(|e| MinidodoError::BadRequest {
                message: format!("Invalid query parameters: {}", e),
                code: ValidationErrorCode::INVALID_FIELD,
            })?;

        let page = params
            .get("page")
            .and_then(|p| p.parse::<u32>().ok())
            .unwrap_or(Pagination::DEFAULT_PAGE);

        let limit = params
            .get("limit")
            .and_then(|l| l.parse::<u32>().ok())
            .unwrap_or(Pagination::DEFAULT_LIMIT);

        let sort_order = params
            .get("sort_order")
            .cloned()
            .unwrap_or_else(|| Pagination::DEFAULT_SORT_ORDER.to_string());

        Ok(Pagination::new(page, limit, sort_order, Some(params)))
    }
}
