-- AWS RDS PostgreSQL Schema for Blocks Monitor
-- This schema is compatible with AWS RDS PostgreSQL instances

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
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (chain_id, block_number, block_hash) -- Add UNIQUE constraint for deduplication
);

-- Create indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_blocks_chain_id ON blocks_monitor_data(chain_id);
CREATE INDEX IF NOT EXISTS idx_blocks_block_number ON blocks_monitor_data(block_number);
CREATE INDEX IF NOT EXISTS idx_blocks_block_hash ON blocks_monitor_data(block_hash); -- Primary deduplication index
CREATE INDEX IF NOT EXISTS idx_blocks_block_timestamp ON blocks_monitor_data(block_timestamp);
CREATE INDEX IF NOT EXISTS idx_blocks_block_time ON blocks_monitor_data(block_time);
CREATE INDEX IF NOT EXISTS idx_blocks_parent_hash ON blocks_monitor_data(parent_hash);
CREATE INDEX IF NOT EXISTS idx_blocks_gas_used ON blocks_monitor_data(gas_used);
CREATE INDEX IF NOT EXISTS idx_blocks_gas_limit ON blocks_monitor_data(gas_limit);
CREATE INDEX IF NOT EXISTS idx_blocks_tx_data_jsonb ON blocks_monitor_data USING gin (transactions);

-- Comprehensive compound indexes for complex queries
CREATE INDEX IF NOT EXISTS idx_blocks_chain_id_block_number_hash ON blocks_monitor_data(chain_id, block_number, block_hash);
CREATE INDEX IF NOT EXISTS idx_blocks_chain_id_block_number_hash_timestamp ON blocks_monitor_data(chain_id, block_number, block_hash, block_timestamp);
CREATE INDEX IF NOT EXISTS idx_blocks_chain_id_block_number_hash_timestamp_parent_hash ON blocks_monitor_data(chain_id, block_number, block_hash, block_timestamp, parent_hash);
CREATE INDEX IF NOT EXISTS idx_blocks_chain_id_block_number_hash_timestamp_parent_hash_gas_used_limit ON blocks_monitor_data(chain_id, block_number, block_hash, block_timestamp, parent_hash, gas_used, gas_limit);

CREATE INDEX IF NOT EXISTS idx_blocks_created_at ON blocks_monitor_data(created_at);
CREATE INDEX IF NOT EXISTS idx_blocks_updated_at ON blocks_monitor_data(updated_at);

-- Create a function to update the updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Create trigger to automatically update updated_at
DROP TRIGGER IF EXISTS update_blocks_monitor_data_updated_at ON blocks_monitor_data;
CREATE TRIGGER update_blocks_monitor_data_updated_at
    BEFORE UPDATE ON blocks_monitor_data
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
