# Requirements: minidodo (Resource-Driven)

**Defined:** 2026-09-02
**Core Value:** Absolute financial correctness, double-charge prevention, and robust handling of network timeouts/crashes using database-level status-conditional claims, domain-table recovery, and deterministic PSP idempotency.

## v1 Requirements

### Workspace & Core Architecture
- [ ] **CORE-01**: Cargo workspace with crates `minidodo-core`, `minidodo-infra`, `minidodo-server`, `minidodo-worker`, `minidodo-psp`.
- [ ] **CORE-02**: Core error model `MinidodoError` with static string codes, implementing Axum `IntoResponse` with standardized `ErrorResponse` JSON envelope.
- [ ] **CORE-03**: Generic PostgreSQL infrastructure with connection pool, migration execution, and lowercase SQL error translation.
- [ ] **CORE-04**: Axum server setup with `ValidatedJson<T>` validation extractor, request tracing `TraceLayer`, and root `utoipa::OpenApi` doc.

### Resource 1: Businesses
- [ ] **BUS-01**: Migration for `businesses` table (`id uuid primary key`, `name`, `created_at`).
- [ ] **BUS-02**: Core domain model `Business` deriving `ToSchema`.
- [ ] **BUS-03**: Action-based queries under `postgres/actions/businesses/{create.rs, get.rs}` using lowercase SQL.
- [ ] **BUS-04**: API routes `POST /v1/businesses`, `GET /v1/businesses/{id}` with `routes.rs` and `mod.rs` `utoipa` docs.

### Resource 2: API Keys & Auth
- [ ] **AUTH-01**: Migration for `api_keys` (`token_hash bytea unique`, `token_prefix`, `status`, `idx(business_id)`).
- [ ] **AUTH-02**: Token generation `dodo_<prefix>_<secret>` and SHA-256 hashing.
- [ ] **AUTH-03**: Action-based queries under `postgres/actions/apikeys/{create.rs, get_by_prefix.rs, revoke.rs}`.
- [ ] **AUTH-04**: Axum `AuthContext` extractor resolving `business_id` from Bearer token with constant-time verification.
- [ ] **AUTH-05**: API routes `POST /v1/apikeys`, `GET /v1/apikeys`, `DELETE /v1/apikeys/{id}` with `utoipa` docs.

### Resource 3: Customers
- [ ] **CUST-01**: Migration for `customers` (`id`, `business_id`, `name`, `email`, `idx(business_id, created_at desc)`).
- [ ] **CUST-02**: Domain model `Customer` and `NewCustomer` with email/name validation.
- [ ] **CUST-03**: Action-based queries under `postgres/actions/customers/{create.rs, get.rs, list.rs}`.
- [ ] **CUST-04**: API routes `POST /v1/customers`, `GET /v1/customers/{id}`, `GET /v1/customers` scoped to authenticated business.

### Resource 4: Invoices & Line Items
- [ ] **INV-01**: Migration for `invoices` and `line_items` (`total_cents bigint`, `state` check, `idx(business_id, state)`).
- [ ] **INV-02**: Domain models `Invoice`, `InvoiceState` (`draft`, `open`, `processing`, `paid`, `void`, `uncollectible`), `LineItem` with server-side cents total computation.
- [ ] **INV-03**: Action-based queries under `postgres/actions/invoices/{create.rs, get.rs, list.rs, finalize.rs, void.rs}` with status checks.
- [ ] **INV-04**: API routes `POST /v1/invoices`, `POST /v1/invoices/{id}/finalize`, `POST /v1/invoices/{id}/void`, `GET /v1/invoices/{id}`, `GET /v1/invoices?state=...`.

### Resource 5: Mock PSP Service
- [ ] **PSP-01**: Standalone mock PSP HTTP server (`POST /charge`).
- [ ] **PSP-02**: Token simulation matrix (`tok_visa`, `tok_card_declined`, `tok_insufficient_funds`, `tok_timeout` 30s sleep, `tok_network_error` 500/drop).
- [ ] **PSP-03**: PSP idempotency tracking and outcome replaying.

### Resource 6: Webhooks
- [ ] **WEB-01**: Migration for `webhook_endpoints` and `webhook_deliveries` (`status`, `attempts`, `next_attempt_at`, `locked_at`).
- [ ] **WEB-02**: HMAC-SHA256 signature generator (`X-Webhook-Signature: sha256=<hex>`) signing payload and timestamp.
- [ ] **WEB-03**: Transactional delivery staging in `webhook_deliveries` on invoice state events (`invoice.created`, `invoice.paid`, `invoice.payment_failed`).
- [ ] **WEB-04**: API routes `POST /v1/webhooks` for endpoint registration.
- [ ] **WEB-05**: Standalone worker sweep delivering pending webhooks with exponential backoff.

### Resource 7: Payments, Concurrency & Recovery
- [ ] **PAY-01**: Migration for `payment_attempts` (`unique(business_id, idempotency_key)`) and `idempotency_keys` (`recovery_point`, `locked_at`).
- [ ] **PAY-02**: Status-conditional atomic claim query (`update invoices set state='processing' where id=$1 and state='open'`).
- [ ] **PAY-03**: API endpoint `POST /v1/invoices/{id}/pay` returning non-blocking 202 Accepted.
- [ ] **PAY-04**: Background worker payment completion and crash recovery sweep (finding stale `recovery_point <> 'finished'` and calling PSP with deterministic key `invsvc-<attempt_id>`).

### Verification & Delivery
- [ ] **TEST-01**: Concurrency test firing N concurrent `POST /pay` requests.
- [ ] **TEST-02**: Idempotency retry test.
- [ ] **TEST-03**: PSP failure/timeout recovery test.
- [ ] **PKG-01**: `docker-compose.yaml` running Postgres, migrations, and all 3 processes with one command.
- [ ] **DOC-01**: Updated `DESIGN.md`, `AI_USAGE.md`, and `README.md` with curl examples.

---
*Requirements updated: 2026-09-02*
