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

The default development API key seeded during migration for `Acme Corp` is:

```sh
export API_KEY="dodo_test_key_12345"
export BASE="http://localhost:3000"
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

The OpenAPI 3.1 spec is committed at [`openapi/openapi.json`](openapi/openapi.json).

When the server is running it is also served live:

- Swagger UI: `http://localhost:3000/swagger-ui`
- Raw spec: `http://localhost:3000/api-docs/openapi.json`

Regenerate the committed copy from a running stack with:

```
curl -s http://localhost:3000/api-docs/openapi.json | python3 -m json.tool > openapi/openapi.json
```

## Demo Video

TODO: add link before submission.
