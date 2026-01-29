-- Migration 021: Add failed_chunks table for retry queue
--
-- @implements SPEC-003: Chunk-level resilience with retry functionality
--
-- WHY: When using process_with_resilience, some chunks may fail during extraction.
-- This table stores failed chunks so they can be retried later, either automatically
-- or manually via the UI.

CREATE TABLE IF NOT EXISTS failed_chunks (
    -- Primary key
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- Foreign keys (references without FK constraints for flexibility)
    document_id VARCHAR(255) NOT NULL,
    workspace_id UUID NOT NULL,
    tenant_id UUID,
    
    -- Chunk identification
    chunk_index INTEGER NOT NULL,
    chunk_id VARCHAR(255) NOT NULL,
    
    -- Failure details
    error_message TEXT NOT NULL,
    was_timeout BOOLEAN NOT NULL DEFAULT FALSE,
    retry_attempts INTEGER NOT NULL DEFAULT 0,
    processing_time_ms BIGINT,
    
    -- Status for retry tracking
    -- 'pending' = awaiting retry
    -- 'retrying' = currently being retried
    -- 'succeeded' = retry succeeded
    -- 'abandoned' = exceeded max retries, user gave up
    status VARCHAR(32) NOT NULL DEFAULT 'pending',
    
    -- Timestamps
    failed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    retry_scheduled_at TIMESTAMPTZ,
    last_retry_at TIMESTAMPTZ,
    resolved_at TIMESTAMPTZ,
    
    -- Constraint to prevent duplicate entries
    UNIQUE(document_id, chunk_index, failed_at)
);

-- Index for fetching failed chunks by document
CREATE INDEX IF NOT EXISTS idx_failed_chunks_document_id 
ON failed_chunks(document_id);

-- Index for fetching pending retries by workspace
CREATE INDEX IF NOT EXISTS idx_failed_chunks_workspace_pending 
ON failed_chunks(workspace_id, status) 
WHERE status = 'pending';

-- Index for scheduling retries
CREATE INDEX IF NOT EXISTS idx_failed_chunks_retry_scheduled 
ON failed_chunks(retry_scheduled_at) 
WHERE status = 'pending' AND retry_scheduled_at IS NOT NULL;

-- Comment for documentation
COMMENT ON TABLE failed_chunks IS 'Stores failed chunk extractions for retry functionality (SPEC-003)';
COMMENT ON COLUMN failed_chunks.status IS 'Retry status: pending, retrying, succeeded, abandoned';
COMMENT ON COLUMN failed_chunks.was_timeout IS 'True if failure was due to extraction timeout';
COMMENT ON COLUMN failed_chunks.retry_attempts IS 'Number of retry attempts so far';
