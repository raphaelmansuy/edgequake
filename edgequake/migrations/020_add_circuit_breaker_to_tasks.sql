-- Migration: 020_add_circuit_breaker_to_tasks
-- Description: Add circuit breaker fields to tasks table for timeout tracking
-- Phase: 1.2.0
-- Date: 2026-01-28
-- 
-- WHY: Circuit breaker pattern prevents infinite retries on documents that consistently timeout
-- After 3 consecutive LLM timeouts, task is permanently failed to conserve resources
--
-- Related: 
-- - adaptive chunking (orchestrator.rs) reduces timeout probability
-- - this migration handles cases where adaptive chunking isn't enough

SET search_path = public;

-- ============================================================================
-- Add circuit breaker fields to tasks table
-- ============================================================================

-- Add consecutive_timeout_failures column
-- WHY: Track consecutive timeouts (not total retries, just timeout-specific failures)
-- Reset to 0 on: success or non-timeout failure
-- Increment on: LLM timeout or embedding timeout
ALTER TABLE tasks 
ADD COLUMN IF NOT EXISTS consecutive_timeout_failures INTEGER DEFAULT 0 NOT NULL;

-- Add circuit_breaker_tripped column
-- WHY: Once tripped (3+ consecutive timeouts), task is permanently failed
-- Prevents infinite retries on documents that are too large for LLM timeout
ALTER TABLE tasks
ADD COLUMN IF NOT EXISTS circuit_breaker_tripped BOOLEAN DEFAULT FALSE NOT NULL;

-- Add error column for structured error information
-- WHY: Store TaskFailureInfo with classification (timeout vs other errors)
ALTER TABLE tasks
ADD COLUMN IF NOT EXISTS error JSONB;

-- Add comments for documentation
COMMENT ON COLUMN tasks.consecutive_timeout_failures IS 'Number of consecutive timeout failures (resets to 0 on success or non-timeout failure). Circuit breaker trips at 3.';
COMMENT ON COLUMN tasks.circuit_breaker_tripped IS 'Whether circuit breaker has permanently failed this task (3+ consecutive timeouts). Prevents infinite retries.';
COMMENT ON COLUMN tasks.error IS 'Structured error information (TaskFailureInfo): message, step, reason, suggestion, retryable, is_timeout.';

-- Create index for querying circuit breaker state
CREATE INDEX IF NOT EXISTS idx_tasks_circuit_breaker ON tasks(circuit_breaker_tripped, status);

-- Create index for timeout tracking
CREATE INDEX IF NOT EXISTS idx_tasks_consecutive_timeouts ON tasks(consecutive_timeout_failures) WHERE consecutive_timeout_failures > 0;

-- ============================================================================
-- Data migration: Identify existing timeout failures
-- ============================================================================

-- Mark existing tasks with "timeout" in error message
-- WHY: Retroactively classify timeout errors for existing failed tasks
UPDATE tasks
SET 
    consecutive_timeout_failures = 1,
    error = jsonb_build_object(
        'message', 'Operation timed out',
        'step', 'extraction',
        'reason', COALESCE(error_message, 'LLM call timed out'),
        'suggestion', 'Document may be too large. Try: 1) Use smaller chunk size (adaptive chunking), 2) Split document, 3) Use provider with longer timeout',
        'retryable', false
    )
WHERE 
    status = 'failed'
    AND error_message IS NOT NULL
    AND (
        LOWER(error_message) LIKE '%timeout%'
        OR LOWER(error_message) LIKE '%timed out%'
    )
    AND error IS NULL;

-- Log migration results
DO $$ 
DECLARE
    updated_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO updated_count
    FROM tasks
    WHERE consecutive_timeout_failures > 0;
    
    RAISE NOTICE 'Migration 020 completed: Added circuit breaker fields';
    RAISE NOTICE 'Updated % existing tasks with timeout classification', updated_count;
END $$;
