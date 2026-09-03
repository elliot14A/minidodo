# DESIGN.md: Invoice & Payment Service

A small invoice and payment service. A business creates invoices for its customers, customers pay
them, and the business is notified of state changes through signed webhooks. The product surface
is deliberately small. The real work is the payment state machine, concurrency, and how the
system behaves when the PSP is slow, fails, or the worker crashes at the wrong moment.

**Stack.** Rust, Axum, and sqlx on Postgres. Three processes share one database:

- `minidodo-server`: the API.
- `minidodo-psp`: the mock payment processor.
- `minidodo-worker`: completes payments and delivers webhooks.

**No message broker.** Background work is driven off the domain tables, not a queue. `pg_notify`
wakes the worker for low latency. A periodic sweep is the correctness backstop when a notification
is lost or a process crashes. This keeps `docker compose up` to one datastore.

---

## 1. Data Model

Money is stored as integer cents in `bigint` columns everywhere. No floats appear in any money
column, computation, or serialized field. An invoice total is always computed server-side from
its line items. A client-supplied total is ignored.

| Table | Key columns | PK | Indexes |
|---|---|---|---|
| `businesses` | `id`, `name`, `created_at` | uuid v4 | none |
| `api_keys` | `id`, `business_id`, `token_hash` (bytea), `token_prefix`, `name` | uuid v4 | `unique(token_hash)`, `idx(business_id)`, `idx(token_prefix)` |
| `customers` | `id`, `business_id`, `name`, `email` | uuid v4 | `idx(business_id, created_at desc)` |
| `invoices` | `id`, `business_id`, `customer_id`, `state`, `total_cents bigint`, `due_date` | uuid v4 | `idx(business_id, state)`, `idx(customer_id)` |
| `line_items` | `id`, `invoice_id`, `description`, `quantity int`, `unit_amount_cents bigint` | uuid v4 | `idx(invoice_id)` |
| `payment_attempts` | `id`, `invoice_id`, `business_id`, `idempotency_key`, `payload_hash`, `status`, `psp_ref`, `psp_error_code` | uuid v4 | `unique(business_id, idempotency_key)`, `idx(invoice_id)` |
| `idempotency_keys` | `business_id`, `idempotency_key`, `payload_hash`, `recovery_point`, `locked_at`, `response_code`, `response_body` | `(business_id, idempotency_key)` | `idx(recovery_point, locked_at) where recovery_point <> 'finished'` |
| `webhooks` | `id`, `business_id`, `url`, `signing_secret`, `active` | uuid v4 | `idx(business_id)` |
| `webhook_deliveries` | `id`, `endpoint_id`, `business_id`, `event_type`, `payload jsonb`, `status`, `attempts`, `last_error`, `last_attempt_at` | uuid v4 | `idx(business_id, created_at desc)` |

**Why this shape.**

- **UUID v4 keys, not serial ints.** Ids appear in URLs and are business-scoped. Opaque keys do
  not leak volume and need no insert coordination. The cost is 16 bytes and random index
  locality, which is fine at this scale.
- **`line_items` as a real table, not a jsonb blob.** Line items are the audit trail for how
  `total_cents` was computed. A real table keeps the sum queryable and verifiable, which matters
  for money.
- **`idempotency_keys` separate from `payment_attempts`.** The idempotency row is a request
  recovery record: it holds `recovery_point` and the replayable response. The attempt is the
  domain record of one charge try. Different lifecycles, so they stay separate.

---

## 2. Invoice State Machine

Happy path, left to right. Branches are listed in the table below it.

```
draft --> open --> processing --> paid
```

**States:** `draft`, `open`, `processing`, `paid` (terminal), `void` (terminal), `uncollectible`
(terminal).

**Transitions.**

| From | To | Trigger | Notes |
|---|---|---|---|
| (create) | `draft` | create invoice | starting state |
| `draft` | `open` | `PATCH state=open` (finalize) | now payable |
| `open` | `processing` | `POST /pay` (atomic claim) | one winner per invoice |
| `open` | `void` | `PATCH state=void` | business voids an unpaid invoice |
| `open` | `uncollectible` | `PATCH state=uncollectible` | manual write-off |
| `processing` | `paid` | PSP success | terminal |
| `processing` | `open` | PSP definite failure | fires `invoice.payment_failed`, payable again |
| `processing` | `processing` | PSP timeout / unknown | stays put, sweep re-drives later |

**Terminal:** `paid`, `void`, `uncollectible`. No transitions leave them.

`processing -> void` is not allowed: an in-flight charge must resolve first.

---

## 3. Payment Correctness and Failure Modes

**Concurrency mechanism: a status-conditional atomic update, not a held lock.** The claim is
`update invoices set state='processing' where id=$1 and state='open'`. No lock and no open
transaction is ever held across the PSP network call. Payment runs in two phases that each commit
a `recovery_point` on the `idempotency_keys` row. The worker claims work with
`select ... for update skip locked`, so exactly one worker charges each payment, and the lock is
released before the PSP call.

Why not the alternatives: a `select ... for update` held across the PSP call would lock the row
for up to 30s during `tok_timeout` and block every concurrent payer, which is the anti-pattern we
avoid. `serializable` plus retry is heavier and its 40001 retries interact badly with the long PSP
window. Advisory locks give mutual exclusion but not the "must be open" check. The conditional
update encodes lock and state check in one statement.

`Idempotency-Key` is required on `POST /pay` (missing gives `400`). Two guards prevent
double-charging: `unique(business_id, idempotency_key)` dedups retries of the same request, and
the conditional claim dedups distinct requests racing for the same invoice.

**(a) Two clients POST /pay the same invoice at once.** Both insert their own attempt rows. Both
run the claim. Postgres serializes them on the invoice row for microseconds. Exactly one gets one
row affected and goes to the PSP. The loser gets zero rows, marks its attempt failed, and returns
`409` immediately without ever calling the PSP. At most one charge.

**(b) The PSP times out (`tok_timeout`, 30s then success).** This is a slow success, not a
failure. The API always returns `202 Accepted` right after committing the claim (`invoice =
processing`, `recovery_point = charge_pending`). The worker owns the PSP call and has no HTTP time
limit, so the endpoint never hangs. When the PSP returns at around 30s the worker commits
`processing -> paid`. The caller learns the result from `GET /invoices/{id}` or by retrying with
the same idempotency key, which replays the stored response.

**(c) The PSP succeeds but the worker crashes before saving it.** This is the important case, and
the customer must not be charged twice. What we do:

1. Before calling the PSP, phase 1 has already committed the idempotency key. We send the PSP a
   fixed idempotency key derived from the attempt id (`invsvc-<attempt_id>`), so it survives a
   crash and is identical on any re-run.
2. On crash the invoice stays in `processing` and the `idempotency_keys` row stays at
   `recovery_point='charge_pending'` with a `locked_at` that stops advancing.
3. The worker sweep finds rows stalled past the reclaim threshold and re-runs the payment, sending
   the same derived key again:

   ```sql
   select * from idempotency_keys
   where recovery_point <> 'finished'
     and locked_at < now() - interval '60 seconds';
   ```

4. The invoice moves out of `processing` and the outcome is finally persisted.

**The assumption.** Step 3 calls the PSP a second time for a charge that already went through. We
rely on the PSP to recognize the repeated idempotency key and not charge again. This is how real
PSPs behave (Stripe, Adyen). Our mock PSP does not implement it, and we do not fake it. Our
responsibility is to always send a crash-survivable key and to re-drive stuck payments, both built
and tested here. Deduping the charge itself is the PSP's job, because only it holds the money. So
the stuck-invoice recovery is real and verified; exactly-once charging rests on this documented
PSP behavior.

The timeouts are ordered on purpose: the 60s reclaim threshold is longer than the worker's 45s PSP
timeout (so a call still in flight is never reclaimed under itself), which is longer than the 30s
`tok_timeout` (so a slow-but-alive charge finishes normally). Because the resend key makes re-runs
safe, these are tuning knobs, not correctness risks.

**(d) Idempotency key reused with a different body.** On the unique violation we load the existing
row and compare `payload_hash`. On a mismatch we return `422` and do nothing else. No second
charge, no state change.

**(e) A paid invoice receives another POST /pay.** A new key inserts a new attempt, but the claim
`where state='open'` affects zero rows. The attempt is marked failed and we return `409` ("invoice
not payable in state 'paid'"). No PSP call.

**`tok_network_error`.** A 500 or dropped connection is a definite failure, unlike a timeout. The
invoice goes `processing -> open`, the attempt is marked `failed`, and we stage
`invoice.payment_failed`. The invoice is payable again and never stuck.

---

## 4. Webhook Design

**Events.** `invoice.created`, `invoice.paid`, and `invoice.payment_failed`. `invoice.created` is
staged inside the invoice-create transaction; the paid and failed events are staged from the settle
transactions. All three use the same transactional outbox.

**Registration.** Businesses register endpoint URLs with `POST /v1/webhooks` (plus `GET` to list
and fetch), scoped to the authenticated business. The server generates the per-endpoint signing
secret (`whsec_...`) and returns it once in the create response. The secret is stored in plaintext
because HMAC signing needs the raw key, unlike the API key which is a bearer token we only compare
and therefore hash. Neither is ever logged.

**Transactional outbox.** The delivery row is inserted into `webhook_deliveries` in the same
transaction as the settle it describes. If the settle commits, the row exists; if it rolls back,
it does not. So `invoice.paid` can never fire for an invoice that is not paid, and can never be
missed for one that is.

**Signing.** HMAC-SHA256 with a per-endpoint secret. The signed material is
`<unix_timestamp>.<exact JSON body>`, sent as `X-Webhook-Signature: sha256=<hex>` and
`X-Webhook-Timestamp: <unix>`. Binding the timestamp into the signature is the replay protection:
a receiver recomputes the HMAC over the received timestamp plus body, rejects a stale timestamp,
and constant-time compares. The body carries an `event_id` so receivers can dedupe. The mock sink
in `minidodo-psp` performs exactly this check and returns 200 or 401.

**Retry policy.** After the settle commits, the worker fires `pg_notify('webhooks', <id>)`, claims
the row with `for update skip locked`, and delivers with an inline retry loop: up to 5 attempts
with backoff sleeps of 2s, 4s, 8s, 16s, a total budget of about 30s. A 2xx marks the row
`delivered`. All 5 failing marks it `failed`.

**Exhausted budget.** `failed` is a terminal record for observability only. Nothing re-drives it.
There is no dead letter queue (section 6).

**Reconciling missed events.** The API is the source of truth. A business reads current state any
time via `GET /invoices` or `GET /invoices/{id}`. Webhooks are a convenience notification, not the
system of record.

**Why decoupled, and how.** A slow receiver must not slow every payer. Writing the delivery row
inside the settle transaction and handing it to the worker over `LISTEN/NOTIFY` lets the API and
settle commit in milliseconds while the worker delivers asynchronously.

Unlike payments, webhook delivery is deliberately best-effort. There is no sweep and no crash
recovery: a lost notification or a worker crash mid-delivery drops that delivery. This is an
accepted trade (section 6), because there is no money at stake and the current state is always
re-readable from the API.

---

## 5. API Key Model

- **Generation.** A token shaped `dodo_<prefix>_<secret>`. One key is seeded at migration time
  (`dodo_test_key_12345`, prefix `dodo_test`, for `Acme Corp`) and its plaintext is in the README.
- **Storage.** Only the SHA-256 hash is stored (`token_hash` bytea), with `token_prefix` for
  lookup. Plaintext is never persisted or logged. SHA-256 is enough because the token is
  high-entropy random, so slow password hashing buys nothing.
- **Transmission.** `Authorization: Bearer dodo_...` over TLS, terminated at the edge. Never in
  query strings, which leak into logs.
- **Lookup.** By `token_prefix` (indexed), then a constant-time hash compare. A leaked prefix
  alone reveals nothing.
- **Rotation and revocation.** Not built. The key is a static seeded credential. Revocation would
  be a `status` column checked per request; rotation would allow multiple active keys per business
  (section 6).
- **Blast radius.** A key is scoped to one business, so a leak grants no cross-business authority.
  Without revocation the mitigation is to reseed the key.

---

## 6. What We Cut and Why

- **PSP idempotency in the mock.** The crash-recovery re-call is safe only because a real PSP
  dedups on the derived key. The mock does not, so we assume it (section 3c) rather than build a
  fake that would prove nothing. The stuck-invoice recovery itself is built and tested.

- **Durable webhook delivery.** Best-effort by choice: `pg_notify` plus an inline retry loop, but
  no recovery sweep, no crash recovery, and no dead letter queue. A lost notification or a crash
  mid-delivery drops that delivery. No money is at stake and state is re-readable from the API.
  Endpoint registration is limited to create, list, and get; update, delete, and secret rotation
  are cut.

- **Worker high availability.** We assume at least one worker is running. `pg_notify` is
  fire-and-forget, so a notification sent while no worker is listening is lost. The periodic sweep
  (every 30s, reclaim threshold 60s) recovers those payments, but only once a worker is back up:
  nothing sweeps while every worker is down. There is also no hard mutex across multiple worker
  instances (single worker assumed) and no graceful in-flight drain on shutdown; a killed worker's
  claimed rows are recovered by the sweep after the reclaim threshold.

- **A message broker (NATS, SQS).** Background work stays on Postgres to keep setup to one
  datastore. `pg_notify` gives latency and the sweep gives the guarantee. A broker earns its place
  once there are many job types or the lost-notification window during a full worker outage must
  be closed at the transport layer.

- **API key rotation and revocation.** One seeded, business-scoped key satisfies scoped auth. A
  `status` column would add revocation and multiple keys would add rotation, but neither earns its
  complexity for a single-tenant demo.

---
