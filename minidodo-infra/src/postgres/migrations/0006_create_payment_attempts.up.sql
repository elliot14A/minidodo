create type payment_status as enum ('pending', 'succeeded', 'failed');

create table if not exists payment_attempts (
    id uuid primary key default gen_random_uuid(),
    invoice_id uuid not null references invoices(id) on delete cascade,
    business_id uuid not null references businesses(id) on delete cascade,
    idempotency_key text not null,
    payload_hash text not null,
    card_token text not null,
    status payment_status not null default 'pending',
    psp_ref uuid,
    psp_error_code text,
    created_at timestamptz not null default current_timestamp,
    unique(business_id, idempotency_key)
);

create index if not exists idx_payment_attempts_invoice_id on payment_attempts(invoice_id);
create index if not exists idx_payment_attempts_business_id on payment_attempts(business_id);
