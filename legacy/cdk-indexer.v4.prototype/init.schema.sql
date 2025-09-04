-- Core logs table with exhaustive indexes for high-volume indexing.
CREATE TABLE IF NOT EXISTS logs (
    -- Unique 32-byte SHA3-256 hash over the alloy Log (see subscriptions.rs)
    log_hash         BYTEA PRIMARY KEY,

    chain_id         BIGINT       NOT NULL,

    -- Basic log identity
    contract_name    TEXT,
    contract_address BYTEA        NOT NULL,

    -- Event identity
    event_name       TEXT,
    event_hash       BYTEA,

    -- Raw log payload
    topics           JSONB        NOT NULL,
    data             BYTEA        NOT NULL,

    -- Placement within chain
    block_hash       BYTEA,
    block_number     BIGINT,
    block_timestamp  TIMESTAMPTZ,

    transaction_hash BYTEA,
    transaction_index INTEGER,
    log_index        INTEGER,
    removed          BOOLEAN      NOT NULL DEFAULT FALSE,

    -- Enrichment
    tx_sender        BYTEA,

    -- Decoded values (ABI-aware); arbitrary shape per event
    decoded_params   JSONB,

    inserted_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- Uniqueness and fast lookups
CREATE UNIQUE INDEX IF NOT EXISTS logs_tx_log_unique
    ON logs (transaction_hash, log_index);

CREATE INDEX IF NOT EXISTS logs_addr_evhash_idx
    ON logs (chain_id, contract_address, event_hash);

CREATE INDEX IF NOT EXISTS logs_blocknum_idx
    ON logs (block_number);

CREATE INDEX IF NOT EXISTS logs_blockhash_idx
    ON logs (block_hash);

CREATE INDEX IF NOT EXISTS logs_txhash_idx
    ON logs (transaction_hash);

CREATE INDEX IF NOT EXISTS logs_bts_idx
    ON logs (block_timestamp);

CREATE INDEX IF NOT EXISTS logs_event_name_idx
    ON logs (event_name);

-- GIN index for JSONB search over decoded params
CREATE INDEX IF NOT EXISTS logs_decoded_params_gin
    ON logs USING GIN (decoded_params);

-- Useful composite for time-ranged address scans
CREATE INDEX IF NOT EXISTS logs_addr_time_idx
    ON logs (contract_address, block_timestamp);
