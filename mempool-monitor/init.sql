CREATE USER mempool_monitor WITH ENCRYPTED PASSWORD 'passwd' CREATEDB;

ALTER USER mempool_monitor CREATEDB;

CREATE DATABASE mempool_monitor_db OWNER mempool_monitor;

\c mempool_monitor_db mempool_monitor;

GRANT ALL PRIVILEGES ON DATABASE mempool_monitor_db TO mempool_monitor;
GRANT CONNECT, CREATE ON DATABASE mempool_monitor_db TO mempool_monitor;
GRANT CREATE ON SCHEMA public TO mempool_monitor;

ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO mempool_monitor;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO mempool_monitor;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON FUNCTIONS TO mempool_monitor;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TYPES TO mempool_monitor;

GRANT ALL ON ALL TABLES IN SCHEMA public TO mempool_monitor;
GRANT ALL ON ALL SEQUENCES IN SCHEMA public TO mempool_monitor;
GRANT ALL ON ALL FUNCTIONS IN SCHEMA public TO mempool_monitor;
-- GRANT ALL ON ALL TYPES IN SCHEMA public TO mempool_monitor;

GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO mempool_monitor;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO mempool_monitor;
GRANT ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA public TO mempool_monitor;
-- GRANT ALL PRIVILEGES ON ALL TYPES IN SCHEMA public TO mempool_monitor;

-- Mempool transactions table
CREATE TABLE IF NOT EXISTS mempool_monitor_data (
    id BIGSERIAL PRIMARY KEY,
    chain_id TEXT NOT NULL,
    transaction_hash TEXT NOT NULL UNIQUE,
    transaction_sender TEXT NOT NULL,
    transaction_receiver TEXT,
    nonce TEXT NOT NULL,
    value TEXT NOT NULL,
    gas_limit TEXT NOT NULL,
    gas_price TEXT,
    max_fee_per_gas TEXT,
    max_priority_fee_per_gas TEXT,
    input_data TEXT NOT NULL,
    transaction_type TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (chain_id, transaction_hash, nonce)
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_mempool_tx_chain_id ON mempool_monitor_data(chain_id);
CREATE INDEX IF NOT EXISTS idx_mempool_tx_hash ON mempool_monitor_data(transaction_hash);
CREATE INDEX IF NOT EXISTS idx_mempool_tx_sender ON mempool_monitor_data(transaction_sender);
CREATE INDEX IF NOT EXISTS idx_mempool_tx_receiver ON mempool_monitor_data(transaction_receiver);
CREATE INDEX IF NOT EXISTS idx_mempool_tx_nonce ON mempool_monitor_data(nonce);
CREATE INDEX IF NOT EXISTS idx_mempool_tx_value ON mempool_monitor_data(value);
CREATE INDEX IF NOT EXISTS idx_mempool_tx_gas_limit ON mempool_monitor_data(gas_limit);
CREATE INDEX IF NOT EXISTS idx_mempool_tx_gas_price ON mempool_monitor_data(gas_price);
CREATE INDEX IF NOT EXISTS idx_mempool_tx_max_fee_per_gas ON mempool_monitor_data(max_fee_per_gas);
CREATE INDEX IF NOT EXISTS idx_mempool_tx_max_priority_fee_per_gas ON mempool_monitor_data(max_priority_fee_per_gas);
CREATE INDEX IF NOT EXISTS idx_mempool_tx_input_data ON mempool_monitor_data(input_data);
CREATE INDEX IF NOT EXISTS idx_mempool_tx_type ON mempool_monitor_data(transaction_type);
CREATE INDEX IF NOT EXISTS idx_mempool_tx_timestamp ON mempool_monitor_data(timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_mempool_tx_chain_id_hash_nonce ON mempool_monitor_data(chain_id, transaction_hash, nonce);
CREATE INDEX IF NOT EXISTS idx_mempool_tx_chain_id_hash_sender_receiver_nonce ON mempool_monitor_data(chain_id, transaction_hash, transaction_sender, transaction_receiver, nonce);
CREATE INDEX IF NOT EXISTS idx_mempool_tx_chain_id_hash_sender_receiver_nonce_value_input_data_tx_type ON mempool_monitor_data(chain_id, transaction_hash, transaction_sender, transaction_receiver, nonce, value, input_data, transaction_type);
CREATE INDEX IF NOT EXISTS idx_mempool_tx_chain_id_hash_sender_receiver_nonce_value_input_data_tx_type_gas_fee_timestamp ON mempool_monitor_data(chain_id, transaction_hash, transaction_sender, transaction_receiver, nonce, value, input_data, transaction_type, gas_limit, gas_price, max_fee_per_gas, max_priority_fee_per_gas, timestamp);
