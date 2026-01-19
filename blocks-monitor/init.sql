CREATE USER blocks_monitor WITH ENCRYPTED PASSWORD 'passwd' CREATEDB;

ALTER USER blocks_monitor CREATEDB;

CREATE DATABASE blocks_monitor_db OWNER blocks_monitor;

\c blocks_monitor_db blocks_monitor;

GRANT ALL PRIVILEGES ON DATABASE blocks_monitor_db TO blocks_monitor;
GRANT CONNECT, CREATE ON DATABASE blocks_monitor_db TO blocks_monitor;
GRANT CREATE ON SCHEMA public TO blocks_monitor;

ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO blocks_monitor;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO blocks_monitor;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON FUNCTIONS TO blocks_monitor;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TYPES TO blocks_monitor;

GRANT ALL ON ALL TABLES IN SCHEMA public TO blocks_monitor;
GRANT ALL ON ALL SEQUENCES IN SCHEMA public TO blocks_monitor;
GRANT ALL ON ALL FUNCTIONS IN SCHEMA public TO blocks_monitor;
-- GRANT ALL ON ALL TYPES IN SCHEMA public TO blocks_monitor;

GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO blocks_monitor;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO blocks_monitor;
GRANT ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA public TO blocks_monitor;
-- GRANT ALL PRIVILEGES ON ALL TYPES IN SCHEMA public TO blocks_monitor;

CREATE TABLE IF NOT EXISTS blocks_monitor_data (
    id BIGSERIAL PRIMARY KEY,
    chain_id TEXT NOT NULL,
    block_number TEXT NOT NULL,
    block_hash TEXT NOT NULL,
    block_timestamp TEXT NOT NULL,
    block_time TEXT NOT NULL,
    parent_hash TEXT NOT NULL,
    gas_used TEXT NOT NULL,
    gas_limit TEXT NOT NULL,
    transactions JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (chain_id, block_number, block_hash)
);

CREATE INDEX IF NOT EXISTS idx_blocks_chain_id ON blocks_monitor_data(chain_id);
CREATE INDEX IF NOT EXISTS idx_blocks_block_number ON blocks_monitor_data(block_number);
CREATE INDEX IF NOT EXISTS idx_blocks_block_hash ON blocks_monitor_data(block_hash);
CREATE INDEX IF NOT EXISTS idx_blocks_block_timestamp ON blocks_monitor_data(block_timestamp);
CREATE INDEX IF NOT EXISTS idx_blocks_block_time ON blocks_monitor_data(block_time);
CREATE INDEX IF NOT EXISTS idx_blocks_parent_hash ON blocks_monitor_data(parent_hash);
CREATE INDEX IF NOT EXISTS idx_blocks_gas_used ON blocks_monitor_data(gas_used);
CREATE INDEX IF NOT EXISTS idx_blocks_gas_limit ON blocks_monitor_data(gas_limit);
CREATE INDEX IF NOT EXISTS idx_blocks_tx_data_jsonb ON blocks_monitor_data USING gin (transactions);
CREATE INDEX IF NOT EXISTS idx_blocks_chain_id_block_number_hash ON blocks_monitor_data(chain_id, block_number, block_hash);
CREATE INDEX IF NOT EXISTS idx_blocks_chain_id_block_number_hash_timestamp ON blocks_monitor_data(chain_id, block_number, block_hash, block_timestamp);
CREATE INDEX IF NOT EXISTS idx_blocks_chain_id_block_number_hash_timestamp_parent_hash ON blocks_monitor_data(chain_id, block_number, block_hash, block_timestamp, parent_hash);
CREATE INDEX IF NOT EXISTS idx_blocks_chain_id_block_number_hash_timestamp_parent_hash_gas_used_limit ON blocks_monitor_data(chain_id, block_number, block_hash, block_timestamp, parent_hash, gas_used, gas_limit);
