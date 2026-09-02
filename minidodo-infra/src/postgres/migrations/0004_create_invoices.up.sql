create type invoice_state as enum ('draft', 'open', 'processing', 'paid', 'void', 'uncollectible');

create table if not exists invoices (
    id uuid primary key default gen_random_uuid(),
    business_id uuid not null references businesses(id) on delete cascade,
    customer_id uuid not null references customers(id) on delete restrict,
    state invoice_state not null default 'draft',
    total_cents bigint not null,
    due_date date not null,
    created_at timestamptz not null default current_timestamp
);

create index if not exists idx_invoices_business_id_state on invoices(business_id, state);
create index if not exists idx_invoices_customer_id on invoices(customer_id);
create index if not exists idx_invoices_business_id_created_at on invoices(business_id, created_at desc);
