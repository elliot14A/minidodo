# Phase 7 Summary: Webhook Endpoints & Deliveries

## Accomplishments
- **Database Migrations**:
  - `0008_create_webhooks.up.sql`: Created `webhooks` table and seeded default endpoint `http://psp:3000/webhooks/sink` with secret `whsec_test_secret_12345`.
  - `0009_create_webhook_deliveries.up.sql`: Created `webhook_delivery_status` enum (`pending`, `delivered`, `failed`), `webhook_event_type` enum (`invoice.paid`, `invoice.payment_failed`), and `webhook_deliveries` outbox table with index on `(business_id, created_at desc)`.
- **Domain Models & Cryptographic Verification**:
  - Added domain types in `minidodo-core/src/models/webhook.rs`: `WebhookDeliveryStatus`, `WebhookEventType`, `WebhookEndpoint`, `WebhookDelivery`, and `WebhookEventPayload`.
  - Added HMAC-SHA256 signing (`sign_payload`) and constant-time verification helper (`verify_signature`).
  - Added saturating integer exponential backoff helper (`backoff_secs`) with `WEBHOOK_MAX_ATTEMPTS = 5`.
- **Infrastructure Service & Database Actions**:
  - Dedicated `minidodo-infra/src/webhooks/` service exposing pure async `deliver()` with headers `x-webhook-signature` and `x-webhook-timestamp`.
  - Actions under `minidodo-infra/src/postgres/actions/webhooks/`: `stage.rs`, `notify.rs`, `claim.rs` (using `for update skip locked`), `mark_delivered.rs`, `mark_failed.rs`.
  - Transactional Outbox Staging: `settle_success` and `settle_failure` insert pending `webhook_deliveries` rows inside the payment settlement database transaction.
- **Worker Webhook Listener & Mock PSP Verifier**:
  - Added `webhook_listener` in `minidodo-worker` listening to `webhooks` notification channel, claiming pending rows, and delegating delivery to `minidodo_infra::webhooks::deliver`.
  - Added `POST /webhooks/sink` route in `minidodo-psp` to verify the HMAC signature and timestamp headers.
- **Live Verification**:
  - Tested `invoice.paid` delivery on `tok_success` (status updated to `delivered`, verified in PSP sink logs).
  - Tested `invoice.payment_failed` delivery on `tok_card_declined` (status updated to `delivered`).
  - Verified no `reqwest` dependency in `minidodo-worker` (all outbound HTTP clients isolated to `minidodo-infra`).
  - `cargo clippy --all-targets -- -D warnings` passing cleanly.
