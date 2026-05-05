CREATE TYPE invoice_state AS ENUM (
    'draft',
    'open',
    'paid',
    'void',
    'uncollectible'
);

CREATE TABLE invoices (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    business_id         UUID NOT NULL REFERENCES businesses(id) ON DELETE CASCADE,
    customer_id         UUID NOT NULL REFERENCES customers(id),
    state               invoice_state NOT NULL DEFAULT 'draft',
    total_cents         BIGINT NOT NULL DEFAULT 0,
    due_date            DATE NOT NULL,
    description         TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_invoices_business ON invoices(business_id);
CREATE INDEX idx_invoices_customer ON invoices(customer_id);
CREATE INDEX idx_invoices_state ON invoices(business_id, state);
