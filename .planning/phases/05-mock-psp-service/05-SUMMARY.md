# Phase 5 Summary: Mock PSP Service

## Accomplishments
- **Workspace Integration & Architecture**:
  - Added `minidodo-psp` crate following the 3-tier structure (`src/lib.rs`, `src/serve.rs`, `src/http/server.rs`, `src/http/routes/v1/charges/`).
  - Added typed figment configuration in `minidodo-core/src/config/components/psp.rs` (`load_psp_config()` loading `MINIDODO_PSP_HOST` / `MINIDODO_PSP_PORT`).
  - Added `psp` subcommand to `minidodo-cli`.
- **Deduplication**:
  - Reused `ValidatedJson` extractor from `minidodo-core` across both server and psp crates.
- **Exact Token & Response Spec (per assignment doc)**:
  - `tok_success` / default: Returns `200 OK` with `{"status": "succeeded", "psp_ref": "<uuid>"}`.
  - `tok_card_declined`: Returns `400 Bad Request` with `{"status": "failed", "code": "card_declined"}`.
  - `tok_insufficient_funds`: Returns `400 Bad Request` with `{"status": "failed", "code": "insufficient_funds"}`.
  - `tok_timeout`: Simulates slow success (`sleep(30s)` -> `200 OK` with `{"status": "succeeded", "psp_ref": "<uuid>"}`).
  - `tok_network_error`: Returns `500 Internal Server Error` with `{"status": "failed", "code": "network_error"}`.
- **Docker Compose & Verification**:
  - Configured `psp` service in `docker-compose.yaml` (port `8080:3000`).
  - Live verified `GET /v1/health` and all charge scenarios (`tok_success`, `tok_card_declined`, `tok_insufficient_funds`, `tok_network_error`).
