# Roadmap: minidodo (Resource-Driven Execution)

## Phases

### Phase 1: Workspace Foundation & Core Error Infrastructure
- **Goal**: Set up Cargo workspace (`minidodo-cli`, `minidodo-core`, `minidodo-infra`, `minidodo-server`), shared dependencies, PostgreSQL pool setup, `MinidodoError` with static error codes, `JsonResponse<T>`, `ValidatedJson<T>`, `Dockerfile`, and `docker-compose.yaml`.
- **Plans**:
  - `01-01`: Cargo workspace configuration, CLI harness (`server`, `migrate`), Dockerfile, and docker-compose.yaml.
  - `01-02`: `minidodo-core` error model (`MinidodoError`, `ErrorResponse`, `error_codes.rs`) implementing Axum `IntoResponse`.
  - `01-03`: `minidodo-infra` postgres connection pool, error handling (`impl From<PostgresError> for MinidodoError`), and migration runner.
  - `01-04`: `minidodo-server` HTTP server harness, `ValidatedJson<T>`, `TraceLayer`, and root `ApiDoc` setup.

### Phase 2: Business Entity (End-to-End Slice)
- **Goal**: Complete vertical slice for `Business`.
- **Plans**:
  - `02-01`: Migration `0001_create_businesses.sql` (lowercase SQL, UUID v4 primary key).
  - `02-02`: Domain model `Business`, `NewBusiness` in `minidodo-core` deriving `ToSchema`.
  - `02-03`: Postgres actions under `minidodo-infra/src/postgres/actions/businesses/{create.rs, get.rs, mod.rs}`.
  - `02-04`: API routes under `minidodo-server/src/http/routes/v1/businesses/{create.rs, get.rs, routes.rs, mod.rs}` with `utoipa` docs.

### Phase 3: API Key & Authentication (End-to-End Slice)
- **Goal**: Complete vertical slice for `ApiKey` and Axum authentication middleware.
- **Plans**:
  - `03-01`: Migration `0002_create_api_keys.sql` (SHA-256 `token_hash`, indexed `token_prefix`, `unique(token_hash)`).
  - `03-02`: Domain models `ApiKey`, `ApiKeyStatus`, `NewApiKey` in `minidodo-core`.
  - `03-03`: Postgres actions under `minidodo-infra/src/postgres/actions/apikeys/{create.rs, get_by_prefix.rs, revoke.rs, mod.rs}`.
  - `03-04`: Token generation utilities (`dodo_<prefix>_<secret>`) and Axum `AuthContext` extractor middleware in `minidodo-server`.
  - `03-05`: API routes under `minidodo-server/src/http/routes/v1/apikeys/{create.rs, list.rs, revoke.rs, routes.rs, mod.rs}` with `utoipa` docs.

### Phase 4: Customer Entity (End-to-End Slice)
- **Goal**: Complete vertical slice for `Customer`.
- **Plans**:
  - `04-01`: Migration `0003_create_customers.sql` (`idx(business_id, created_at desc)`).
  - `04-02`: Domain models `Customer`, `NewCustomer` in `minidodo-core`.
  - `04-03`: Postgres actions under `minidodo-infra/src/postgres/actions/customers/{create.rs, get.rs, list.rs, mod.rs}`.
  - `04-04`: API routes under `minidodo-server/src/http/routes/v1/customers/{create.rs, get.rs, list.rs, routes.rs, mod.rs}` with `utoipa` docs.

### Phase 5: Invoices & Line Items (End-to-End Slice)
- **Goal**: Complete vertical slice for `Invoice` and `LineItem` lifecycle.
- **Plans**:
  - `05-01`: Migration `0004_create_invoices_and_line_items.sql` (`total_cents bigint`, `state` check, `idx(business_id, state)`).
  - `05-02`: Domain models `Invoice`, `InvoiceState`, `LineItem`, `NewLineItem` with server-side integer cents sum logic.
  - `05-03`: Postgres actions under `minidodo-infra/src/postgres/actions/invoices/{create.rs, get.rs, list.rs, finalize.rs, void.rs, mod.rs}`.
  - `05-04`: API routes under `minidodo-server/src/http/routes/v1/invoices/{create.rs, get.rs, list.rs, finalize.rs, void.rs, routes.rs, mod.rs}` with `utoipa` docs.

### Phase 6: Mock PSP Service (Dedicated Phase)
- **Goal**: Implement `minidodo-psp` crate, standalone mock PSP HTTP server with deterministic token simulations, idempotency replaying, and wire into CLI `minidodo-cli psp`.
- **Plans**:
  - `06-01`: `minidodo-psp` crate setup, Axum router, and `minidodo-cli psp` subcommand.
  - `06-02`: Token matrix simulation (`tok_visa`, `tok_card_declined`, `tok_insufficient_funds`, `tok_timeout` 30s sleep, `tok_network_error` 500/drop).
  - `06-03`: PSP idempotency storage and response replay in `minidodo-psp`.

### Phase 7: Webhook Endpoints & Deliveries (End-to-End Slice)
- **Goal**: Webhook endpoint registration, delivery staging, HMAC-SHA256 signing, and API endpoints.
- **Plans**:
  - `07-01`: Migration `0005_create_webhooks.sql` (`webhook_endpoints`, `webhook_deliveries` with `locked_at` and retry tracking).
  - `07-02`: Domain models `WebhookEndpoint`, `WebhookDelivery`, `WebhookEvent`.
  - `07-03`: Postgres actions under `minidodo-infra/src/postgres/actions/webhooks/{create_endpoint.rs, stage_delivery.rs, claim_due.rs, update_delivery.rs, mod.rs}`.
  - `07-04`: Webhook signing helper (`minidodo-infra/src/webhook/signer.rs`) and pure delivery action (`minidodo-infra/src/webhook/deliver.rs`).
  - `07-05`: API routes for webhook endpoint registration (`minidodo-server/src/http/routes/v1/webhooks/`).

### Phase 8: Payment Processing, Concurrency & Crash Recovery (End-to-End Slice)
- **Goal**: Atomic payment claim (`POST /invoices/{id}/pay`), deterministic idempotency, and dedicated `minidodo-worker` crate.
- **Plans**:
  - `08-01`: Migration `0006_create_payments_and_idempotency.sql` (`payment_attempts`, `idempotency_keys` with `recovery_point` and `locked_at`).
  - `08-02`: Domain models `PaymentAttempt`, `IdempotencyRecord`, `PaymentRecoveryPoint`.
  - `08-03`: Postgres actions for atomic status-conditional claim (`update invoices set state='processing' where id=$1 and state='open'`), idempotency fingerprint validation, and recovery point updating.
  - `08-04`: Payment endpoint handler (`POST /invoices/{id}/pay`) returning non-blocking 202 Accepted.
  - `08-05`: `minidodo-worker` crate setup & `minidodo-cli worker` subcommand running payment completion sweep and webhook delivery sweep.

### Phase 9: Verification & Testing Suite
- **Goal**: Automated integration tests covering concurrency, idempotency, and failure modes.
- **Plans**:
  - `09-01`: Concurrency test for N simultaneous `POST /pay` requests on the same invoice.
  - `09-02`: Idempotency test verifying identical responses and zero duplicate PSP charges.
  - `09-03`: PSP failure test (`tok_timeout`, `tok_network_error`) asserting correct state recovery.

### Phase 10: Docker Compose & Operational Packaging
- **Goal**: Full stack `docker compose up` orchestration running Postgres, migrations, API server, mock PSP, and worker.
- **Plans**:
  - `10-01`: Finalize `docker-compose.yaml` with all services.
  - `10-02`: Export OpenAPI 3.1 `openapi.json` / `openapi.yaml`.
  - `10-03`: Final updates to `README.md`, `DESIGN.md`, and `AI_USAGE.md`.

---
*Roadmap updated: 2026-09-02*
