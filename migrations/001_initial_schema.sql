-- migrations/001_initial_schema.sql

CREATE TABLE IF NOT EXISTS blocks (
    number          BIGINT PRIMARY KEY,
    hash            TEXT NOT NULL UNIQUE,
    parent_hash     TEXT NOT NULL,
    timestamp       BIGINT NOT NULL,
    indexed_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS transactions (
    hash                TEXT PRIMARY KEY,
    block_number        BIGINT NOT NULL REFERENCES blocks(number),
    from_address        TEXT NOT NULL,
    to_address          TEXT,
    value               TEXT NOT NULL,
    input               TEXT NOT NULL,
    transaction_index   BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS logs (
    id                  BIGSERIAL PRIMARY KEY,
    block_number        BIGINT NOT NULL REFERENCES blocks(number),
    block_hash          TEXT NOT NULL,
    transaction_hash    TEXT NOT NULL,
    transaction_index   BIGINT NOT NULL,
    log_index           BIGINT NOT NULL,
    address             TEXT NOT NULL,
    topics              TEXT[] NOT NULL,
    data                TEXT NOT NULL,
    UNIQUE(transaction_hash, log_index)
);

-- The indexer reads this on startup to know where to resume.
-- A single row, updated atomically with each block write.
CREATE TABLE IF NOT EXISTS indexer_state (
    id                  INT PRIMARY KEY DEFAULT 1,
    last_indexed_block  BIGINT NOT NULL,
    CHECK (id = 1)  -- enforces single row
);
