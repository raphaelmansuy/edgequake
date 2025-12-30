-- Migration: 002_add_document_status_fields
SET search_path = public;
-- Description: Add status tracking fields to documents table
-- Phase: 1.1.0
-- Date: 2025-12-22 (Updated: 2025-01-28)
-- NOTE: Uses PUBLIC schema for consistency

-- Add new columns to documents table
ALTER TABLE documents
    ADD COLUMN IF NOT EXISTS status VARCHAR(20) DEFAULT 'indexed' NOT NULL,
    ADD COLUMN IF NOT EXISTS track_id VARCHAR(50),
    ADD COLUMN IF NOT EXISTS file_path TEXT,
    ADD COLUMN IF NOT EXISTS file_size_bytes BIGINT,
    ADD COLUMN IF NOT EXISTS content_type VARCHAR(100),
    ADD COLUMN IF NOT EXISTS content_hash VARCHAR(64),
    ADD COLUMN IF NOT EXISTS chunk_count INTEGER DEFAULT 0,
    ADD COLUMN IF NOT EXISTS entity_count INTEGER DEFAULT 0,
    ADD COLUMN IF NOT EXISTS relationship_count INTEGER DEFAULT 0,
    ADD COLUMN IF NOT EXISTS processing_time_ms INTEGER,
    ADD COLUMN IF NOT EXISTS error_message TEXT;

-- Add constraint for valid status (idempotent)
DO $$
BEGIN
    ALTER TABLE documents DROP CONSTRAINT IF EXISTS valid_document_status;
    ALTER TABLE documents ADD CONSTRAINT valid_document_status CHECK (
        status IN ('pending', 'processing', 'indexed', 'failed')
    );
EXCEPTION WHEN duplicate_object THEN
    NULL;
END $$;

-- Create indexes for new fields
CREATE INDEX IF NOT EXISTS idx_documents_status ON documents(status);
CREATE INDEX IF NOT EXISTS idx_documents_track_id ON documents(track_id);
CREATE INDEX IF NOT EXISTS idx_documents_content_hash ON documents(content_hash);
CREATE INDEX IF NOT EXISTS idx_documents_file_path ON documents(file_path);

-- Create unique index for content hash (deduplication)
CREATE UNIQUE INDEX IF NOT EXISTS idx_documents_content_hash_unique 
    ON documents(content_hash) 
    WHERE content_hash IS NOT NULL AND status = 'indexed';

-- Success message
DO $$ BEGIN
    RAISE NOTICE 'Migration 002_add_document_status_fields completed successfully!';
END $$;
