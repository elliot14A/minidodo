# DESIGN.md: Invoice & Payment Service

A minimal invoice and payment service. Businesses create invoices for customers, customers
pay them, and businesses are notified of state changes through signed webhooks. The
interesting work is the payment state machine, concurrency, and PSP failure handling. This
document covers those directly.

**Stack:** Rust, Axum, and sqlx on Postgres. Three processes share one database: the API
server, a mock PSP, and a worker that handles payment completion and webhook delivery.

Background work is driven off domain tables rather than a generic queue. Payment completion and
crash recovery key off `idempotency_keys`, where the `recovery_point` column records how far a
payment got. Webhook delivery keys off `webhook_deliveries`. A worker sweep reclaims work whose
`locked_at` is older than a fixed timeout, which is how a payment left in flight by a crash is
resumed.

---

## 1. Data Model

Money is stored as integer minor units (cents) in `BIGINT` columns everywhere. No floats
appear in any money column, computation, or serialization path. Totals are always computed
server-side from line items. A client-supplied total is ignored.

| Table | Shape (key columns) | PK strategy | Notable indexes |
|---|---|---|---|
| `businesses` | `id`, `name`, `created_at` | UUID v4 | none |
| `api_keys` | `id`, `business_id`, `token_hash` (bytea), `token_prefix`, `name`, `created_at` | UUID v4 | `unique(token_hash)`; `idx(business_id)`; `idx(token_prefix)` |
| `customers` | `id`, `business_id`, `name`, `email`, `created_at` | UUID v4 | `idx(business_id, created_at desc)` |
| `invoices` | `id`, `business_id`, `customer_id`, `state`, `total_cents BIGINT`, `due_date`, `created_at` | UUID v4 | `idx(business_id, state)`; `idx(customer_id)` |
| `line_items` | `id`, `invoice_id`, `description`, `quantity INT`, `unit_amount_cents BIGINT` | UUID v4 | `idx(invoice_id)` |
| `payment_attempts` | `id`, `invoice_id`, `business_id`, `idempotency_key`, `payload_hash`, `status`, `psp_ref`, `psp_error_code`, `created_at` | UUID v4 | `unique(business_id, idempotency_key)`; `idx(invoice_id)` |
| `idempotency_keys` | `business_id`, `idempotency_key`, `payload_hash`, `recovery_point`, `locked_at`, `last_run_at`, `response_code`, `response_body`, `created_at` | `(business_id, idempotency_key)` | `idx(recovery_point, locked_at) where recovery_point <> 'finished'` |
| `webhooks` | `id`, `business_id`, `url`, `signing_secret`, `active`, `created_at` | UUID v4 | `idx(business_id)` |
| `webhook_deliveries` | `id`, `endpoint_id`, `business_id`, `event_type`, `payload jsonb`, `status`, `attempts`, `last_error`, `last_attempt_at`, `created_at` | UUID v4 | `idx(business_id, created_at desc)` |

**Design choices and alternatives.**

- **UUID v4 primary keys over serial `BIGINT`.** Keys are business-scoped and appear in URLs.
  Opaque, non-enumerable ids avoid leaking volume and avoid insert coordination. The cost is
  16 bytes and random index locality, which is fine at this scale.
- **`line_items` as its own table rather than a `jsonb` column on `invoices`.** Line items
  are the audit trail for how `total_cents` was computed. A real table keeps the sum
  verifiable and queryable. A `jsonb` blob would save a join but weaken auditability, which
  matters for money.
- **`idempotency_keys` kept separate from `payment_attempts`.** The idempotency row is a
  request recovery record. It tracks `recovery_point` and holds the replayable response. The
  attempt is the domain record of a single charge try. They have different lifecycles, so
  keeping them apart keeps each one single-purpose.

**At 100x scale.** Partition `invoices` and `payment_attempts` by `business_id` or by time so
hot indexes stay small. Add a read replica for invoice listing. Prune old idempotency keys (24
to 72 hours) and terminal webhook deliveries on a retention sweep.

---

## 2. Invoice State Machine

```
                 create
                   │
                   ▼
   ┌─────────┐  finalize   ┌────────┐
   │  draft  │───────────► │  open  │
   └─────────┘             └───┬────┘
        │                      │
        │ void                 │ POST /pay  (claim)
        ▼                      ▼
   ┌────────┐            ┌────────────┐
   │  void  │◄─── void ──│ processing │
   └────────┘   (open)   └─────┬──────┘
   (terminal)                  │
                     PSP result│
              ┌────────────────┼───────────────────┐
              │ succeeded      │ definitive failure │ (unknown/timeout)
              ▼                ▼                    │
         ┌────────┐       back to open ◄────────────┘ (retryable)
         │  paid  │       (payment_failed)
         └────────┘
         (terminal)        [ uncollectible ]  (manual/admin, terminal)
```

**States:** `draft`, `open`, `processing`, `paid` (terminal), `void` (terminal),
`uncollectible` (terminal).

**Transitions and triggers:**

- `draft -> open`: finalize. The invoice is ready to be paid.
- `open -> void`: the business voids an unpaid invoice.
- `open -> processing`: a `POST /pay` request wins the atomic claim.
- `processing -> paid`: the PSP returns success.
- `processing -> open`: the PSP returns a definitive failure (`tok_card_declined`,
  `tok_insufficient_funds`, or `tok_network_error`). The invoice becomes payable again and an
  `invoice.payment_failed` event fires.
- `processing -> void`: not allowed. An in-flight charge must resolve first.
- `-> uncollectible`: an administrative terminal state for invoices written off. It sits
  outside the automated payment path.

**How transitions are triggered.** The payment transitions (`open -> processing -> paid` and
the `processing -> open` reversal) are driven by `POST /v1/invoices/{id}/pay` and the PSP
result. The non-payment transitions are driven by a single `PATCH /v1/invoices/{id}` endpoint
that takes a target `state` in the body: `draft -> open` (finalize), `open -> void`, and
`open -> uncollectible`. This endpoint is a guarded transition, not a raw field setter. It
never accepts `paid` or `processing` as a target, and it never moves an invoice out of a
terminal state. Each accepted target maps to one status-conditional update, so the set of
legal transitions is enforced by the same mechanism as the pay claim.

**Reversibility and rejection.** `processing -> open` is the only reversal, and it happens
only on a definitive PSP failure, never on a timeout (see section 3b). `paid`, `void`, and
`uncollectible` are terminal and irreversible. Invalid transitions are rejected at the
database level by the status-conditional update. For the pay claim this is
`UPDATE invoices SET state='processing' WHERE id=$1 AND state='open'`; for a `PATCH` to `void`
it is `UPDATE invoices SET state='void' WHERE id=$1 AND state='open'`. If the current state
does not match the guard, the update affects 0 rows and the API returns `409 Conflict` with a
clear error naming the current state. There is no read-then-write race because the guard and
the write are the same statement.

---

## 3. Payment Correctness and Failure Modes

**Concurrency mechanism: a status-conditional atomic UPDATE (the claim), not a held lock.**
The payment is split into phases, and the core rule is that no lock and no open transaction is
ever held across the PSP network call. The claim is a single-statement conditional update.
The PSP call happens between committed transactions, with nothing locked. Each phase commits
the next `recovery_point` on the `idempotency_keys` row, so the row itself records how far the
payment got and there is no need for a separate queue entry.

On the worker side, both the notification path and the recovery sweep claim a job with
`select ... for update skip locked` before charging. `pg_notify` broadcasts to every listening
worker, so the notification alone does not guarantee a single processor. The skip-locked claim
does: the first worker to lock the row wins, every other worker skips it and gets zero rows, so
exactly one worker charges each payment. The lock is held only for the microsecond claim and
released before the PSP call, so the no-lock-across-I/O rule still holds. A second layer, the
`status='pending'` guard on the settle update, makes double settlement impossible even if two
charges somehow raced.

Why not the alternatives:

- **`SELECT ... FOR UPDATE` held across the PSP call.** This would hold a row lock for up to
  30 seconds during `tok_timeout`, blocking every concurrent payer on the slowest possible
  external call. Locking across a foreign I/O boundary is the anti-pattern this design avoids.
- **`SERIALIZABLE` plus retry.** Heavier, and 40001 retries interact badly with the long PSP
  window. The conditional update gives the same "exactly one winner" guarantee with a
  microsecond row lock and no retry loop.
- **Advisory locks.** These give mutual exclusion but not the "invoice must be open" check.
  The conditional update encodes both the lock and the state check in one statement.

The `Idempotency-Key` header is required on `POST /invoices/{id}/pay`. A request without it is
rejected with `400`. A money-moving endpoint should never accept a request it cannot safely
deduplicate, so we enforce the header rather than treating it as optional.

**Two independent guards against double-charging:**

1. `unique(business_id, idempotency_key)` on `payment_attempts` dedups retries of the same
   request.
2. The status-conditional claim (`WHERE state='open'`) dedups distinct requests racing for the
   same invoice.

Losing guard 1, for example when a client loses its idempotency key, degrades safely to guard
2, which returns a `409`. It never causes a double charge.

### (a) Two clients POST /pay the same invoice at the same instant
Both insert their own `payment_attempts` rows with distinct keys. Both then run the claim
`UPDATE invoices SET state='processing' WHERE id=$1 AND state='open'`. Postgres serializes
these on the row for microseconds. Exactly one gets `rows_affected = 1` and proceeds to the
PSP. The loser gets 0 rows, marks its attempt failed, and returns `409 Conflict` immediately.
It never waits on the PSP. At most one charge happens and the final state is consistent.

### (b) The PSP times out (`tok_timeout`, sleeps 30s then returns success)
`tok_timeout` is a slow success, not a failure. Treating it as failed would reject a payment
the PSP actually accepted. The API always returns `202 Accepted` after committing the claim
(`invoice=processing`, `attempt=pending`, `recovery_point='charge_pending'`). The worker owns
the PSP call, which has no HTTP-handler time limit, so the endpoint never hangs.

The completer runs on two mechanisms. The normal path is event-driven: `/pay` calls
`pg_notify` after committing, and the worker's `LISTEN` wakes and completes the charge in
milliseconds of wall time. The recovery sweep in (c) is the correctness backstop for when a
notification is lost, since `NOTIFY` is fire-and-forget. `NOTIFY` provides latency; the sweep
provides the guarantee.

When the PSP returns success at around 30 seconds, the worker commits `processing -> paid`.
The caller learns the eventual result through `GET /invoices/{id}` or by retrying with the same
idempotency key, which replays the stored terminal response.

### (c) PSP returns success but the service crashes before persisting it
The customer is not charged twice. The PSP is called with a deterministically derived
idempotency key (`invsvc-<attempt_id>`) that can be reconstructed from the row committed in
phase 1. On a crash the invoice sits in `processing` and the `idempotency_keys` row sits at
`recovery_point='charge_pending'` with a `locked_at` that stops advancing. The worker's
completer is a periodic sweep that finds these rows:

```sql
SELECT * FROM idempotency_keys
WHERE recovery_point <> 'finished'
  AND locked_at < now() - interval '60 seconds';
```

It reclaims each stale row and resumes from the recorded `recovery_point` by re-calling the
PSP with the same derived key. The invoice self-heals out of `processing` and the outcome is
finally persisted. No-double-charge on that re-call depends on the PSP being idempotent on a
key we control, which real PSPs like Stripe and Adyen are. The mock PSP does not implement
this and we do not simulate it, so we state it as an assumption: double-charge prevention is
necessarily the PSP's responsibility because it holds the money, and only it can dedup a
charge. Our responsibility is sending a crash-survivable key so every retry is dedupable, and
building the recovery sweep that re-drives a stuck payment. The stuck-invoice recovery is
built and verifiable here; the exactly-once charge rests on the documented PSP behavior.

The 60-second reclaim threshold must exceed the worker's PSP request timeout of 45 seconds, so
a call that is still legitimately in flight is never reclaimed underneath itself. The 45-second
client timeout in turn exceeds the 30-second `tok_timeout` slow success, so a slow-but-alive
charge completes normally rather than timing out. If a call exceeds 45 seconds it resolves to an
indeterminate outcome, the worker leaves the payment in `processing`, and the sweep re-drives it
after the reclaim threshold. The derived key makes that re-run safe, so the threshold is a
tuning knob rather than a correctness risk.

### (d) Idempotency key reused with a different request body
On the `unique(business_id, idempotency_key)` violation we load the existing row and compare
`payload_hash`. On a mismatch we return `422 Unprocessable Entity` ("idempotency key
reused with a different payload"). No second charge and no state change.

### (e) An already-paid invoice receives another POST /pay
A new key inserts a new attempt row, but the claim `WHERE state='open'` affects 0 rows because
the state is `paid`. The attempt is marked failed and we return `409 Conflict` ("invoice not
payable in state 'paid'"). No PSP call.

### tok_network_error (definitive failure)
The worker's PSP call errors with a 500 or a dropped connection. Unlike a timeout, this is a
definitive failure signal. The invoice goes `processing -> open`, the attempt is marked
`failed`, and we stage `invoice.payment_failed`. The invoice is payable again and is never
stuck in a bad state.

---

## 4. Webhook Design

**Events:** `invoice.paid` and `invoice.payment_failed`. Both are staged from the payment-settle
transactions. `invoice.created` is a documented cut (section 6).

**Who fires them: a transactional outbox.** The event is inserted into `webhook_deliveries` in
the same transaction as the settle it describes (`settle_success` or `settle_failure`). If the
transaction commits, the delivery row exists. If it rolls back, it does not. So `invoice.paid`
can never fire for an invoice that is not paid, and it can never be missed for one that is. This
decouples delivery from both the API response and the payment-charge logic. A flaky receiver
retries the delivery, it never re-runs the charge.

**Signing.** HMAC-SHA256 with a per-endpoint secret. The signed material is
`<unix_timestamp>.<exact serialized JSON body>`, sent as `X-Webhook-Signature: sha256=<hex>` and
`X-Webhook-Timestamp: <unix>`. Binding the timestamp into the signature is the replay-protection
input: a receiver recomputes the HMAC over the received timestamp plus body, rejects a stale
timestamp, and constant-time compares. The body carries an `event_id` (the delivery id).
Delivery is at-least-once, so receivers dedupe on `event_id`. The mock receiver in
`minidodo-psp` performs this verification and returns 200 on a match, 401 otherwise.

**Retry policy.** After the settle commits, the worker fires `pg_notify('webhooks', <id>)`. A
worker claims the row and delivers it with an inline retry loop: up to 5 attempts with
exponential backoff sleeps of 2s, 4s, 8s, and 16s between them, a total budget of about 30s. A
2xx marks the row `delivered`. When all 5 attempts fail the row is marked `failed`. `failed` is
a terminal record for observability only; nothing re-drives it (no dead letter queue, section 6).

**Reconciliation of missed events.** Businesses can list their invoices
(`GET /invoices?state=...`) and read the current state at any time. The API is the source of
truth and webhooks are a convenience notification. A production system would add long-horizon
retries, a delivery-log, and a replay endpoint, noted in section 7.

**Why delivery is decoupled, and how.** Delivery must not block the API response, since a slow
receiver would slow every payer. Writing the delivery row inside the settle transaction and
handing it to the worker over `LISTEN/NOTIFY` provides this: the API (and the settle) commit and
return in milliseconds while the worker delivers asynchronously. The claim uses
`for update skip locked` so two workers never grab the same row, mirroring the payment path, and
the status-conditional `where status = 'pending'` guard on the final write means a duplicate
delivery attempt can never flip a row twice.

Unlike payments, webhook delivery is deliberately best-effort. There is no recovery sweep and no
crash recovery: a lost notification or a worker crash mid-delivery drops that delivery. This is
an accepted trade (section 6). Payments hold the durable, crash-survivable recovery story because
they move money; webhooks are a notification whose ground truth is always re-readable from the
API, so the extra machinery is not worth it at this scale.

---

## 5. API Key Model

- **Generation.** A token of the form `dodo_<prefix>_<secret>`. The `<prefix>` is stored in
  plaintext for lookup and display. The full token is never stored. For this exercise one key
  is seeded at migration time (`dodo_test_key_12345` with prefix `dodo_test` for `Acme Corp`) and
  its plaintext is documented in the README.
- **Storage.** Only the SHA-256 hash of the token is stored in `token_hash` (as bytea), with
  `token_prefix` alongside it. Plaintext is never persisted or logged. SHA-256 is sufficient
  here because the token is high-entropy random, so slow password hashing (Argon2) buys
  nothing.
- **Transmission.** `Authorization: Bearer dodo_...` over TLS, with TLS terminated at the edge
  in production. Never in query strings, which leak into logs.
- **Lookup.** By `token_prefix` (indexed) then a constant-time hash comparison. A leaked
  prefix alone reveals nothing, and lookups do not leak timing.
- **Rotation and revocation.** Not implemented. The token is a static seeded credential tied to
  one business. Revocation would be a `status` column checked on every request, and rotation
  would allow multiple active keys per business, but neither is built here (see section 6).
- **Blast radius if leaked.** A key is scoped to one business, so a leaked key can act only
  within that business and carries no cross-business authority. Without revocation, the
  mitigation is to reseed the key. In production the first additions would be a revocation
  status flip and alerting on anomalous usage.

---

## 6. What We Cut and Why

- **Refunds and partial payments.** Out of scope. This would add a `refunds` table and a
  `paid -> refunded` transition. The integer-cents model already supports it.
- **A durable message broker (NATS or SQS).** Background work stays on Postgres, which keeps
  `docker compose up` to a single datastore. `pg_notify` wakes the worker and the recovery
  sweep is the correctness backstop. A broker with redelivery would be the move once there are
  many job types, the load grows, or the fire-and-forget window of a lost notification during a
  full worker outage needs to be closed at the transport layer rather than by the sweep.
- **PSP idempotency in the mock.** The recovery re-call is safe from double-charging only
  because a production PSP dedups on the derived key. The mock does not implement this, so we
  assume it (see section 3c). The built recovery sweep still guarantees the invoice never stays
  stuck in `processing`.
- **Worker hardening.** No hard mutex for multiple concurrent worker instances (a single worker
  is assumed), no heartbeat or liveness probe, and no graceful in-flight drain on shutdown. A
  killed worker's claimed rows are recovered by the sweep after the reclaim timeout.
- **Durable webhook delivery.** Webhooks are deliberately best-effort: `pg_notify` drives an
  inline retry loop, but there is no recovery sweep, no crash recovery, and no dead letter queue
  for deliveries. A lost notification or a worker crash mid-delivery drops that delivery. Unlike
  payments there is no money at stake and the current state is always re-readable from the API,
  so the durable machinery is not worth it here. Endpoint registration CRUD is also cut: one
  `webhooks` endpoint is seeded. The `invoice.created` event is cut so the outbox hook stays
  confined to the two settle transactions. A production build would add long-horizon retries, a
  durable sweep, and a replay endpoint (section 7).
- **API key rotation and revocation.** The assignment leaves revocation to our discretion. One
  key is seeded and tied to a business, which satisfies scoped authentication. A `status` and
  `revoked_at` column would add revocation, and multiple active keys per business would add
  zero-downtime rotation, but neither earns its complexity for a single-tenant demo.
- **Production rate limiting.** Discussed in section 7 rather than built.
- **Multi-currency, tax, subscriptions, and email sending.** Explicitly out of scope. Email is
  logged as "would send."

## 7. Production Readiness Gap

If this shipped tomorrow, the top gaps are:

1. **Observability.** Structured tracing spans exist, but there are no metrics (payment success
   rate, PSP latency, webhook delivery lag) and no alerting on stuck-`processing` invoices or
   `failed` webhook deliveries. This is the first thing I would add.
2. **Rate limiting and abuse controls.** Per-API-key limits (via `tower_governor`) plus a cap
   on payment attempts per invoice to bound PSP spend.
3. **Full reconciliation and an audit log.** An append-only audit trail of every state
   transition and PSP interaction, plus a scheduled reconciliation job that re-queries the PSP
   for any `processing` invoice older than a threshold. That closes the residual window where
   the worker itself is down for an extended period. A webhook replay and delivery-log endpoint
   would let businesses reconcile missed events.
