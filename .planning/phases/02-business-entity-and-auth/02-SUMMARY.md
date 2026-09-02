# Phase 2 Summary: Business Entity & API Key Auth

## Accomplishments
- **Database Migrations & Deterministic Seeding**:
  - `0001_create_businesses.up.sql` / `down.sql`: Created `businesses` table (`id` UUID v4, `name`, `created_at`) and seeded default business `Acme Corp` (`id = '00000000-0000-0000-0000-000000000001'`).
  - `0002_create_api_keys.up.sql` / `down.sql`: Created `api_keys` table (`business_id`, `token_hash`, `token_prefix`, `name`, `created_at`) with indexes and seeded default test API key (`dodo_test_key_12345`).
- **Domain Models & Gaur Alignment (`minidodo-core`)**:
  - `Business` model deriving `sqlx::FromRow`, `Serialize`, `Deserialize`, and `utoipa::ToSchema`.
- **Database Actions (`minidodo-infra`)**:
  - `actions/businesses/get.rs`: Pure async function `get_by_id`.
  - `actions/apikeys/verify.rs`: Pure async function `verify_api_key`.
- **Authentication & 3-Tier API Endpoints (`minidodo-server`)**:
  - `middleware/auth.rs`: `AuthContext` extractor validating `Authorization: Bearer <token>` against the SHA-256 hashed database token.
  - `routes/v1/businesses/`: `GET /v1/businesses/me` returning the authenticated business in `JsonResponse<Business>`.
  - `BusinessesApiDoc` OpenAPI sub-docs mounted under root `ApiDoc`.
- **Live Verification**:
  - `GET /v1/businesses/me` with `Bearer dodo_test_key_12345` -> `200 OK` (`{"data":{"id":"00000000-0000-0000-0000-000000000001","name":"Acme Corp",...}}`).
  - `GET /v1/businesses/me` with invalid token -> `401 Unauthorized` (`AUTH_INVALID_KEY`).
  - `GET /v1/businesses/me` with missing token -> `401 Unauthorized` (`AUTH_UNAUTHORIZED`).
