CREATE TABLE IF NOT EXISTS contract_events (
    id BIGSERIAL PRIMARY KEY,
    contract_name TEXT NOT NULL,
    contract_address TEXT NOT NULL,
    block_number TEXT NOT NULL,
    block_hash TEXT NOT NULL,
    block_timestamp TEXT NOT NULL,
    block_time TEXT NOT NULL,
    transaction_hash TEXT NOT NULL,
    transaction_index TEXT NOT NULL,
    log_index TEXT NOT NULL,
    event_name TEXT NOT NULL,
    event_signature TEXT NOT NULL,
    event_data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_contract_name ON contract_events (contract_name);
CREATE INDEX IF NOT EXISTS idx_contract_address ON contract_events (contract_address);
CREATE INDEX IF NOT EXISTS idx_contract_name_address ON contract_events (contract_name, contract_address);

CREATE INDEX IF NOT EXISTS idx_block_number ON contract_events (block_number);
CREATE INDEX IF NOT EXISTS idx_block_hash ON contract_events (block_hash);
CREATE INDEX IF NOT EXISTS idx_block_timestamp ON contract_events (block_timestamp);
CREATE INDEX IF NOT EXISTS idx_block_time ON contract_events (block_time);
CREATE INDEX IF NOT EXISTS idx_block_number_hash_timestamp ON contract_events (block_number, block_hash, block_timestamp);

CREATE INDEX IF NOT EXISTS idx_transaction_hash ON contract_events (transaction_hash);
CREATE INDEX IF NOT EXISTS idx_transaction_index ON contract_events (transaction_index);
CREATE INDEX IF NOT EXISTS idx_log_index ON contract_events (log_index);
CREATE INDEX IF NOT EXISTS idx_transaction_hash_index_log_index ON contract_events (transaction_hash, transaction_index, log_index);

CREATE INDEX IF NOT EXISTS idx_event_name ON contract_events (event_name);
CREATE INDEX IF NOT EXISTS idx_event_signature ON contract_events (event_signature);
CREATE INDEX IF NOT EXISTS idx_event_data_jsonb ON contract_events USING gin (event_data);
CREATE INDEX IF NOT EXISTS idx_event_name_signature ON contract_events (event_name, event_signature);

CREATE INDEX IF NOT EXISTS idx_contract_block_tx_event ON contract_events (contract_name, contract_address, block_number, block_hash, transaction_hash, event_name, event_signature);
