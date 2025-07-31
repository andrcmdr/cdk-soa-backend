CREATE TABLE IF NOT EXISTS contract_events (
    id SERIAL PRIMARY KEY,
    contract_name TEXT NOT NULL,
    contract_address TEXT NOT NULL,
    block_number BIGINT NOT NULL,
    transaction_hash TEXT NOT NULL,
    event_name TEXT NOT NULL,
    event_data JSONB NOT NULL,
    inserted_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_contract_name ON contract_events (contract_name);
CREATE INDEX IF NOT EXISTS idx_contract_address ON contract_events (contract_address);
CREATE INDEX IF NOT EXISTS idx_block_number ON contract_events (block_number);
CREATE INDEX IF NOT EXISTS idx_transaction_hash ON contract_events (transaction_hash);
CREATE INDEX IF NOT EXISTS idx_event_name ON contract_events (event_name);
CREATE INDEX IF NOT EXISTS idx_event_data_jsonb ON contract_events USING gin (event_data);
