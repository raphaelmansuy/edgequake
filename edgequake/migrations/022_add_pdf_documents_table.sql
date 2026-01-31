-- ============================================================================
-- Migration 022: Add PDF Documents Table
-- Version: 1.0.0
-- Date: 2025-01-31
-- Author: EdgeQuake Team
-- Description: Store raw PDF files with format metadata for vision LLM processing
-- ============================================================================
--
-- @implements SPEC-007: PDF Upload Support with Vision LLM
-- @implements BR0001: Document deduplication via SHA-256 checksum
-- @implements BR0201: Workspace isolation with RLS
--
-- This migration creates the pdf_documents table to store raw PDF files
-- alongside their processing metadata. PDFs are stored as BYTEA and processed
-- asynchronously with vision LLM (gpt-4o-mini or gemma3:latest).
--
-- USAGE:
--   psql -U postgres -d edgequake -f 022_add_pdf_documents_table.sql
--
-- ROLLBACK:
--   See rollback section at end of file
--
-- ============================================================================

-- Set search path to public schema
SET search_path = public;

-- ============================================================================
-- SECTION 1: DROP EXISTING (Idempotent)
-- ============================================================================

-- Drop policies if they exist
DROP POLICY IF EXISTS pdf_documents_tenant_isolation ON pdf_documents;
DROP POLICY IF EXISTS pdf_documents_workspace_select ON pdf_documents;
DROP POLICY IF EXISTS pdf_documents_workspace_insert ON pdf_documents;
DROP POLICY IF EXISTS pdf_documents_workspace_update ON pdf_documents;
DROP POLICY IF EXISTS pdf_documents_workspace_delete ON pdf_documents;

-- Drop indexes if they exist
DROP INDEX IF EXISTS idx_pdf_documents_workspace CASCADE;
DROP INDEX IF EXISTS idx_pdf_documents_status CASCADE;
DROP INDEX IF EXISTS idx_pdf_documents_created CASCADE;
DROP INDEX IF EXISTS idx_pdf_documents_checksum CASCADE;
DROP INDEX IF EXISTS idx_pdf_documents_workspace_status CASCADE;
DROP INDEX IF EXISTS idx_pdf_documents_document_id CASCADE;

-- Drop table if exists (for development/testing)
-- CAUTION: This will delete all data!
-- DROP TABLE IF EXISTS pdf_documents CASCADE;

-- ============================================================================
-- SECTION 2: CREATE TABLE
-- ============================================================================

CREATE TABLE IF NOT EXISTS pdf_documents (
    -- ========================================================================
    -- Primary Key
    -- ========================================================================
    pdf_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- ========================================================================
    -- Foreign Keys
    -- ========================================================================
    
    -- Workspace isolation (MANDATORY)
    workspace_id UUID NOT NULL REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
    
    -- Link to processed document (NULL during processing, set when indexed)
    document_id UUID UNIQUE REFERENCES documents(id) ON DELETE CASCADE,
    
    -- ========================================================================
    -- PDF Metadata
    -- ========================================================================
    
    -- Original filename from upload
    filename VARCHAR(512) NOT NULL,
    
    -- MIME type (always 'application/pdf' for now, future: allow other formats)
    content_type VARCHAR(100) NOT NULL DEFAULT 'application/pdf',
    
    -- File size in bytes (for billing, quota, and optimization decisions)
    file_size_bytes BIGINT NOT NULL,
    
    -- SHA-256 checksum for deduplication and integrity verification
    -- WHY: Prevents duplicate uploads, enables integrity checks after storage
    sha256_checksum VARCHAR(64) NOT NULL,
    
    -- Number of pages in PDF (extracted during initial parse)
    page_count INTEGER,
    
    -- ========================================================================
    -- Raw PDF Storage
    -- ========================================================================
    
    -- Raw PDF bytes stored as BYTEA
    -- WHY: Enables reprocessing with different settings/models without re-upload
    -- TRADE-OFF: Increases storage size but eliminates need for external blob storage
    pdf_data BYTEA NOT NULL,
    
    -- ========================================================================
    -- Processing State
    -- ========================================================================
    
    -- Current processing status
    -- Values: pending, processing, completed, failed
    processing_status VARCHAR(50) NOT NULL DEFAULT 'pending',
    
    -- Extraction method used (text, vision, hybrid)
    -- NULL if not yet processed
    extraction_method VARCHAR(50),
    
    -- Vision model used for extraction (if applicable)
    -- e.g., 'gpt-4o-mini', 'gemma3:latest', NULL for text-only
    vision_model VARCHAR(100),
    
    -- ========================================================================
    -- Processing Results
    -- ========================================================================
    
    -- Extracted markdown content
    -- NULL if processing incomplete
    markdown_content TEXT,
    
    -- Extraction errors/warnings (JSONB for structured logging)
    -- Format: {"errors": [...], "warnings": [...], "retries": N}
    extraction_errors JSONB,
    
    -- ========================================================================
    -- Timestamps
    -- ========================================================================
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- ========================================================================
    -- Constraints
    -- ========================================================================
    
    CONSTRAINT valid_processing_status CHECK (
        processing_status IN ('pending', 'processing', 'completed', 'failed')
    ),
    
    CONSTRAINT valid_extraction_method CHECK (
        extraction_method IS NULL OR 
        extraction_method IN ('text', 'vision', 'hybrid')
    ),
    
    CONSTRAINT valid_file_size CHECK (
        file_size_bytes > 0 AND file_size_bytes <= 104857600
    ), -- 100MB max
    
    CONSTRAINT valid_page_count CHECK (
        page_count IS NULL OR page_count > 0
    ),
    
    CONSTRAINT valid_checksum_format CHECK (
        sha256_checksum ~ '^[a-f0-9]{64}$'
    )
);

-- ============================================================================
-- SECTION 3: CREATE INDEXES
-- ============================================================================

-- Index for workspace queries (most common access pattern)
CREATE INDEX IF NOT EXISTS idx_pdf_documents_workspace 
    ON pdf_documents(workspace_id);

-- Index for status filtering
CREATE INDEX IF NOT EXISTS idx_pdf_documents_status 
    ON pdf_documents(processing_status);

-- Index for chronological queries
CREATE INDEX IF NOT EXISTS idx_pdf_documents_created 
    ON pdf_documents(created_at DESC);

-- Index for deduplication checks
CREATE INDEX IF NOT EXISTS idx_pdf_documents_checksum 
    ON pdf_documents(sha256_checksum);

-- Composite index for workspace + status queries (most efficient)
CREATE INDEX IF NOT EXISTS idx_pdf_documents_workspace_status 
    ON pdf_documents(workspace_id, processing_status, created_at DESC);

-- Index for document_id lookups
CREATE INDEX IF NOT EXISTS idx_pdf_documents_document_id 
    ON pdf_documents(document_id) 
    WHERE document_id IS NOT NULL;

-- ============================================================================
-- SECTION 4: ENABLE ROW LEVEL SECURITY (RLS)
-- ============================================================================

-- Enable RLS for multi-tenancy isolation
ALTER TABLE pdf_documents ENABLE ROW LEVEL SECURITY;

-- ============================================================================
-- SECTION 5: CREATE RLS POLICIES
-- ============================================================================

-- Policy: Tenant isolation (read-only, for admins/system)
-- Allows users to see only PDFs in workspaces belonging to their tenant
CREATE POLICY pdf_documents_tenant_isolation ON pdf_documents
    FOR SELECT
    USING (
        workspace_id IN (
            SELECT workspace_id 
            FROM workspaces 
            WHERE tenant_id = current_setting('app.current_tenant_id', TRUE)::UUID
        )
    );

-- Policy: Workspace-level SELECT
-- Users can only select PDFs in their current workspace
CREATE POLICY pdf_documents_workspace_select ON pdf_documents
    FOR SELECT
    USING (
        workspace_id = current_setting('app.current_workspace_id', TRUE)::UUID
    );

-- Policy: Workspace-level INSERT
-- Users can only insert PDFs into their current workspace
CREATE POLICY pdf_documents_workspace_insert ON pdf_documents
    FOR INSERT
    WITH CHECK (
        workspace_id = current_setting('app.current_workspace_id', TRUE)::UUID
    );

-- Policy: Workspace-level UPDATE
-- Users can only update PDFs in their current workspace
CREATE POLICY pdf_documents_workspace_update ON pdf_documents
    FOR UPDATE
    USING (
        workspace_id = current_setting('app.current_workspace_id', TRUE)::UUID
    )
    WITH CHECK (
        workspace_id = current_setting('app.current_workspace_id', TRUE)::UUID
    );

-- Policy: Workspace-level DELETE
-- Users can only delete PDFs in their current workspace
CREATE POLICY pdf_documents_workspace_delete ON pdf_documents
    FOR DELETE
    USING (
        workspace_id = current_setting('app.current_workspace_id', TRUE)::UUID
    );

-- ============================================================================
-- SECTION 6: ADD COLUMN COMMENTS (Documentation)
-- ============================================================================

COMMENT ON TABLE pdf_documents IS 
'Stores raw PDF files with format metadata for vision LLM processing (SPEC-007). PDFs are processed asynchronously and linked to documents table after markdown extraction.';

COMMENT ON COLUMN pdf_documents.pdf_id IS 
'Unique identifier for PDF file';

COMMENT ON COLUMN pdf_documents.workspace_id IS 
'Workspace isolation (MANDATORY). Each PDF belongs to exactly one workspace.';

COMMENT ON COLUMN pdf_documents.document_id IS 
'Link to processed document (NULL during processing, set when indexed)';

COMMENT ON COLUMN pdf_documents.filename IS 
'Original filename from upload (max 512 chars)';

COMMENT ON COLUMN pdf_documents.content_type IS 
'MIME type (application/pdf)';

COMMENT ON COLUMN pdf_documents.file_size_bytes IS 
'File size in bytes (for billing, quotas, optimization)';

COMMENT ON COLUMN pdf_documents.sha256_checksum IS 
'SHA-256 hash for deduplication and integrity verification';

COMMENT ON COLUMN pdf_documents.page_count IS 
'Number of pages in PDF (extracted during initial parse)';

COMMENT ON COLUMN pdf_documents.pdf_data IS 
'Raw PDF bytes stored as BYTEA. Enables reprocessing without re-upload.';

COMMENT ON COLUMN pdf_documents.processing_status IS 
'Current processing status: pending | processing | completed | failed';

COMMENT ON COLUMN pdf_documents.extraction_method IS 
'Method used for extraction: text | vision | hybrid (NULL if not processed)';

COMMENT ON COLUMN pdf_documents.vision_model IS 
'Vision LLM model used: gpt-4o-mini | gemma3:latest | NULL for text-only';

COMMENT ON COLUMN pdf_documents.markdown_content IS 
'Extracted markdown content (NULL if processing incomplete)';

COMMENT ON COLUMN pdf_documents.extraction_errors IS 
'JSONB with errors/warnings: {"errors": [...], "warnings": [...], "retries": N}';

COMMENT ON COLUMN pdf_documents.created_at IS 
'Timestamp when PDF was uploaded';

COMMENT ON COLUMN pdf_documents.processed_at IS 
'Timestamp when processing completed (NULL if incomplete)';

COMMENT ON COLUMN pdf_documents.updated_at IS 
'Timestamp of last update';

-- ============================================================================
-- SECTION 7: CREATE TRIGGER FOR updated_at
-- ============================================================================

-- Create trigger function if it doesn't exist
CREATE OR REPLACE FUNCTION trigger_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create trigger for pdf_documents
DROP TRIGGER IF EXISTS set_updated_at ON pdf_documents;
CREATE TRIGGER set_updated_at
    BEFORE UPDATE ON pdf_documents
    FOR EACH ROW
    EXECUTE FUNCTION trigger_set_updated_at();

-- ============================================================================
-- SECTION 8: GRANT PERMISSIONS
-- ============================================================================

-- Grant permissions to edgequake role (adjust as needed)
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'edgequake') THEN
        GRANT SELECT, INSERT, UPDATE, DELETE ON pdf_documents TO edgequake;
        -- Note: No sequence grant needed since we use gen_random_uuid()
    END IF;
END $$;

-- ============================================================================
-- SECTION 9: VERIFICATION
-- ============================================================================

DO $$
DECLARE
    table_exists BOOLEAN;
    index_count INTEGER;
    policy_count INTEGER;
BEGIN
    -- Check if table exists
    SELECT EXISTS (
        SELECT FROM information_schema.tables 
        WHERE table_schema = 'public' 
        AND table_name = 'pdf_documents'
    ) INTO table_exists;
    
    IF NOT table_exists THEN
        RAISE EXCEPTION 'Migration 022 FAILED: pdf_documents table not created';
    END IF;
    
    -- Check indexes
    SELECT COUNT(*) INTO index_count
    FROM pg_indexes
    WHERE schemaname = 'public'
    AND tablename = 'pdf_documents';
    
    IF index_count < 6 THEN
        RAISE WARNING 'Migration 022: Expected 6 indexes, found %', index_count;
    END IF;
    
    -- Check RLS policies
    SELECT COUNT(*) INTO policy_count
    FROM pg_policies
    WHERE schemaname = 'public'
    AND tablename = 'pdf_documents';
    
    IF policy_count < 5 THEN
        RAISE WARNING 'Migration 022: Expected 5 RLS policies, found %', policy_count;
    END IF;
    
    -- Success message
    RAISE NOTICE '✅ Migration 022 completed successfully!';
    RAISE NOTICE '   - Table: pdf_documents created';
    RAISE NOTICE '   - Indexes: % created', index_count;
    RAISE NOTICE '   - RLS Policies: % created', policy_count;
    RAISE NOTICE '   - Trigger: updated_at created';
    RAISE NOTICE '';
    RAISE NOTICE '🚀 Ready to store PDF documents with vision LLM support!';
END $$;

-- ============================================================================
-- ROLLBACK INSTRUCTIONS
-- ============================================================================
--
-- To rollback this migration, run:
--
-- DROP POLICY IF EXISTS pdf_documents_tenant_isolation ON pdf_documents;
-- DROP POLICY IF EXISTS pdf_documents_workspace_select ON pdf_documents;
-- DROP POLICY IF EXISTS pdf_documents_workspace_insert ON pdf_documents;
-- DROP POLICY IF EXISTS pdf_documents_workspace_update ON pdf_documents;
-- DROP POLICY IF EXISTS pdf_documents_workspace_delete ON pdf_documents;
-- DROP TABLE IF EXISTS pdf_documents CASCADE;
-- DROP FUNCTION IF EXISTS trigger_set_updated_at() CASCADE;
--
-- ============================================================================
