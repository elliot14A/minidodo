create table if not exists line_items (
    id uuid primary key default gen_random_uuid(),
    invoice_id uuid not null references invoices(id) on delete cascade,
    description text not null,
    quantity int not null check (quantity > 0),
    unit_amount_cents bigint not null check (unit_amount_cents >= 0),
    created_at timestamptz not null default current_timestamp
);

create index if not exists idx_line_items_invoice_id on line_items(invoice_id);
