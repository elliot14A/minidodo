create table if not exists api_keys (
    id uuid primary key default gen_random_uuid(),
    business_id uuid not null references businesses(id) on delete cascade,
    token_hash bytea not null unique,
    token_prefix varchar(32) not null,
    name varchar(255) not null default 'default',
    created_at timestamptz not null default current_timestamp
);

create index if not exists idx_api_keys_business_id on api_keys(business_id);
create index if not exists idx_api_keys_token_prefix on api_keys(token_prefix);

insert into api_keys (id, business_id, token_hash, token_prefix, name, created_at)
values (
    '00000000-0000-0000-0000-000000000002',
    '00000000-0000-0000-0000-000000000001',
    decode('8696e7364a6dc2bbf59bd9a49ab245a0434e286918c046a36e1a8642525daf0a', 'hex'),
    'dodo_test',
    'Default Dev Key',
    current_timestamp
)
on conflict (token_hash) do nothing;
