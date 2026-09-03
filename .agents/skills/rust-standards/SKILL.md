---
name: rust-standards
description: Generic Rust coding standards, workspace architecture, error handling patterns, figment config loading, and idioms derived from gaur practices.
---

# Rust Architecture & Coding Standards (Gaur Reference)

This skill defines the generic architectural layout, configuration management, error handling pattern, SQL/database practices, and idiomatic Rust standards for projects in this workspace.

---

## 1. Workspace Layout & Hexagonal Structure (Pure Functions)

We follow a **Hexagonal / Ports-and-Adapters Architecture using Pure Async Functions** (avoiding heavy trait objects or unnecessary abstraction layers):

```
├── Cargo.toml                  # Workspace root with workspace.dependencies & profile configs
├── <project>-core/             # Pure domain models, config loader (figment), master error enum, error codes, validation
│   └── src/
│       ├── lib.rs              # Re-exports domain and config modules
│       ├── config/             # Typed figment configuration loading (components & shared)
│       │   ├── mod.rs          # Exposes load_<component>_config()
│       │   ├── loader.rs       # Figment Env providers with section allowlists & key mapping
│       │   ├── shared/         # Shared configs (database.rs, etc.)
│       │   └── components/     # Component-specific configs (server.rs, worker.rs, etc.)
│       ├── error.rs            # Top-level %Project%Error enum implementing IntoResponse
│       ├── error_codes.rs      # Machine-readable error codes (&'static str constants)
│       └── models/             # Domain structs, value objects & state machines (derives ToSchema)
├── <project>-infra/            # Dedicated infra modules for each external service / subsystem
│   └── src/
│       ├── lib.rs
│       ├── postgres/           # Database adapter
│       │   ├── connection.rs   # sqlx PgPool setup, begin_transaction, run_migrations (takes &DatabaseConfig)
│       │   ├── error.rs        # Postgres-specific error enum -> impl From<Error> for %Project%Error
│       │   ├── migrations/     # SQL migration files (all lowercase SQL), co-located per-db subsystem (see §4)
│       │   └── actions/        # Action-based granular query files
│       │       ├── <resource_1>/
│       │       │   ├── create.rs
│       │       │   ├── get.rs
│       │       │   ├── list.rs
│       │       │   ├── update.rs
│       │       │   ├── delete.rs
│       │       │   └── mod.rs
│       │       └── <resource_2>/
│       ├── <external_service_1>/ # Dedicated infra adapter for external service 1
│       │   ├── error.rs        # Subsystem error enum -> impl From<Error> for %Project%Error
│       │   ├── <action_1>.rs   # Pure async function for action 1
│       │   └── mod.rs
│       └── <external_service_2>/ # Dedicated infra adapter for external service 2
│           ├── error.rs        # Subsystem error enum -> impl From<Error> for %Project%Error
│           ├── <action_1>.rs   # Pure async function for action 1
│           └── mod.rs
├── <project>-server/           # Axum HTTP API server
│   └── src/
│       ├── serve.rs            # Server builder, listener binding, graceful shutdown
│       ├── http/
│       │   ├── mod.rs          # Re-exports router, ApiDoc, JsonResponse
│       │   ├── middleware/     # AuthContext extractor (Bearer token), validation, request tracing
│       │   │   ├── auth.rs
│       │   │   ├── validator.rs
│       │   │   └── mod.rs
│       │   ├── routes/         # HTTP routes hierarchy
│       │   │   ├── mod.rs      # Mounts v1 router
│       │   │   └── v1/
│       │   │       ├── mod.rs  # Aggregates /v1 routes via .nest() & defines root ApiDoc
│       │   │       ├── <resource_1>/
│       │   │       │   ├── mod.rs    # Sub-ApiDoc definition & re-exports
│       │   │       │   ├── routes.rs # Router definition mounting handler routes
│       │   │       │   ├── create.rs # Individual action handler
│       │   │       │   ├── get.rs
│       │   │       │   └── list.rs
│       │   │       └── <resource_2>/
│       └── utils/              # Shared server helpers (crypto, tokens, ids, formatting)
└── <project>-cli/              # Unified CLI binary with clap subcommands (server, migrate, etc.)
    └── src/
        ├── main.rs
        ├── cli.rs
        └── commands/
            ├── mod.rs
            ├── server.rs
            └── migrate.rs
```

---

## 2. Configuration Management with `figment`

We use `figment` with `figment::providers::Env` for typed configuration loading in `<project>-core/src/config/`:

1. **Structured Layout**:
   - `shared/`: Configs shared across components (e.g. `DatabaseConfig`, `EncryptionConfig`).
   - `components/`: Specific target configs (e.g. `ServerConfig`, `WorkerConfig`).
   - `loader.rs`: Helper functions building a `Figment` instance using `Env::prefixed("<PREFIX>_")` and raw envs with custom key mapping (e.g. mapping `POSTGRES_HOST` to `DATABASE.HOST`) and section allowlist filtering.
   - `mod.rs`: Public loader functions (e.g. `load_server_config() -> Result<ServerConfig>`).
2. **Deny Unknown Fields**: Config structs derive `Deserialize` and use `#[serde(deny_unknown_fields)]` on nested sections to catch typos early.

---

## 3. Route & Module Conventions (Gaur Standard)

Every resource under `routes/v1/<resource>/` MUST strictly follow this 3-tier structure:

1. **`routes.rs`**: Dedicated file exposing `pub fn routes() -> Router` mounting action handlers:
   ```rust
   use axum::{Router, routing::{get, post, delete}};

   pub fn routes() -> Router {
       Router::new()
           .route("/", get(super::list::list).post(super::create::create))
           .route("/{id}", get(super::get::get).delete(super::delete::delete))
   }
   ```

2. **`mod.rs`**: Declares submodules, re-exports `pub use routes::routes;`, and defines the `utoipa::OpenApi` sub-documentation:
   ```rust
   pub mod create;
   pub mod get;
   pub mod list;
   pub mod routes;

   pub use routes::routes;

   #[derive(utoipa::OpenApi)]
   #[openapi(
       paths(create::create, get::get, list::list),
       components(schemas(crate::routes::v1::JsonResponse<...>, CreateResourceRequest)),
       tags((name = "resources", description = "Resource management endpoints"))
   )]
   pub struct ResourcesApiDoc;
   ```

3. **`v1/mod.rs`**:
   - Nests all resource routers (`.nest("/resources", resources::routes())`).
   - Defines the standard `JsonResponse<T>` response wrapper envelope (`success`, `with_message`).
   - Defines the root `ApiDoc` using `#[openapi(nest(...))]` and security schemes.

---

## 4. SQL Syntax & Action-Based Infra

1. **Strictly Lowercase SQL**: All SQL statements, keywords, and clauses must be written in **lowercase** (`select`, `from`, `where`, `update`, `set`, `insert into`, `values`, `for update skip locked`, `begin`, `commit`, `create table`, `alter table`).
2. **Explicit SQL over ORM DSL**: Use raw explicit SQL queries in action files (`sqlx::query!` / `sqlx::query_as!` or `sqlx::query`).
3. **Action-Based Organization**: Group queries under `postgres/actions/<entity>/<action>.rs` as pure async functions taking `&ConnectionPool` or `&mut PgTransaction`.
4. **Call actions by their full module path, do not import the function.** At the call site write the entire path, e.g. `minidodo_infra::postgres::actions::invoices::create(&pool, business_id, &new_invoice).await?`, rather than `use ...::create;` and calling `create(...)`. Generic verbs like `create`, `get`, `list`, `update` repeat across many entities; spelling out the path makes it obvious at a glance which entity and subsystem a call hits and avoids name collisions between same-named actions. It is a deliberate readability choice, not an oversight.
5. **Migrations live inside the subsystem, not at the workspace root.** SQL migrations are kept in `<project>-infra/src/postgres/migrations/` (co-located with the Postgres adapter), not in a single top-level `migrations/` directory. This is intentional: in projects that talk to several databases (Postgres plus ClickHouse, DuckDB, etc.), each database subsystem owns its own `migrations/` directory next to its adapter code. Keeping migrations beside the code that runs them makes each database independently maintainable and keeps a multi-database repo from collapsing all migrations into one ambiguous folder.
6. **Integer Financial & Quantity Types**: Use explicit integer types (`i64` / `u64` / `bigint`) for monetary values or countable items. Avoid floating-point arithmetic in critical domain math.

---

## 5. Dedicated Service Infra & Error Pattern

Every external service / subsystem (PostgreSQL, external HTTP APIs, third-party dispatchers, etc.) has its own dedicated directory in `<project>-infra` with:
- **Dedicated `error.rs`**: Captures internal/external failure modes using `snafu` or `thiserror`.
- **`impl From<ServiceError> for %Project%Error`**: Maps service-specific failures to top-level `AppError` with static error codes and HTTP statuses.
- **Pure Functions**: Export pure async functions taking connection pools/HTTP clients and parameters directly (no unnecessary trait boilerplate).
