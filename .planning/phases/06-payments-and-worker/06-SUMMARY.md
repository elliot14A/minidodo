# Phase 6 Summary: Payment Processing, Worker & Crash Recovery

## Accomplishments
- **Database Migrations**:
  - `0006_create_payment_attempts.up.sql`: Created `payment_status` enum (`pending`, `succeeded`, `failed`) and `payment_attempts` table.
  - `0007_create_idempotency_keys.up.sql`: Created `recovery_point` enum (`charge_pending`, `finished`) and `idempotency_keys` table with partial recovery index `where recovery_point <> 'finished'`.
- **Domain Models & Gaur Infrastructure**:
  - Domain types `PaymentStatus`, `PaymentAttempt`, `RecoveryPoint`, `IdempotencyRecord` in `minidodo-core`.
  - Database actions in `minidodo-infra/src/postgres/actions/payments/`: `create.rs`, `claim.rs`, `idempotency.rs`, `settle.rs`, `recover.rs`, `notify.rs`.
  - Dedicated external service infra adapter for PSP in `minidodo-infra/src/psp/` (`charge.rs`, `error.rs`, `mod.rs`).
- **3-Tier Pay Endpoint**:
  - `POST /v1/invoices/{id}/pay` accepting `Idempotency-Key` header and `{card_token}` body.
  - Status-conditional database claim (`WHERE state = 'open' -> state = 'processing'`). Losers of concurrent requests or non-open invoices receive clean `409 Conflict`.
  - Same key + different payload returns `422 Unprocessable Entity`.
  - Same key + same payload replays stored terminal response.
  - Fires PostgreSQL `pg_notify('payments', attempt_id)` and returns `202 Accepted` immediately without blocking on network I/O.
- **Worker Service & Crash Recovery**:
  - Added `minidodo-worker` crate running fast-path notification listener (`LISTEN payments`) and background recovery sweep (30s interval for stale `charge_pending` claims).
  - Calls PSP with deterministically derived key `invsvc-<attempt_id>`.
  - Settles invoice states (`paid` on success, reverts to `open` on failure) and updates `idempotency_keys` to `finished`.
- **Live Verification**:
  - Docker compose stack running `postgres`, `migrate`, `psp`, `server`, and `worker`.
  - Tested successful charge (`tok_success` -> `202 Accepted` -> settled to `paid` in ~1s -> replay returns `200 OK`).
  - Tested failed charge (`tok_card_declined` -> `202 Accepted` -> reverted to `open` -> replay returns `400 Bad Request`).
  - `cargo clippy --all-targets -- -D warnings` passing cleanly across all crates.
