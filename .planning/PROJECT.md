# minidodo

## What This Is

A minimal, robust, and audit-friendly invoice and payment service in Rust (Axum + sqlx + Postgres). Businesses create invoices for customers, customers pay them via a mock PSP, and businesses receive signed webhooks for payment state changes, backed by a domain-table driven outbox and crash recovery sweep.

## Core Value

Absolute financial correctness, double-charge prevention, and robust handling of network timeouts/crashes using database-level status-conditional claims, domain-table recovery, and deterministic PSP idempotency.

## Architecture & Style Rules

- **Resource-Driven Vertical Slices**: Each entity is developed end-to-end (migration -> domain model -> infra actions -> route handlers -> openapi doc).
- **Hexagonal Architecture with Pure Functions**: Ports & adapters implemented as pure async functions taking connection pools and clients directly (no heavy traits).
- **Lowercase SQL**: All SQL statements, clauses, and keywords written strictly in lowercase.
- **Dedicated Infra Subsystems**: Each service (`postgres`, `psp`, `webhook`) has its own directory with dedicated `error.rs` mapping to `MinidodoError`.
- **Gaur Route Hierarchy**: Resource routes use `routes.rs` (mounting router), `mod.rs` (re-exports & utoipa OpenApi sub-doc), and granular `<action>.rs` handlers.

## Requirements

### Active

- [ ] Resource Slice 1: Workspace & Core Error/Context Infrastructure
- [ ] Resource Slice 2: Business Entity (migration, model, actions, API endpoints)
- [ ] Resource Slice 3: API Key & Authentication Middleware (migration, hashing, actions, `AuthContext` extractor)
- [ ] Resource Slice 4: Customer Entity (migration, model, actions, API endpoints)
- [ ] Resource Slice 5: Invoice & Line Items Entity (migration, model, status-conditional actions, lifecycle endpoints: draft, finalize, void, list)
- [ ] Resource Slice 6: Mock PSP Service (token matrix: `tok_visa`, `tok_card_declined`, `tok_insufficient_funds`, `tok_timeout`, `tok_network_error`, idempotency replaying)
- [ ] Resource Slice 7: Webhook Endpoints & Deliveries (migration, models, signing, delivery infra actions, worker sweep)
- [ ] Resource Slice 8: Payment Processing, Concurrency & Crash Recovery (`POST /invoices/{id}/pay` atomic claim, `payment_attempts`, `idempotency_keys` recovery loop)
- [ ] Resource Slice 9: Concurrency & Failure Verification Tests (N concurrent claims, idempotency replay, timeout/network failure recovery)
- [ ] Resource Slice 10: Docker Compose & Operational Packaging (`docker compose up`, OpenAPI spec export, README)

### Out of Scope

- Generic queue/broker (`jobs` table, NATS/RabbitMQ) — background work is driven off domain tables (`idempotency_keys`, `webhook_deliveries`)
- Subscriptions, recurring billing, plans, proration
- Refunds or partial payments
- Multi-currency / FX — USD integer minor units only
- Tax calculation
- Frontend UI — Backend API only
- Email sending — logged as "would send"

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Status-conditional atomic `update` | Prevents holding row locks across slow PSP HTTP boundaries while guaranteeing single winner | — Pending |
| Domain-table recovery (`idempotency_keys`, `webhook_deliveries`) | Avoids unnecessary generic job queues; drives crash recovery directly off domain state | — Pending |
| Deterministic PSP idempotency key (`invsvc-<attempt_id>`) | Enables crash recovery sweep without risking double charges | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

---
*Last updated: 2026-09-02 after resource-driven alignment*
