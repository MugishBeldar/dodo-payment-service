CREATE TYPE payment_status AS ENUM (
    'pending',
    'succeeded',
    'failed'
);

CREATE TABLE payment_attempts (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    invoice_id       UUID NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
    idempotency_key  TEXT NOT NULL UNIQUE,
    request_hash     TEXT NOT NULL,
    status           payment_status NOT NULL DEFAULT 'pending',
    psp_reference    TEXT,
    failure_code     TEXT,
    card_token       TEXT NOT NULL,
    amount_cents     BIGINT NOT NULL,
    response_json    JSONB,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_payment_attempts_invoice ON payment_attempts(invoice_id);
CREATE INDEX idx_payment_attempts_idempotency ON payment_attempts(idempotency_key);
