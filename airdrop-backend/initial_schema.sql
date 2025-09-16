-- Airdrop Backend Database Schema

-- Extension for UUID generation
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Trie states table - main storage for Merkle Patricia Trie data
CREATE TABLE trie_states (
    round_id INTEGER PRIMARY KEY,
    root_hash BYTEA NOT NULL,
    trie_data BYTEA NOT NULL,
    entry_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Index on root_hash for quick lookups
CREATE INDEX idx_trie_states_root_hash ON trie_states(root_hash);

-- Eligibility records table - individual user eligibility data
CREATE TABLE eligibility_records (
    id SERIAL PRIMARY KEY,
    address BYTEA NOT NULL,
    amount NUMERIC(78, 0) NOT NULL, -- Support for U256 values
    round_id INTEGER NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(address, round_id),
    FOREIGN KEY (round_id) REFERENCES trie_states(round_id) ON DELETE CASCADE
);

-- Indexes for performance
CREATE INDEX idx_eligibility_records_round_id ON eligibility_records(round_id);
CREATE INDEX idx_eligibility_records_address ON eligibility_records(address);
CREATE INDEX idx_eligibility_records_amount ON eligibility_records(amount);

-- Processing logs table - audit trail for all operations
CREATE TABLE processing_logs (
    id SERIAL PRIMARY KEY,
    round_id INTEGER NOT NULL,
    operation VARCHAR(50) NOT NULL,
    status VARCHAR(20) NOT NULL CHECK (status IN ('started', 'completed', 'failed', 'skipped')),
    message TEXT,
    transaction_hash VARCHAR(66), -- Ethereum transaction hash (0x + 64 hex chars)
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for processing logs
CREATE INDEX idx_processing_logs_round_id ON processing_logs(round_id);
CREATE INDEX idx_processing_logs_status ON processing_logs(status);
CREATE INDEX idx_processing_logs_operation ON processing_logs(operation);
CREATE INDEX idx_processing_logs_created_at ON processing_logs(created_at DESC);

-- CSV uploads table - track uploaded CSV files
CREATE TABLE csv_uploads (
    id SERIAL PRIMARY KEY,
    round_id INTEGER NOT NULL,
    filename VARCHAR(255) NOT NULL,
    file_size BIGINT NOT NULL,
    file_hash VARCHAR(64) NOT NULL, -- SHA-256 hash of file content
    nats_object_name VARCHAR(255) NOT NULL,
    records_count INTEGER NOT NULL DEFAULT 0,
    uploaded_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    processed_at TIMESTAMP WITH TIME ZONE
);

-- Index for CSV uploads
CREATE INDEX idx_csv_uploads_round_id ON csv_uploads(round_id);
CREATE INDEX idx_csv_uploads_uploaded_at ON csv_uploads(uploaded_at DESC);

-- Blockchain transactions table - track all blockchain interactions
CREATE TABLE blockchain_transactions (
    id SERIAL PRIMARY KEY,
    round_id INTEGER NOT NULL,
    transaction_hash VARCHAR(66) NOT NULL UNIQUE,
    operation_type VARCHAR(50) NOT NULL, -- 'submit_trie', 'verify_eligibility'
    block_number BIGINT,
    gas_used BIGINT,
    gas_price BIGINT,
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'confirmed', 'failed')),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    confirmed_at TIMESTAMP WITH TIME ZONE
);

-- Indexes for blockchain transactions
CREATE INDEX idx_blockchain_transactions_round_id ON blockchain_transactions(round_id);
CREATE INDEX idx_blockchain_transactions_hash ON blockchain_transactions(transaction_hash);
CREATE INDEX idx_blockchain_transactions_status ON blockchain_transactions(status);

-- System configuration table - store system-wide settings
CREATE TABLE system_config (
    key VARCHAR(100) PRIMARY KEY,
    value TEXT NOT NULL,
    description TEXT,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Insert default system configuration
INSERT INTO system_config (key, value, description) VALUES
('schema_version', '1', 'Database schema version'),
('last_processed_round', '0', 'Last successfully processed round'),
('maintenance_mode', 'false', 'System maintenance mode flag');

-- Views for reporting and analytics

-- Round summary view
CREATE VIEW round_summary AS
SELECT 
    ts.round_id,
    ts.root_hash,
    ts.entry_count,
    ts.created_at,
    ts.updated_at,
    COUNT(DISTINCT er.address) as unique_addresses,
    SUM(er.amount) as total_amount,
    MIN(er.amount) as min_amount,
    MAX(er.amount) as max_amount,
    AVG(er.amount) as avg_amount,
    COUNT(DISTINCT pl.id) as processing_events,
    COUNT(DISTINCT bt.id) as blockchain_transactions
FROM trie_states ts
LEFT JOIN eligibility_records er ON ts.round_id = er.round_id
LEFT JOIN processing_logs pl ON ts.round_id = pl.round_id
LEFT JOIN blockchain_transactions bt ON ts.round_id = bt.round_id
GROUP BY ts.round_id, ts.root_hash, ts.entry_count, ts.created_at, ts.updated_at;

-- Processing status view
CREATE VIEW processing_status AS
SELECT
    round_id,
    COUNT(*) as total_operations,
    COUNT(CASE WHEN status = 'completed' THEN 1 END) as completed_operations,
    COUNT(CASE WHEN status = 'failed' THEN 1 END) as failed_operations,
    COUNT(CASE WHEN status = 'pending' THEN 1 END) as pending_operations,
    MAX(created_at) as last_activity
FROM processing_logs
GROUP BY round_id;

-- Functions for maintenance

-- Function to clean up old processing logs (keep last 30 days)
CREATE OR REPLACE FUNCTION cleanup_old_logs()
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM processing_logs
    WHERE created_at < NOW() - INTERVAL '30 days';

    GET DIAGNOSTICS deleted_count = ROW_COUNT;

    INSERT INTO processing_logs (round_id, operation, status, message)
    VALUES (0, 'cleanup', 'completed', 'Cleaned up ' || deleted_count || ' old log entries');

    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- Function to get round statistics
CREATE OR REPLACE FUNCTION get_round_stats(p_round_id INTEGER DEFAULT NULL)
RETURNS TABLE (
    round_id INTEGER,
    entry_count INTEGER,
    unique_addresses BIGINT,
    total_amount NUMERIC,
    avg_amount NUMERIC,
    processing_events BIGINT,
    blockchain_txs BIGINT,
    last_updated TIMESTAMP WITH TIME ZONE
) AS $$
BEGIN
    IF p_round_id IS NULL THEN
        RETURN QUERY
        SELECT * FROM round_summary ORDER BY round_summary.round_id;
    ELSE
        RETURN QUERY
        SELECT * FROM round_summary WHERE round_summary.round_id = p_round_id;
    END IF;
END;
$$ LANGUAGE plpgsql;

-- Triggers for automatic timestamp updates
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_trie_states_updated_at
    BEFORE UPDATE ON trie_states
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_system_config_updated_at
    BEFORE UPDATE ON system_config
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
