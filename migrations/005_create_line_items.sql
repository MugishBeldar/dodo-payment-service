CREATE TABLE invoice_line_items (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    invoice_id          UUID NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
    description         TEXT NOT NULL,
    quantity            INT NOT NULL CHECK (quantity > 0),
    unit_amount_cents   BIGINT NOT NULL CHECK (unit_amount_cents > 0),
    line_total_cents    BIGINT GENERATED ALWAYS AS (quantity * unit_amount_cents) STORED
);

CREATE INDEX idx_line_items_invoice ON invoice_line_items(invoice_id);
