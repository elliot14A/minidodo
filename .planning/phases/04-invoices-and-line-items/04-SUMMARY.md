# Phase 4 Summary: Invoices & Line Items

## Accomplishments
- **Database Migrations (`minidodo-infra`)**:
  - `0004_create_invoices.up.sql` / `down.sql`: Created PostgreSQL enum `invoice_state ('draft', 'open', 'processing', 'paid', 'void', 'uncollectible')`, `invoices` table with `total_cents BIGINT`, `due_date`, and indexes `idx(business_id, state)`, `idx(customer_id)`, `idx(business_id, created_at desc)`.
  - `0005_create_line_items.up.sql` / `down.sql`: Created `line_items` table with foreign key cascade to `invoices`, integer `quantity`, integer minor units `unit_amount_cents BIGINT`, and index `idx(invoice_id)`.
- **Domain Models & Integer Cents Math (`minidodo-core`)**:
  - `models/line_item.rs`: `LineItem` and `NewLineItem`.
  - `models/invoice.rs`: `Invoice`, `InvoiceState` enum mapped directly to postgres enum type `invoice_state`, `InvoiceWithLineItems`, `UpdateInvoiceStateTarget`, and server-side total calculation: `total_cents = sum(quantity * unit_amount_cents)`.
  - `error_codes.rs`: `InvoiceErrorCode::INVALID_STATE_TRANSITION` (`"INVOICE_INVALID_STATE_TRANSITION"`).
- **Transactional Database Actions (`minidodo-infra`)**:
  - `actions/invoices/create.rs`: Transactional insertion of invoice in `draft` state and associated line items.
  - `actions/invoices/get.rs`: Tenant-scoped retrieval of invoice with its line items.
  - `actions/invoices/list.rs`: Paginated list supporting optional filtering by `state` and `customer_id`.
  - `actions/invoices/update_state.rs`: Status-conditional atomic update guarded per state machine rules (`draft -> open`, `open -> void`, `open -> uncollectible`). Returns clear, helpful `409 Conflict` error on invalid transition naming current state.
- **3-Tier API Endpoints & OpenAPI (`minidodo-server`)**:
  - `POST /v1/invoices`: Validated line items and customer, returns `201 Created` with `InvoiceWithLineItems`.
  - `GET /v1/invoices/{id}`: Returns `200 OK` with full invoice details.
  - `GET /v1/invoices`: Returns paginated invoices with filter support.
  - `PATCH /v1/invoices/{id}`: Guarded state transitions (`open`, `void`, `uncollectible`).
  - Mounted `InvoicesApiDoc` into OpenAPI root document.
- **Live Verification**:
  - Created invoice with 2 items ($150 * 2 + $200 * 1 = $500 => `50000` cents) -> `201 Created` in `draft` state.
  - `PATCH /v1/invoices/{id}` with `state: "open"` -> `200 OK` (transitioned to `open`).
  - `PATCH /v1/invoices/{id}` with `state: "void"` -> `200 OK` (transitioned to `void`).
  - `PATCH /v1/invoices/{id}` with `state: "open"` from `void` -> `409 Conflict` (`{"message": "cannot mark invoice as 'open': invoice is in terminal state 'void'", "code": "INVOICE_INVALID_STATE_TRANSITION"}`).
  - `GET /v1/invoices?state=void` -> `200 OK` (returned paginated result matching filter).
