CREATE TABLE IF NOT EXISTS decoded_events (
    id                  BIGSERIAL PRIMARY KEY,
    contract_address    TEXT NOT NULL,
    contract_name       TEXT NOT NULL,
    event_name          TEXT NOT NULL,
    block_number        BIGINT NOT NULL,
    transaction_hash    TEXT NOT NULL,
    log_index           BIGINT NOT NULL,
    parameters          JSONB NOT NULL,
    UNIQUE(transaction_hash, log_index)
);

CREATE INDEX IF NOT EXISTS idx_decoded_events_contract 
    ON decoded_events(contract_address);

CREATE INDEX IF NOT EXISTS idx_decoded_events_block 
    ON decoded_events(block_number);

CREATE INDEX IF NOT EXISTS idx_decoded_events_event_name 
    ON decoded_events(event_name);
