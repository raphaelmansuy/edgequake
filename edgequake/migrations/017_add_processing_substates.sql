-- Migration: 017_add_processing_substates
-- Description: Add processing sub-state status values for detailed progress tracking
-- Phase: 1.2.0
-- Date: 2025-05-27
-- 
-- Context: OODA-07 - Frontend needs to display processing sub-states
-- (chunking, extracting, embedding, indexing) for better user visibility.
-- Previously, only 'pending', 'processing', 'indexed', 'failed' were allowed.

SET search_path = public;

-- Drop existing constraint
DO $$
BEGIN
    ALTER TABLE documents DROP CONSTRAINT IF EXISTS valid_document_status;
EXCEPTION WHEN undefined_object THEN
    RAISE NOTICE 'Constraint valid_document_status does not exist, skipping drop';
END $$;

-- Add new constraint with extended status values
-- Includes legacy 'indexed' for backward compatibility
ALTER TABLE documents ADD CONSTRAINT valid_document_status CHECK (
    status IN (
        'pending',      -- Uploaded, waiting for processing
        'processing',   -- Generic processing state (fallback)
        'chunking',     -- Text being split into chunks
        'extracting',   -- LLM extracting entities/relationships
        'embedding',    -- Generating vector embeddings  
        'indexing',     -- Storing in graph/vector databases
        'completed',    -- Successfully processed
        'indexed',      -- Legacy: same as completed (kept for compatibility)
        'failed',       -- Processing failed with error
        'cancelled'     -- User cancelled processing
    )
);

-- Migrate legacy 'indexed' status to 'completed' for consistency
-- Comment out if you want to keep both for backward compatibility
-- UPDATE documents SET status = 'completed' WHERE status = 'indexed';

-- Add index on status for faster filtering
CREATE INDEX IF NOT EXISTS idx_documents_status_v2 ON documents(status);

-- Success message
DO $$ BEGIN
    RAISE NOTICE 'Migration 017_add_processing_substates completed successfully!';
    RAISE NOTICE 'New status values: pending, processing, chunking, extracting, embedding, indexing, completed, failed, cancelled';
END $$;
