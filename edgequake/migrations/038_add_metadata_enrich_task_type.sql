-- Migration 038: Add metadata_enrich to valid task_type constraint
--
-- WHY: Migration 026 added a CHECK constraint limiting task_type to a fixed list.
-- The PDF Metadata Enrichment Pipeline introduces a new 'metadata_enrich' task type
-- that runs before full graph processing to extract summary/topic/language/keywords.
-- Without this, INSERT of metadata_enrich tasks fails with a constraint violation
-- and the upload endpoint returns HTTP 500.

ALTER TABLE tasks DROP CONSTRAINT IF EXISTS valid_task_type;

ALTER TABLE tasks ADD CONSTRAINT valid_task_type CHECK (
    task_type IN (
        'upload',
        'insert',
        'scan',
        'reindex',
        'pdf_processing',
        'metadata_enrich'
    )
);
