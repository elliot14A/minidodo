# Roadmap: minidodo (Resource-Driven Execution)

## Phases

### Phase 1: Workspace Foundation & Core Error Infrastructure (Complete)
- **Goal**: Set up Cargo workspace (`minidodo-cli`, `minidodo-core`, `minidodo-infra`, `minidodo-server`), shared dependencies, PostgreSQL pool setup, `MinidodoError` with static error codes, `JsonResponse<T>`, `ValidatedJson<T>`, `Dockerfile`, and `docker-compose.yaml`.
- **Plans**:
  - `01-01`: Cargo workspace configuration, CLI harness (`server`, `migrate`), Dockerfile, and docker-compose.yaml.
  - `01-02`: `minidodo-core` error model (`MinidodoError`, `ErrorResponse`, `error_codes.rs`) implementing Axum `IntoResponse`.
  - `01-03`: `minidodo-infra` postgres connection pool, error handling (`impl From<PostgresError> for MinidodoError`), and migration runner.
  - `01-04`: `minidodo-server` HTTP server harness, `ValidatedJson<T>`, `TraceLayer`, and root `ApiDoc` setup.

### Phase 2: Business Entity & API Key Auth (Complete)
- **Goal**: Complete vertical slice for `Business` and deterministic API key authentication with seeding.
- **Plans**:
  - `02-01`: Migrations for `businesses` and `api_keys` with default business and API key seed.
  - `02-02`: Domain model `Business`, `AuthContext` in `minidodo-core` and database actions in `minidodo-infra`.
  - `02-03`: Axum `AuthContext` extractor middleware, business API route, and OpenAPI docs.

### Phase 3: Customer Entity (Complete)
- **Goal**: Complete vertical slice for `Customer`.
- **Plans**:
  - `03-01`: Migration `0003_create_customers.sql` (`idx(business_id, created_at desc)`).
  - `03-02`: Domain models `Customer`, `NewCustomer` in `minidodo-core`.
  - `03-03`: Postgres actions under `minidodo-infra/src/postgres/actions/customers/`.
  - `03-04`: API routes under `minidodo-server/src/http/routes/v1/customers/` with `utoipa` docs.

### Phase 4: Invoices & Line Items (Complete)
- **Goal**: Complete vertical slice for `Invoice` and `LineItem` lifecycle.
- **Plans**:
  - `04-01`: Migrations `0004_create_invoices` and `0005_create_line_items` (`total_cents bigint`, state enum, `idx(business_id, state)`).
  - `04-02`: Domain models `Invoice`, `InvoiceState`, `LineItem`, `NewLineItem` with server-side integer cents sum, and status-conditional transition actions.
  - `04-03`: API routes under `minidodo-server/src/http/routes/v1/invoices/` (`POST`, `GET`, `GET` list, `PATCH` guarded transition) with `utoipa` docs.

### Phase 5: Mock PSP Service (Complete)
- **Goal**: Implement `minidodo-psp` crate, a standalone mock PSP HTTP server with deterministic token simulations, wired into CLI `minidodo psp`.
- **Plans**:
  - `05-01`: `minidodo-psp` crate setup, Axum router, `worker`-style config, and `minidodo psp` subcommand.
  - `05-02`: Token matrix simulation (`tok_success`, `tok_card_declined`, `tok_insufficient_funds`, `tok_timeout` 30s slow success, `tok_network_error` 500).
  - `05-03`: Docker compose `psp` service and multi-token verification.

### Phase 6: Payment Processing & Worker (Completer) (Planned)
- **Goal**: Payment path end to end. `payment_attempts` and `idempotency_keys` tables, `POST /v1/invoices/{id}/pay` returning `202`, and the `minidodo-worker` crate running a LISTEN/NOTIFY driven completer with a recovery sweep backstop. Webhooks are out of scope for this phase. See `.planning/phases/06-payments-and-worker/CONTEXT.md`.
- **Plans**:
  - `06-01`: Migrations `0006_create_payment_attempts` and `0007_create_idempotency_keys` (`payload_hash`, `recovery_point`, `locked_at`, partial index).
  - `06-02`: Domain models (`PaymentAttempt`, `PaymentStatus`, `IdempotencyRecord`, `RecoveryPoint`, `payload_hash` helper) and infra actions (idempotency lookup, insert attempt, status-conditional claim, two-phase settle, recovery find/reclaim).
  - `06-03`: `POST /v1/invoices/{id}/pay` (Phase-1 commit, `pg_notify('payments', ...)`, `202`, idempotency and failure modes a/d/e). Endpoint never calls the PSP.
  - `06-04`: `minidodo-worker` crate (PSP client, completer, NOTIFY listener, recovery sweep), `worker` CLI subcommand, workspace and compose wiring.

### Phase 7: Webhooks (Planned)
- **Goal**: Signed webhook delivery for payment outcomes. Seeded `webhooks` endpoint (no CRUD), transactional outbox staging in `webhook_deliveries` inside the settle transaction, HMAC-SHA256 signing, LISTEN/NOTIFY plus `for update skip locked` claim (same as payments), and an inline retry loop with exponential backoff in the worker. No recovery sweep and no dead letter queue for webhooks (documented cuts). Mock receiver lives in `minidodo-psp` and verifies the signature. See `.planning/phases/07-webhook-deliveries/CONTEXT.md`.
- **Plans**:
  - `07-01`: Migrations `0008_create_webhooks` (seeded endpoint) and `0009_create_webhook_deliveries` (`status`, `attempts`, `last_error`, `last_attempt_at`; no `next_attempt_at`/`locked_at`).
  - `07-02`: Domain models `WebhookEndpoint`, `WebhookDelivery`, `WebhookEventType` (`invoice.paid`, `invoice.payment_failed`), `WebhookDeliveryStatus`, payload plus HMAC-SHA256 sign and verify helpers.
  - `07-03`: Postgres actions (one per file): stage inside the settle tx, notify, claim (skip locked), mark delivered, mark failed. Wire staging into `settle_success`/`settle_failure`.
  - `07-04`: Worker webhook listener and inline retry deliverer, signed outbound delivery, and the verifying `POST /webhooks/sink` mock receiver in `minidodo-psp`. No sweep.

### Phase 8: Verification & Testing Suite
- **Goal**: Automated integration tests covering concurrency, idempotency, and failure modes.
- **Plans**:
  - `08-01`: Concurrency test for N simultaneous `POST /pay` requests on the same invoice (at most one succeeds, no double charge, consistent final state).
  - `08-02`: Idempotency test verifying identical responses on retry and no duplicate settlement.
  - `08-03`: PSP failure test (`tok_timeout`, `tok_network_error`) asserting the invoice is never stuck in a bad state.

### Phase 9: Docker Compose & Operational Packaging
- **Goal**: Full stack `docker compose up` running Postgres, migrations, API server, mock PSP, and worker.
- **Plans**:
  - `09-01`: Finalize `docker-compose.yaml` with all services.
  - `09-02`: Export OpenAPI 3.1 `openapi.json` / `openapi.yaml`.
  - `09-03`: Final updates to `README.md`, `DESIGN.md`, and `AI_USAGE.md`.

---
*Roadmap updated: 2026-09-03*
