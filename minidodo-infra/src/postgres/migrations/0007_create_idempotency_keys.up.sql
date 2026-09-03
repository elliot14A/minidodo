create type recovery_point as enum ('charge_pending', 'finished');

create table if not exists idempotency_keys (
    business_id uuid not null references businesses(id) on delete cascade,
    idempotency_key text not null,
    payload_hash text not null,
    recovery_point recovery_point not null default 'charge_pending',
    locked_at timestamptz not null default current_timestamp,
    last_run_at timestamptz,
    response_code int,
    response_body jsonb,
    created_at timestamptz not null default current_timestamp,
    primary key (business_id, idempotency_key)
);

create index if not exists idx_idempotency_keys_recovery on idempotency_keys(recovery_point, locked_at) where recovery_point <> 'finished';
