CREATE TABLE contract_events (
    id SERIAL PRIMARY KEY,
    contract_name TEXT NOT NULL,
    contract_address TEXT NOT NULL,
    transaction_hash TEXT NOT NULL,
    block_number BIGINT NOT NULL,
    params JSONB NOT NULL,
    inserted_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_contract_name ON contract_events (contract_name);
CREATE INDEX idx_contract_address ON contract_events (contract_address);
CREATE INDEX idx_transaction_hash ON contract_events (transaction_hash);
CREATE INDEX idx_block_number ON contract_events (block_number);

CREATE INDEX idx_params_jsonb ON contract_events USING gin (params);
