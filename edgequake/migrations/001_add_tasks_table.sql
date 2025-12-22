-- Migration: 001_add_tasks_table
-- Description: Add tasks table for background task processing
-- Phase: 1.1.0
-- Date: 2025-12-22

-- Create tasks table
CREATE TABLE IF NOT EXISTS edgequake.tasks (
    -- Identity
    track_id VARCHAR(50) PRIMARY KEY,
    task_type VARCHAR(20) NOT NULL,

    -- Status
    status VARCHAR(20) NOT NULL,

    -- Timestamps
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,

    -- Error handling
    error_message TEXT,
    retry_count INTEGER DEFAULT 0 NOT NULL,
    max_retries INTEGER DEFAULT 3 NOT NULL,

    -- Payload
    task_data JSONB NOT NULL,

    -- Metadata
    metadata JSONB,

    -- Progress tracking
    progress JSONB,

    -- Result (on success)
    result JSONB,

    -- Constraints
    CONSTRAINT valid_status CHECK (status IN ('pending', 'processing', 'indexed', 'failed', 'cancelled')),
    CONSTRAINT valid_task_type CHECK (task_type IN ('upload', 'insert', 'scan', 'reindex'))
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_tasks_status ON edgequake.tasks(status, created_at);
CREATE INDEX IF NOT EXISTS idx_tasks_type ON edgequake.tasks(task_type);
CREATE INDEX IF NOT EXISTS idx_tasks_created ON edgequake.tasks(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_tasks_updated ON edgequake.tasks(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_tasks_status_type ON edgequake.tasks(status, task_type);

-- Grant permissions
GRANT ALL PRIVILEGES ON edgequake.tasks TO edgequake;

-- Success message
DO $$ BEGIN
    RAISE NOTICE 'Migration 001_add_tasks_table completed successfully!';
END $$;
