CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE businesses (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO businesses (id, name)
VALUES ('00000000-0000-0000-0000-000000000001', 'Test Business Inc.');
