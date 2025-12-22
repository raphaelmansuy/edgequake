-- Migration: 002_add_document_status_fields
-- Description: Add status tracking fields to documents table
-- Phase: 1.1.0
-- Date: 2025-12-22

-- Add new columns to documents table
ALTER TABLE edgequake.documents
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

-- Add constraint for valid status
ALTER TABLE edgequake.documents
    DROP CONSTRAINT IF EXISTS valid_document_status,
    ADD CONSTRAINT valid_document_status CHECK (
        status IN ('pending', 'processing', 'indexed', 'failed')
    );

-- Create indexes for new fields
CREATE INDEX IF NOT EXISTS idx_documents_status ON edgequake.documents(status);
CREATE INDEX IF NOT EXISTS idx_documents_track_id ON edgequake.documents(track_id);
CREATE INDEX IF NOT EXISTS idx_documents_content_hash ON edgequake.documents(content_hash);
CREATE INDEX IF NOT EXISTS idx_documents_file_path ON edgequake.documents(file_path);

-- Create unique index for content hash (deduplication)
CREATE UNIQUE INDEX IF NOT EXISTS idx_documents_content_hash_unique 
    ON edgequake.documents(content_hash) 
    WHERE content_hash IS NOT NULL AND status = 'indexed';

-- Success message
DO $$ BEGIN
    RAISE NOTICE 'Migration 002_add_document_status_fields completed successfully!';
END $$;
