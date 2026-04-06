CREATE TABLE IF NOT EXISTS contracts (
    address     TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    abi         JSONB NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
