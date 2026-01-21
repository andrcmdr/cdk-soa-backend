-- Oracle Service Database Schema
-- Based on Phase 2 requirements from plan.md

-- Revenue tracking
CREATE TABLE IF NOT EXISTS revenue_reports (
    id SERIAL PRIMARY KEY,
    artifact_address VARCHAR(42) NOT NULL,
    -- revenue NUMERIC(78,0) NOT NULL,
    revenue TEXT NOT NULL,
    timestamp BIGINT NOT NULL,
    submitted_to_chain BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(artifact_address, timestamp)
);

-- Usage tracking
CREATE TABLE IF NOT EXISTS usage_reports (
    id SERIAL PRIMARY KEY,
    artifact_address VARCHAR(42) NOT NULL,
    -- usage NUMERIC(78,0) NOT NULL,
    usage TEXT NOT NULL,
    timestamp BIGINT NOT NULL,
    submitted_to_chain BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(artifact_address, timestamp)
);

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_revenue_artifact_timestamp ON revenue_reports(artifact_address, timestamp);
CREATE INDEX IF NOT EXISTS idx_usage_artifact_timestamp ON usage_reports(artifact_address, timestamp);
CREATE INDEX IF NOT EXISTS idx_revenue_submitted ON revenue_reports(submitted_to_chain);
CREATE INDEX IF NOT EXISTS idx_usage_submitted ON usage_reports(submitted_to_chain);

-- Mining state tracking - tracks what time periods have been successfully mined
CREATE TABLE IF NOT EXISTS mining_state (
    id SERIAL PRIMARY KEY,
    start_timestamp BIGINT NOT NULL,
    end_timestamp BIGINT NOT NULL,
    status VARCHAR(20) DEFAULT 'completed' CHECK (status IN ('completed', 'failed')),
    records_found INTEGER DEFAULT 0,
    mined_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(start_timestamp, end_timestamp)
);

-- Index for efficient time range queries
CREATE INDEX IF NOT EXISTS idx_mining_state_time_range ON mining_state(start_timestamp, end_timestamp);
CREATE INDEX IF NOT EXISTS idx_mining_state_status ON mining_state(status);

-- Add some sample data for testing (using only registered artifacts)
INSERT INTO revenue_reports (artifact_address, revenue, timestamp) VALUES
    ('0x13844906650C75E8e9FDf035eAc2F4717c3A5A04', '10', EXTRACT(EPOCH FROM NOW() - INTERVAL '25 minutes')::BIGINT),
    ('0xbc03Dd9B9Bfd695bc77b275fAF94BAD45D8d1eE8', '20', EXTRACT(EPOCH FROM NOW() - INTERVAL '25 minutes')::BIGINT)
ON CONFLICT DO NOTHING;

INSERT INTO usage_reports (artifact_address, usage, timestamp) VALUES
    ('0x13844906650C75E8e9FDf035eAc2F4717c3A5A04', '50', EXTRACT(EPOCH FROM NOW() - INTERVAL '25 minutes')::BIGINT),
    ('0xbc03Dd9B9Bfd695bc77b275fAF94BAD45D8d1eE8', '75', EXTRACT(EPOCH FROM NOW() - INTERVAL '25 minutes')::BIGINT)
ON CONFLICT DO NOTHING;
