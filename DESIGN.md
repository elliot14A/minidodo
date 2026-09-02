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
| `payment_attempts` | `id`, `invoice_id`, `business_id`, `idempotency_key`, `request_fingerprint`, `status`, `psp_ref`, `psp_error_code`, `created_at` | UUID v4 | `unique(business_id, idempotency_key)`; `idx(invoice_id)` |
| `idempotency_keys` | `business_id`, `idempotency_key`, `request_fingerprint`, `recovery_point`, `locked_at`, `last_run_at`, `response_code`, `response_body`, `created_at` | `(business_id, idempotency_key)` | `idx(recovery_point, locked_at) where recovery_point <> 'finished'` |
| `webhook_endpoints` | `id`, `business_id`, `url`, `secret`, `created_at` | UUID v4 | `idx(business_id)` |
| `webhook_deliveries` | `id`, `endpoint_id`, `event_id`, `event_type`, `payload jsonb`, `status`, `attempts`, `next_attempt_at`, `locked_at` | UUID v4 | `idx(status, next_attempt_at)` |

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

**Reversibility and rejection.** `processing -> open` is the only reversal, and it happens
only on a definitive PSP failure, never on a timeout (see section 3b). `paid`, `void`, and
`uncollectible` are terminal and irreversible. Invalid transitions are rejected at the
database level by the status-conditional claim:
`UPDATE invoices SET state='processing' WHERE id=$1 AND state='open'`. If the invoice is not
`open`, this affects 0 rows and the API returns `409 Conflict` with a clear error naming the
current state. There is no read-then-write race because the guard and the write are the same
statement.

---

## 3. Payment Correctness and Failure Modes

**Concurrency mechanism: a status-conditional atomic UPDATE (the claim), not a held lock.**
The payment is split into phases, and the core rule is that no lock and no open transaction is
ever held across the PSP network call. The claim is a single-statement conditional update.
The PSP call happens between committed transactions, with nothing locked. Each phase commits
the next `recovery_point` on the `idempotency_keys` row, so the row itself records how far the
payment got and there is no need for a separate queue entry.

Why not the alternatives:

- **`SELECT ... FOR UPDATE` held across the PSP call.** This would hold a row lock for up to
  30 seconds during `tok_timeout`, blocking every concurrent payer on the slowest possible
  external call. Locking across a foreign I/O boundary is the anti-pattern this design avoids.
- **`SERIALIZABLE` plus retry.** Heavier, and 40001 retries interact badly with the long PSP
  window. The conditional update gives the same "exactly one winner" guarantee with a
  microsecond row lock and no retry loop.
- **Advisory locks.** These give mutual exclusion but not the "invoice must be open" check.
  The conditional update encodes both the lock and the state check in one statement.

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
the PSP call, which has no HTTP-handler time limit, so the endpoint never hangs. When the PSP
returns success at around 30 seconds, the worker commits `processing -> paid` and stages an
`invoice.paid` delivery in `webhook_deliveries`. The caller learns the eventual result through
the webhook, through `GET /invoices/{id}`, or by retrying with the same idempotency key, which
replays the stored terminal response.

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
PSP with the same derived key. The idempotent PSP replays the original result instead of
charging again. The charge happens exactly once, and recovery only re-learns its outcome. This
relies on the PSP being idempotent on a key we control, which real PSPs like Stripe and Adyen
are, and which I made the mock PSP honor for this reason. Double-charge prevention is
necessarily the PSP's responsibility, because it holds the money, so only it can dedup a
charge. Our responsibility is sending a crash-survivable key so every retry is dedupable.

The 60-second claim timeout only needs to exceed the longest legitimate PSP call (the 30-second
`tok_timeout`) so a slow-but-alive charge is not reclaimed underneath itself. Even if it were,
the derived key makes the re-run safe, so the timeout is a tuning knob rather than a
correctness risk.

### (d) Idempotency key reused with a different request body
On the `unique(business_id, idempotency_key)` violation we load the existing row and compare
`request_fingerprint`. On a mismatch we return `422 Unprocessable Entity` ("idempotency key
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

**Events:** `invoice.created`, `invoice.paid`, `invoice.payment_failed`.

**Who fires them: a transactional outbox.** The event is inserted into `webhook_deliveries` in
the same transaction as the state change it describes, which is either the invoice-create
transaction or the payment-settle transaction. If the transaction commits, the delivery row
exists. If it rolls back, it does not. So `invoice.paid` can never fire for an invoice that is
not paid, and it can never be missed for one that is. The webhook-delivery worker picks up
pending rows and delivers them, retrying on their own backoff schedule. This decouples delivery
from both the API response and the payment-charge logic. A flaky receiver retries the delivery,
it never re-runs the charge.

**Signing.** HMAC-SHA256 over the exact serialized JSON body, using a per-endpoint secret. The
header is `X-Webhook-Signature: sha256=<hex>`. We also sign a timestamp and include an
`event_id`. Receivers verify the signature, reject stale timestamps for replay protection, and
dedupe on `event_id` because delivery is at-least-once.

**Retry policy.** Exponential backoff with attempts at 0s, 30s, 2m, 10m, 1h, and 6h. That is a
maximum of 6 attempts with a total budget of about 8 hours. A delivery that returns 2xx is
marked `completed`. Otherwise `next_attempt_at` is advanced by the backoff schedule. When the
budget is exhausted the delivery is marked `failed`, which is a dead letter.

**Reconciliation of missed events.** Businesses can list their invoices
(`GET /invoices?state=...`) and read the current state at any time. The API is the source of
truth and webhooks are a convenience notification. A production system would add a delivery-log
and replay endpoint, noted in section 7.

**Why delivery is decoupled, and how.** Delivery must not block the API response, since a slow
receiver would slow every payer, and it must survive receiver downtime with long retries.
Writing the delivery row inside the state-change transaction and letting the worker pick it up
provides this. The worker polls `webhook_deliveries` for rows whose `next_attempt_at` is due. A
`LISTEN/NOTIFY` wakeup can be layered on as a latency optimization, but it is only a hint: the
due-time poll is the source of truth, so a lost notification never loses a delivery. The API
commits and returns in milliseconds, and the worker delivers asynchronously.

The worker is the smallest structure that satisfies this. The same process also completes
payments left in flight by a slow PSP or a crash, so one poll loop over two domain tables covers
both async concerns. That is why there is no generic job queue, no broker, and no heartbeat: they
would add moving parts without doing anything the sweep does not already do at this scale.

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
  `docker compose up` to a single datastore. A broker would be the move once there are many job
  types or the polling load grows.
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
   dead-lettered deliveries. This is the first thing I would add.
2. **Rate limiting and abuse controls.** Per-API-key limits (via `tower_governor`) plus a cap
   on payment attempts per invoice to bound PSP spend.
3. **Full reconciliation and an audit log.** An append-only audit trail of every state
   transition and PSP interaction, plus a scheduled reconciliation job that re-queries the PSP
   for any `processing` invoice older than a threshold. That closes the residual window where
   the worker itself is down for an extended period. A webhook replay and delivery-log endpoint
   would let businesses reconcile missed events.
