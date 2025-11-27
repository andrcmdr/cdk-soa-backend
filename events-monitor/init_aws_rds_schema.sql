-- AWS RDS PostgreSQL Schema for Events Monitor
-- This schema is compatible with AWS RDS PostgreSQL instances

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
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (chain_id, log_hash, event_name, event_signature) -- Add UNIQUE constraint for deduplication
);

-- Create indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_aws_contract_name ON events_monitor_data (contract_name);
CREATE INDEX IF NOT EXISTS idx_aws_contract_address ON events_monitor_data (contract_address);
CREATE INDEX IF NOT EXISTS idx_aws_impl_name ON events_monitor_data (implementation_name);
CREATE INDEX IF NOT EXISTS idx_aws_impl_address ON events_monitor_data (implementation_address);
CREATE INDEX IF NOT EXISTS idx_aws_contract_impl_compound ON events_monitor_data (contract_name, contract_address, implementation_name, implementation_address);

CREATE INDEX IF NOT EXISTS idx_aws_chain_id ON events_monitor_data (chain_id);
CREATE INDEX IF NOT EXISTS idx_aws_block_number ON events_monitor_data (block_number);
CREATE INDEX IF NOT EXISTS idx_aws_block_hash ON events_monitor_data (block_hash);
CREATE INDEX IF NOT EXISTS idx_aws_block_timestamp ON events_monitor_data (block_timestamp);
CREATE INDEX IF NOT EXISTS idx_aws_block_time ON events_monitor_data (block_time);
CREATE INDEX IF NOT EXISTS idx_aws_chain_block_compound ON events_monitor_data (chain_id, block_number, block_hash, block_timestamp);

CREATE INDEX IF NOT EXISTS idx_aws_transaction_hash ON events_monitor_data (transaction_hash);
CREATE INDEX IF NOT EXISTS idx_aws_transaction_sender ON events_monitor_data (transaction_sender);
CREATE INDEX IF NOT EXISTS idx_aws_transaction_receiver ON events_monitor_data (transaction_receiver);
CREATE INDEX IF NOT EXISTS idx_aws_transaction_index ON events_monitor_data (transaction_index);
CREATE INDEX IF NOT EXISTS idx_aws_log_index ON events_monitor_data (log_index);
CREATE INDEX IF NOT EXISTS idx_aws_log_hash ON events_monitor_data (log_hash); -- Primary deduplication index
CREATE INDEX IF NOT EXISTS idx_aws_transaction_log_compound ON events_monitor_data (transaction_hash, transaction_sender, transaction_receiver, transaction_index, log_index, log_hash);

CREATE INDEX IF NOT EXISTS idx_aws_event_name ON events_monitor_data (event_name);
CREATE INDEX IF NOT EXISTS idx_aws_event_signature ON events_monitor_data (event_signature);
CREATE INDEX IF NOT EXISTS idx_aws_event_data_jsonb ON events_monitor_data USING gin (event_data);
CREATE INDEX IF NOT EXISTS idx_aws_event_compound ON events_monitor_data (event_name, event_signature);

CREATE INDEX IF NOT EXISTS idx_aws_created_at ON events_monitor_data (created_at);
CREATE INDEX IF NOT EXISTS idx_aws_updated_at ON events_monitor_data (updated_at);

-- Comprehensive compound index for complex queries
CREATE INDEX IF NOT EXISTS idx_aws_comprehensive ON events_monitor_data (contract_name, contract_address, implementation_name, implementation_address, chain_id, block_number, block_hash, block_timestamp, transaction_hash, transaction_sender, transaction_receiver, transaction_index, log_index, log_hash, event_name, event_signature);

-- Create a function to update the updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Create trigger to automatically update updated_at
DROP TRIGGER IF EXISTS update_events_monitor_data_updated_at ON events_monitor_data;
CREATE TRIGGER update_events_monitor_data_updated_at
    BEFORE UPDATE ON events_monitor_data
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
