# Phase 7 Context: Webhook Endpoints & Deliveries

## Goal
Deliver payment lifecycle events to a registered HTTP endpoint using a transactional
outbox, LISTEN/NOTIFY driven delivery, HMAC-SHA256 signing, and an inline retry loop with
exponential backoff. The enqueue and claim path mirrors the Phase 6 payment path (NOTIFY
plus `for update skip locked`). There is no recovery sweep and no dead letter queue for
webhooks; those are deliberate cuts documented in DESIGN.md section 6.

## Locked decisions
- **Transactional outbox.** The `webhook_deliveries` row is inserted inside the same
  Phase 2 settle transaction that changes the invoice state (`settle_success` and
  `settle_failure`). The state change and the enqueue commit atomically, so a settled
  payment always has its delivery staged and a rolled back settle stages nothing.
- **NOTIFY plus skip-locked claim, same as payments. No sweep.** After the settle
  transaction commits, the worker fires `pg_notify('webhooks', <delivery_id>)`. A worker
  picks the delivery off the `webhooks` channel and claims it with `for update skip locked`
  so two workers cannot grab the same row. There is no recovery sweep: if a notification is
  lost or the worker crashes mid delivery, that delivery is not recovered. This is an
  accepted cut for the assignment (payments have the durable recovery story; webhooks are
  best effort). Documented in DESIGN.md section 6.
- **Inline retry loop, no dead letter queue.** After claiming, the deliverer POSTs and, on
  failure, retries in memory with exponential backoff sleeps up to a max attempt count.
  Since there is no sweep to re-drive a row later, backoff is driven by the inline loop, not
  a `next_attempt_at` column. After the last attempt fails the worker stops trying and marks
  the row `failed` for observability only (not a re-drivable dead letter). Specific numbers
  are documented in DESIGN.md section 4.
- **Events (assignment names).** Two event types: `invoice.paid` (invoice reached `paid`)
  and `invoice.payment_failed` (definitive PSP failure, invoice returned to `open`). Both
  are staged from the settle actions. The assignment lists `invoice.created` as well, but it
  is deliberately cut to keep the outbox hook confined to the two settle transactions and
  avoid touching the invoice create path. This cut is documented in DESIGN.md section 6.
- **Signing is required and graded.** The assignment states delivery must be signed so
  receivers can verify, and DESIGN.md section 4 must document the signing scheme (algorithm,
  what is signed, replay protection). The worker signs every outbound request with
  HMAC-SHA256 over the timestamp plus the exact body bytes, sent as
  `X-Webhook-Signature: sha256=<hex>` and `X-Webhook-Timestamp: <unix>`. The timestamp is
  the replay protection input.
- **Verifying mock receiver in `minidodo-psp`.** A `POST /webhooks/sink` endpoint in the
  mock external service crate recomputes the HMAC over the received timestamp plus body,
  constant-time compares it to the `X-Webhook-Signature` header, and returns `200 OK` on a
  match or `401` on a mismatch. It does not store or process the payload beyond that. This
  keeps nothing as theater: the worker signs and the receiver verifies, end to end under
  `docker compose up`. The sink knows the shared secret through the same seeded value the
  endpoint row carries (passed via the PSP config). Delivery results are observable on the
  sender side via the `webhook_deliveries` table (status, attempts, delivered), which is
  what the demo shows.
- **Seeded endpoint, no CRUD.** One `webhooks` row is seeded in the migration for
  the default business, pointing at the mock sink URL with a signing secret. Endpoint
  registration CRUD is a documented scope cut (DESIGN.md section 6). The interesting part,
  the delivery pipeline, is still fully exercised.

## Money and SQL rules (unchanged)
- Lowercase SQL, explicit `sqlx::query!` / `query_as!` in action files, one action per file.
- No lock or open transaction held across the outbound webhook HTTP call. Claim with
  `for update skip locked`, release, then deliver, then update with a status conditional
  write.
- No `unwrap` / `expect` in production paths. Tenant scope every query by `business_id`.
- HMAC secret is stored on the endpoint row, never logged.

## Deliberate cuts for webhooks (DESIGN.md section 6)
- No recovery sweep and no crash recovery. Best effort delivery driven by NOTIFY only.
- No dead letter queue. The `failed` status is observability only, not re-drivable.
- No `invoice.created` event.
- No endpoint registration CRUD; one seeded endpoint.

## Hook points (confirmed against current code)
- `minidodo-infra/src/postgres/actions/payments/settle_success.rs`: between the `invoices`
  update and `tx.commit()` (currently line 53 to 75), insert one `webhook_deliveries` row
  per registered endpoint for `invoice.paid`.
- `minidodo-infra/src/postgres/actions/payments/settle_failure.rs`: same insertion point for
  `invoice.payment_failed`.
- `minidodo-worker/src/completer.rs`: after each `settle_*` call returns true, fire
  `pg_notify('webhooks', <delivery_id>)` for the staged deliveries.
- `minidodo-worker/src/serve.rs`: currently spawns the payments listener and sweep via
  `tokio::select`. Add the webhook listener alongside them (no webhook sweep).

## Plans
- `07-01`: migrations `0008_create_webhooks` (with seed) and
  `0009_create_webhook_deliveries` (outbox, no backoff column).
- `07-02`: domain models `WebhookEndpoint`, `WebhookDelivery`, `WebhookEventType`,
  `WebhookDeliveryStatus`, canonical payload plus sign and verify helpers.
- `07-03`: infra actions (one per file) for staging inside the settle tx, notify, claim
  (skip locked), mark delivered, mark failed. Wire staging into `settle_*`.
- `07-04`: worker webhook listener and inline retry deliverer, the outbound signed HTTP
  delivery, plus the verifying `POST /webhooks/sink` mock receiver in `minidodo-psp`, and
  compose wiring. No sweep.
