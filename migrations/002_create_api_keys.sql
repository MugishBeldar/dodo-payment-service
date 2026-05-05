CREATE TABLE api_keys (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    business_id  UUID NOT NULL REFERENCES businesses(id) ON DELETE CASCADE,
    key_hash     TEXT NOT NULL UNIQUE,
    key_prefix   TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at   TIMESTAMPTZ
);

CREATE INDEX idx_api_keys_hash ON api_keys(key_hash);
CREATE INDEX idx_api_keys_business ON api_keys(business_id);

-- sha256("dp_test_supersecretkey123")
INSERT INTO api_keys (business_id, key_hash, key_prefix)
VALUES (
    '00000000-0000-0000-0000-000000000001',
    '207d58bbf10dd136cefc6b708e30e897ab7f183780a5304ddf5e15dd815a53c1',
    'dp_test_'
);
