-- OODA-17: Fix task status constraint to match Rust TaskStatus enum
-- 
-- Problem: Rust TaskStatus enum uses 'processing' and 'indexed' but
-- the database constraint expected 'running' and 'completed'.
--
-- Root Cause: types.rs defines TaskStatus as:
--   - Processing → serializes to "processing"
--   - Indexed → serializes to "indexed"
--
-- But the constraint CHECK was created with legacy values:
--   CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled'))
--
-- Impact: ALL task status updates were failing with:
--   "new row for relation 'tasks' violates check constraint 'tasks_valid_status'"
--
-- Fix: Update constraint to accept Rust enum values.

-- Step 1: Update any existing data using old values
UPDATE tasks SET status = 'processing' WHERE status = 'running';
UPDATE tasks SET status = 'indexed' WHERE status = 'completed';

-- Step 2: Drop old constraint
ALTER TABLE tasks 
DROP CONSTRAINT IF EXISTS tasks_valid_status;

-- Step 3: Add new constraint matching Rust TaskStatus enum
ALTER TABLE tasks 
ADD CONSTRAINT tasks_valid_status 
  CHECK (status IN ('pending', 'processing', 'indexed', 'failed', 'cancelled'));

-- Step 4: Also fix the valid_status constraint if it exists (from migration 002)
ALTER TABLE tasks 
DROP CONSTRAINT IF EXISTS valid_status;
