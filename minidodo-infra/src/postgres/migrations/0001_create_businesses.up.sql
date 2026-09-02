create table if not exists businesses (
    id uuid primary key default gen_random_uuid(),
    name varchar(255) not null,
    created_at timestamptz not null default current_timestamp
);

insert into businesses (id, name, created_at)
values ('00000000-0000-0000-0000-000000000001', 'Acme Corp', current_timestamp)
on conflict (id) do nothing;
