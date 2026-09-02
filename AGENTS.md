# AGENTS.md

Guidance for AI agents working in this repository.

## What this project is

A minimal Invoice & Payment Service in Rust (Axum + sqlx + Postgres) with a mock PSP.
`DESIGN.md` is the source of truth for architecture, data model, state machine, and
failure handling. Read it before touching code. `AI_USAGE.md` records how AI was used and
is a graded deliverable, keep it honest and current.

## Skills to load

This repo ships opinionated skills under `.agents/skills/`. Load them when relevant:

- **`rust-standards`** — load before writing or scaffolding any Rust. Defines the workspace
  layout, the 3-tier route pattern, action-based infra, error handling, and SQL conventions
  this repo follows. All Rust code MUST conform to it.
- **`rust-code-review`** — load before reviewing Rust. The checklist this repo is graded
  against internally.
- **GSD workflow skills** (`gsd-plan-phase`, `gsd-execute-phase`, `gsd-verify-work`,
  `gsd-progress`, `gsd-code-review`, `gsd-debug`, etc.) — load when planning, executing, or
  verifying phases. Project planning lives in `.planning/`.

Use the `skill` tool with the skill name. Prefer retrieval from these skills over
pretrained assumptions about how this repo is structured.

## Non-negotiable rules

- **Money is integer cents.** `i64` / `bigint` everywhere in the money path. No floats. Ever.
- **Lowercase SQL.** All SQL keywords and statements lowercase (`select`, `insert into`,
  `for update skip locked`).
- **Explicit SQL, not ORM DSL.** Use `sqlx::query!` / `query_as!` in action files.
- **No long locks across network I/O.** Never hold a row lock or open transaction across the
  PSP HTTP call. Concurrency uses status-conditional `update` (see DESIGN.md §3).
- **No `unwrap()` / `expect()` in production paths.** Propagate with `?`.
- **Tenant scoping.** Every resource query filters by the authenticated `business_id`.
- **Secrets hashed, never logged.** API keys stored as SHA-256 hash with a prefix for lookup.
- **Pure async functions over trait objects.** Hexagonal-lite, no ports/adapters ceremony.

## Writing style (docs and comments)

- No em dashes.
- Plain, direct sentences.
- Docs describe what is being built and how, not brainstorm residue or rejected alternatives
  (the one exception is DESIGN.md §3, where the assignment asks for the concurrency choice vs
  alternatives).

## Workspace layout

Cargo workspace with five crates. See `rust-standards` for the full convention.

```
minidodo-core/     domain models, state machine, master error enum, error codes
minidodo-infra/    postgres actions + psp http client + webhook dispatcher
minidodo-server/   axum api (3-tier routes/v1/<resource>/), auth + validation middleware
minidodo-worker/   payment completer + webhook delivery (poll domain tables)
minidodo-psp/      mock payment processor
```

## Commands

- Build: `cargo build`
- Lint: `cargo clippy --all-targets -- -D warnings`
- Format: `cargo fmt`
- Full stack: `docker compose up` (app, database, mock PSP, no manual steps)

## Git

Do not commit unless explicitly asked. The user commits.
