create table if not exists webhooks (
    id uuid primary key default gen_random_uuid(),
    business_id uuid not null references businesses(id) on delete cascade,
    url text not null,
    signing_secret text not null,
    active boolean not null default true,
    created_at timestamptz not null default current_timestamp
);

create index if not exists idx_webhooks_business_id on webhooks(business_id);

insert into webhooks (id, business_id, url, signing_secret, active)
values (
    '00000000-0000-0000-0000-000000000003',
    '00000000-0000-0000-0000-000000000001',
    'http://psp:3000/webhooks/sink',
    'whsec_test_secret_12345',
    true
)
on conflict (id) do nothing;
