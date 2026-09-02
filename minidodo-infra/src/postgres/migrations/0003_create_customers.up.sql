create table if not exists customers (
    id uuid primary key default gen_random_uuid(),
    business_id uuid not null references businesses(id) on delete cascade,
    name varchar(255) not null,
    email varchar(255) not null,
    created_at timestamptz not null default current_timestamp
);

create index if not exists idx_customers_business_id_created_at on customers(business_id, created_at desc);
