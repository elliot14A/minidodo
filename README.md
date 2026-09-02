# minidodo

A minimal Invoice & Payment Service in Rust. Businesses create invoices for customers,
customers pay them through a mock payment processor, and businesses receive signed webhooks
for payment state changes.

Built with Axum, sqlx, and Postgres. See `DESIGN.md` for the architecture, data model, state
machine, and failure handling.

## Requirements

- Docker and Docker Compose

## Run

```sh
docker compose up
```

This brings up the API server, Postgres, and the mock PSP with no manual steps. Migrations run
on startup.

## Curl examples

Set your API key once:

```sh
export API_KEY="<seeded key from startup logs>"
export BASE="http://localhost:8080"
```

Create a customer:

```sh
curl -s -X POST "$BASE/v1/customers" \
  -H "authorization: Bearer $API_KEY" \
  -H "content-type: application/json" \
  -d '{"name": "Ada Lovelace", "email": "ada@example.com"}'
```

Create an invoice (the server computes the total from line items):

```sh
curl -s -X POST "$BASE/v1/invoices" \
  -H "authorization: Bearer $API_KEY" \
  -H "content-type: application/json" \
  -d '{
    "customer_id": "<customer-id>",
    "due_date": "2026-12-31",
    "line_items": [
      {"description": "Consulting", "quantity": 2, "unit_amount_cents": 15000}
    ]
  }'
```

Pay an invoice (success):

```sh
curl -s -X POST "$BASE/v1/invoices/<invoice-id>/pay" \
  -H "authorization: Bearer $API_KEY" \
  -H "idempotency-key: $(uuidgen)" \
  -H "content-type: application/json" \
  -d '{"card_token": "tok_success"}'
```

Pay an invoice (failure):

```sh
curl -s -X POST "$BASE/v1/invoices/<invoice-id>/pay" \
  -H "authorization: Bearer $API_KEY" \
  -H "idempotency-key: $(uuidgen)" \
  -H "content-type: application/json" \
  -d '{"card_token": "tok_card_declined"}'
```

## API documentation

OpenAPI is served at `/docs` when the server is running.

## Demo Video

TODO: add link before submission.
