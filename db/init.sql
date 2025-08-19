-- Oracle Service Database Schema
-- Based on Phase 2 requirements from plan.md

-- Revenue tracking
CREATE TABLE IF NOT EXISTS revenue_reports (
    id SERIAL PRIMARY KEY,
    artifact_address VARCHAR(42) NOT NULL,
    revenue NUMERIC(78,0) NOT NULL,
    timestamp BIGINT NOT NULL,
    submitted_to_chain BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Usage tracking
CREATE TABLE IF NOT EXISTS usage_reports (
    id SERIAL PRIMARY KEY,
    artifact_address VARCHAR(42) NOT NULL,
    usage NUMERIC(78,0) NOT NULL,
    timestamp BIGINT NOT NULL,
    submitted_to_chain BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_revenue_artifact_timestamp ON revenue_reports(artifact_address, timestamp);
CREATE INDEX IF NOT EXISTS idx_usage_artifact_timestamp ON usage_reports(artifact_address, timestamp);
CREATE INDEX IF NOT EXISTS idx_revenue_submitted ON revenue_reports(submitted_to_chain);
CREATE INDEX IF NOT EXISTS idx_usage_submitted ON usage_reports(submitted_to_chain);

-- Add some sample data for testing
INSERT INTO revenue_reports (artifact_address, revenue, timestamp) VALUES
    ('0x1234567890123456789012345678901234567890', 1000000000000000000, EXTRACT(EPOCH FROM NOW())::BIGINT),
    ('0x2345678901234567890123456789012345678901', 2000000000000000000, EXTRACT(EPOCH FROM NOW())::BIGINT)
ON CONFLICT DO NOTHING;

INSERT INTO usage_reports (artifact_address, usage, timestamp) VALUES
    ('0x1234567890123456789012345678901234567890', 500000000000000000, EXTRACT(EPOCH FROM NOW())::BIGINT),
    ('0x2345678901234567890123456789012345678901', 750000000000000000, EXTRACT(EPOCH FROM NOW())::BIGINT)
ON CONFLICT DO NOTHING;
