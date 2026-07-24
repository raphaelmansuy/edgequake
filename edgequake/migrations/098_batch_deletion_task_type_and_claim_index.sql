-- Migration 098: batch_deletion task type + workspace-fair claim index (SPEC-084)
--
-- WHY:
--   * TaskType::BatchDeletion (GH-317) must persist to tasks.task_type.
--     Migration 095 allowed workspace_wipe but not batch_deletion.
--   * GH-316 claim_next selects by (status, workspace_id, created_at); a
--     supporting index keeps workspace-fair SKIP LOCKED plans cheap.
--
-- SAFE: DROP IF EXISTS + recreate constraint; CREATE INDEX IF NOT EXISTS.

ALTER TABLE tasks DROP CONSTRAINT IF EXISTS valid_task_type;
ALTER TABLE tasks DROP CONSTRAINT IF EXISTS tasks_valid_type;

ALTER TABLE tasks ADD CONSTRAINT valid_task_type CHECK (
    task_type IN (
        'upload',
        'insert',
        'scan',
        'reindex',
        'pdf_processing',
        'knowledge_injection',
        'deletion',
        'batch_deletion',
        'workspace_wipe'
    )
);

CREATE INDEX IF NOT EXISTS idx_tasks_claim_workspace_created
    ON tasks (status, workspace_id, created_at ASC)
    WHERE status IN ('pending', 'processing');

COMMENT ON INDEX idx_tasks_claim_workspace_created IS
    'SPEC-084 / GH-316: workspace-fair claim_next (status + workspace + created_at)';
