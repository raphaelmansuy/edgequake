-- Migration 026: Fix task_type constraint to include pdf_processing
--
-- WHY: The task_type CHECK constraint from migration 002 only allowed
-- ('upload', 'insert', 'scan', 'reindex'). The PdfProcessing task type
-- was added later but the constraint was never updated. In practice,
-- migration 002's CREATE TABLE IF NOT EXISTS is a no-op because migration 001
-- already creates the tasks table (without a task_type constraint), so this
-- constraint usually doesn't exist. However, this migration provides a
-- defensive fix for any deployment where the constraint was manually created
-- or the table was initialized from docker/init.sql directly.
--
-- SAFE: Uses IF EXISTS so it's idempotent and won't fail on fresh installs.

-- Drop the old constraint names (both possible names from different schemas)
ALTER TABLE tasks DROP CONSTRAINT IF EXISTS valid_task_type;
ALTER TABLE tasks DROP CONSTRAINT IF EXISTS tasks_valid_type;

-- Recreate with all known task types including pdf_processing
ALTER TABLE tasks ADD CONSTRAINT valid_task_type CHECK (
    task_type IN ('upload', 'insert', 'scan', 'reindex', 'pdf_processing')
);

