-- Add workspace metrics history table for time-series monitoring
-- Migration: 016_workspace_metrics_history.sql
-- Purpose: Track document, entity, relationship, embedding counts over time
-- Use case: Trend analysis, capacity planning, billing, debugging
-- 
-- Design decisions:
-- 1. Separate table (not column additions) for time-series data
-- 2. trigger_type column distinguishes event-driven vs scheduled samples
-- 3. Indexes optimized for time-range queries per workspace
-- 4. CASCADE delete ensures cleanup when workspace is deleted

-- Create workspace_metrics_history table
CREATE TABLE IF NOT EXISTS workspace_metrics_history (
    -- Primary key
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- Foreign key to workspace (UUID to match workspaces.workspace_id type)
    workspace_id UUID NOT NULL,
    
    -- Timestamp when metrics were recorded
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    
    -- How was this sample triggered?
    -- 'event' = document add/delete operation
    -- 'scheduled' = hourly background task
    trigger_type TEXT NOT NULL DEFAULT 'event',
    
    -- Point-in-time counts
    document_count BIGINT NOT NULL DEFAULT 0,
    chunk_count BIGINT NOT NULL DEFAULT 0,
    entity_count BIGINT NOT NULL DEFAULT 0,
    relationship_count BIGINT NOT NULL DEFAULT 0,
    embedding_count BIGINT NOT NULL DEFAULT 0,
    storage_bytes BIGINT NOT NULL DEFAULT 0,
    
    -- Foreign key constraint with cascade delete
    -- WHY: When a workspace is deleted, its history should be cleaned up too
    CONSTRAINT fk_metrics_workspace 
        FOREIGN KEY (workspace_id) 
        REFERENCES workspaces(workspace_id) 
        ON DELETE CASCADE
);

-- Index for time-series queries: "Get metrics for workspace X in time range Y-Z"
-- WHY: Most common query pattern. DESC order for recent-first retrieval.
CREATE INDEX IF NOT EXISTS idx_metrics_workspace_time 
    ON workspace_metrics_history(workspace_id, recorded_at DESC);

-- Index for cleanup queries: "Delete all records older than X"
-- WHY: Retention policy needs to efficiently find old records.
CREATE INDEX IF NOT EXISTS idx_metrics_recorded_at 
    ON workspace_metrics_history(recorded_at);

-- Index for trigger type filtering: "Show only scheduled snapshots"
-- WHY: Analysis may want only scheduled samples for consistent intervals.
CREATE INDEX IF NOT EXISTS idx_metrics_trigger_type 
    ON workspace_metrics_history(trigger_type);

-- Add comment explaining the table purpose
COMMENT ON TABLE workspace_metrics_history IS 
    'Time-series storage of workspace metrics for monitoring and analysis. 
     Samples are recorded either on events (document add/delete) or on schedule (hourly).
     Use with aggregation functions for trend analysis.';

COMMENT ON COLUMN workspace_metrics_history.trigger_type IS 
    'How the sample was triggered: "event" for document operations, "scheduled" for background tasks';

COMMENT ON COLUMN workspace_metrics_history.storage_bytes IS 
    'Total storage used by workspace in bytes (sum of document file sizes)';
