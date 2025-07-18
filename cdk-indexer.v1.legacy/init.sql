CREATE TABLE IF NOT EXISTS events (
    id SERIAL PRIMARY KEY,
    contract_address TEXT NOT NULL,
    event_name TEXT NOT NULL,
    parameters JSONB NOT NULL,
    inserted_at TIMESTAMP DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_contract_address ON events (contract_address);
CREATE INDEX IF NOT EXISTS idx_event_name ON events (event_name);
CREATE INDEX IF NOT EXISTS idx_parameters_jsonb ON events USING gin (parameters);
