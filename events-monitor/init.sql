CREATE USER events_monitor WITH ENCRYPTED PASSWORD 'passwd' CREATEDB;

ALTER USER events_monitor CREATEDB;

CREATE DATABASE events_monitor_db OWNER events_monitor;

\c events_monitor_db events_monitor;

GRANT ALL PRIVILEGES ON DATABASE events_monitor_db TO events_monitor;
GRANT CONNECT, CREATE ON DATABASE events_monitor_db TO events_monitor;
GRANT CREATE ON SCHEMA public TO events_monitor;

ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO events_monitor;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO events_monitor;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON FUNCTIONS TO events_monitor;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TYPES TO events_monitor;

GRANT ALL ON ALL TABLES IN SCHEMA public TO events_monitor;
GRANT ALL ON ALL SEQUENCES IN SCHEMA public TO events_monitor;
GRANT ALL ON ALL FUNCTIONS IN SCHEMA public TO events_monitor;
-- GRANT ALL ON ALL TYPES IN SCHEMA public TO events_monitor;

GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO events_monitor;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO events_monitor;
GRANT ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA public TO events_monitor;
-- GRANT ALL PRIVILEGES ON ALL TYPES IN SCHEMA public TO events_monitor;

CREATE TABLE IF NOT EXISTS events_monitor_data (
    id BIGSERIAL PRIMARY KEY,
    contract_name TEXT NOT NULL,
    contract_address TEXT NOT NULL,
    implementation_name TEXT,
    implementation_address TEXT,
    chain_id TEXT NOT NULL,
    block_number TEXT NOT NULL,
    block_hash TEXT NOT NULL,
    block_timestamp TEXT NOT NULL,
    block_time TEXT NOT NULL,
    transaction_hash TEXT NOT NULL,
    transaction_sender TEXT NOT NULL,
    transaction_receiver TEXT NOT NULL,
    transaction_index TEXT NOT NULL,
    log_index TEXT NOT NULL,
    log_hash TEXT NOT NULL,
    event_name TEXT NOT NULL,
    event_signature TEXT NOT NULL,
    event_data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (chain_id, log_hash, event_name, event_signature)
);

CREATE INDEX IF NOT EXISTS idx_contract_name ON events_monitor_data (contract_name);
CREATE INDEX IF NOT EXISTS idx_contract_address ON events_monitor_data (contract_address);
CREATE INDEX IF NOT EXISTS idx_impl_name ON events_monitor_data (implementation_name);
CREATE INDEX IF NOT EXISTS idx_impl_address ON events_monitor_data (implementation_address);
CREATE INDEX IF NOT EXISTS idx_contract_impl_name_address ON events_monitor_data (contract_name, contract_address, implementation_name, implementation_address);

CREATE INDEX IF NOT EXISTS idx_chain_id ON events_monitor_data (chain_id);
CREATE INDEX IF NOT EXISTS idx_block_number ON events_monitor_data (block_number);
CREATE INDEX IF NOT EXISTS idx_block_hash ON events_monitor_data (block_hash);
CREATE INDEX IF NOT EXISTS idx_block_timestamp ON events_monitor_data (block_timestamp);
CREATE INDEX IF NOT EXISTS idx_block_time ON events_monitor_data (block_time);
CREATE INDEX IF NOT EXISTS idx_chain_id_block_number_hash_timestamp ON events_monitor_data (chain_id, block_number, block_hash, block_timestamp);

CREATE INDEX IF NOT EXISTS idx_transaction_hash ON events_monitor_data (transaction_hash);
CREATE INDEX IF NOT EXISTS idx_transaction_sender ON events_monitor_data (transaction_sender);
CREATE INDEX IF NOT EXISTS idx_transaction_receiver ON events_monitor_data (transaction_receiver);
CREATE INDEX IF NOT EXISTS idx_transaction_index ON events_monitor_data (transaction_index);
CREATE INDEX IF NOT EXISTS idx_log_index ON events_monitor_data (log_index);
CREATE INDEX IF NOT EXISTS idx_log_hash ON events_monitor_data (log_hash);
CREATE INDEX IF NOT EXISTS idx_transaction_hash_sender_index_log_index_hash ON events_monitor_data (transaction_hash, transaction_sender, transaction_receiver, transaction_index, log_index, log_hash);

CREATE INDEX IF NOT EXISTS idx_event_name ON events_monitor_data (event_name);
CREATE INDEX IF NOT EXISTS idx_event_signature ON events_monitor_data (event_signature);
CREATE INDEX IF NOT EXISTS idx_event_data_jsonb ON events_monitor_data USING gin (event_data);
CREATE INDEX IF NOT EXISTS idx_event_name_signature ON events_monitor_data (event_name, event_signature);

CREATE INDEX IF NOT EXISTS idx_contract_chain_block_tx_log_event ON events_monitor_data (contract_name, contract_address, implementation_name, implementation_address, chain_id, block_number, block_hash, block_timestamp, transaction_hash, transaction_sender, transaction_receiver, transaction_index, log_index, log_hash, event_name, event_signature);
