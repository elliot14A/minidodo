# Integration tests

End to end tests that run against a live `docker compose up` stack. They cover every
must-have from the assignment: API key auth, customers, invoices and the state machine,
idempotent payments, PSP failure handling, concurrency with no double charge, and signed
webhook delivery.

The suite is a standalone crate under `tests/integration/`, excluded from the workspace so
its rustls-based dependencies do not affect the main build. It links without OpenSSL, so it
runs directly on the host.

It mirrors the codebase layout: a `common/` module of shared helpers (one file per concern)
and a `suite/` module with one file per resource, each aggregated by a `mod.rs`.

```
tests/integration/
  main.rs                entry, declares the common and suite modules
  common/
    mod.rs               re-exports the helpers
    constants.rs         hardcoded urls, seeded ids, api key, webhook secret
    http.rs              reqwest client, get/post/patch/pay helpers, Resp
    db.rs                sqlx pool and attempt/delivery assertions
    fixtures.rs          create_invoice, finalize, state polling
  suite/
    mod.rs               declares the test modules
    auth.rs
    customers.rs
    invoices.rs
    payments.rs
    psp.rs
    webhooks.rs
```

## Run

Bring the stack up first, then run the suite from the repo root:

```
docker compose up -d
cargo test --manifest-path tests/integration/Cargo.toml
```

The `tok_timeout` test waits for the 30 second slow PSP response, so a full run takes about
30 to 40 seconds.

## What it checks

- **`suite::auth`**: missing header, invalid key, and valid key resolving the seeded business.
- **`suite::customers`**: create, get, list with pagination, 404, tenant-scoped 400.
- **`suite::invoices`**: server-computed total, line items, foreign-customer 404, valid and
  rejected state transitions, `paid` refused as a PATCH target, list by state.
- **`suite::payments`**: 202 non-blocking, required idempotency key, idempotent replay
  returning the stored result with no new attempt, same-key-different-body 422, already-paid
  409, unknown invoice 404, declined and network error returning the invoice to open,
  `tok_timeout` not blocking the API, and 10 concurrent `/pay` charging at most once
  (Postgres confirms a single attempt).
- **`suite::psp`**: the full mock token matrix.
- **`suite::webhooks`**: sink signature verification (valid and invalid), and `invoice.paid`
  / `invoice.payment_failed` deliveries reaching `delivered`.

Assertions that need ground truth the API does not expose (attempt counts, delivery status)
query Postgres directly at `postgres://postgres:postgres@localhost:5432/minidodo`.
