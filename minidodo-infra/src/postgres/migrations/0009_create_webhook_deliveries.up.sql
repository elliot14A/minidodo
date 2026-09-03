create type webhook_delivery_status as enum ('pending', 'delivered', 'failed');
create type webhook_event_type as enum ('invoice.paid', 'invoice.payment_failed');

create table if not exists webhook_deliveries (
    id uuid primary key default gen_random_uuid(),
    endpoint_id uuid not null references webhooks(id) on delete cascade,
    business_id uuid not null references businesses(id) on delete cascade,
    event_type webhook_event_type not null,
    payload jsonb not null,
    status webhook_delivery_status not null default 'pending',
    attempts int not null default 0,
    last_error text,
    last_attempt_at timestamptz,
    created_at timestamptz not null default current_timestamp
);

create index if not exists idx_webhook_deliveries_business_id on webhook_deliveries(business_id, created_at desc);
