-- AWS RDS PostgreSQL Schema for Mempool Monitor
-- This schema is compatible with AWS RDS PostgreSQL instances

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
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (chain_id, transaction_hash, nonce) -- Add UNIQUE constraint for deduplication
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_mempool_tx_chain_id ON mempool_monitor_data(chain_id);
CREATE INDEX IF NOT EXISTS idx_mempool_tx_hash ON mempool_monitor_data(transaction_hash); -- Primary deduplication index
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

-- Comprehensive compound indexes for complex queries
CREATE INDEX IF NOT EXISTS idx_mempool_tx_chain_id_hash_nonce ON mempool_monitor_data(chain_id, transaction_hash, nonce);
CREATE INDEX IF NOT EXISTS idx_mempool_tx_chain_id_hash_sender_receiver_nonce ON mempool_monitor_data(chain_id, transaction_hash, transaction_sender, transaction_receiver, nonce);
CREATE INDEX IF NOT EXISTS idx_mempool_tx_chain_id_hash_sender_receiver_nonce_value_input_data_tx_type ON mempool_monitor_data(chain_id, transaction_hash, transaction_sender, transaction_receiver, nonce, value, input_data, transaction_type);
CREATE INDEX IF NOT EXISTS idx_mempool_tx_chain_id_hash_sender_receiver_nonce_value_input_data_tx_type_gas_fee_timestamp ON mempool_monitor_data(chain_id, transaction_hash, transaction_sender, transaction_receiver, nonce, value, input_data, transaction_type, gas_limit, gas_price, max_fee_per_gas, max_priority_fee_per_gas, timestamp);

CREATE INDEX IF NOT EXISTS idx_mempool_created_at ON mempool_monitor_data(created_at);
CREATE INDEX IF NOT EXISTS idx_mempool_updated_at ON mempool_monitor_data(updated_at);

-- Create a function to update the updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Create trigger to automatically update updated_at
DROP TRIGGER IF EXISTS update_mempool_monitor_data_updated_at ON mempool_monitor_data;
CREATE TRIGGER update_mempool_monitor_data_updated_at
    BEFORE UPDATE ON mempool_monitor_data
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
