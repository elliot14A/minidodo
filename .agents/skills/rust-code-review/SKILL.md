---
name: rust-code-review
description: Checklist and review guidelines for Rust code, ensuring compliance with gaur standards, error handling, correctness, and security.
---

# Rust Code Review Checklist (Gaur Standards)

Use this checklist to perform rigorous reviews on any new or modified Rust code in the workspace.

---

## 1. Route & Server Architecture (Gaur 3-Tier Pattern)
- [ ] **`routes.rs` File Present**: Every resource directory under `routes/v1/<resource>/` contains a dedicated `routes.rs` defining `pub fn routes() -> Router`.
- [ ] **`mod.rs` OpenApi & Re-export**: `routes/v1/<resource>/mod.rs` re-exports `pub use routes::routes;` and defines the `utoipa::OpenApi` sub-doc for that resource.
- [ ] **Standard Response Envelope**: Handlers return `JsonResponse<T>::success(...)` or `JsonResponse::with_message(...)`.
- [ ] **Request Validation**: Incoming JSON payloads derive `validator::Validate` and use `ValidatedJson(body)` extractor.

---

## 2. Error Handling & Subsystem Separation
- [ ] **No `unwrap()` or `expect()` in production paths**: All recoverable errors must be propagated using `?` or explicit match expressions.
- [ ] **Dedicated Subsystem Errors**: Each external service or infra adapter (e.g. `postgres`, external HTTP clients) has its own scoped `error.rs` file.
- [ ] **`From` Trait Implementation**: Each subsystem error implements `From<SubsystemError> for AppError`, translating lower-level codes/rejections to domain error variants.
- [ ] **Error Sanitization & Safe Messages**: Low-level errors are logged via `tracing::error!` with full details, but mapped to safe, sanitized user-facing messages in the API response.
- [ ] **Static Error Codes**: Every domain error variant maps to a static machine-readable string code (e.g. `DatabaseErrorCode::RECORD_NOT_FOUND`).

---

## 3. SQL & Database Practices
- [ ] **Lowercase SQL**: All SQL statements and keywords are written in lowercase (`select`, `insert into`, `update`, `set`, `where`, `join`, `order by`, etc.).
- [ ] **Action-Based Organization**: Queries are structured as pure async functions in granular action files (`postgres/actions/<entity>/<action>.rs`).
- [ ] **Explicit Queries**: Raw SQL is used directly via sqlx rather than dynamic/opaque query builders.
- [ ] **No Long Locks across Network I/O**: Open database transactions or row-level locks must never be held across external HTTP or I/O boundaries.

---

## 4. Architecture, Functional Style & OpenAPI
- [ ] **Pure Functions over Heavy Traits**: Infra adapters and actions use pure async functions taking connection pools, clients, and inputs directly instead of trait object boilerplate unless polymorphism is strictly required.
- [ ] **utoipa OpenAPI Annotations**:
  - Request/response and domain model structs derive `utoipa::ToSchema`.
  - Handlers are annotated with `#[utoipa::path(...)]` with all request/response schemas documented.
  - Sub-ApiDocs are nested in `v1/mod.rs` `ApiDoc`.

---

## 5. Security & Tenant Scoping
- [ ] **Secret / Token Storage**: Tokens and secrets are never stored in plaintext or logged. Hashed storage (e.g. SHA-256) is used with prefix lookups.
- [ ] **Tenant Scoping**: All resource queries filter strictly by the authenticated tenant identifier (`workspace_id` / `business_id`) extracted from context.

---

## 6. Observability & Lints
- [ ] **Tracing Instrumentation**: Public and async functions use `#[tracing::instrument]`, skipping bulky/sensitive structs and capturing important identifiers.
- [ ] **Lint & Format Compliance**: Code is formatted with `rustfmt` and passes `cargo clippy --all-targets -- -D warnings`.
