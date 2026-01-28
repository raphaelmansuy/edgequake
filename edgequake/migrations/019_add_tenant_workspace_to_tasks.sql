-- Migration: 018_add_tenant_workspace_to_tasks
-- Description: Add tenant_id and workspace_id to tasks table for multi-tenancy isolation
-- Phase: 1.2.0
-- Date: 2025-01-28
-- Issue: Tasks were globally visible across all tenants/workspaces - CRITICAL SECURITY FIX

SET search_path = public;

-- ============================================================================
-- STEP 1: Add tenant_id and workspace_id columns
-- ============================================================================

-- Add columns (allow NULL initially for existing rows)
ALTER TABLE tasks 
ADD COLUMN IF NOT EXISTS tenant_id UUID,
ADD COLUMN IF NOT EXISTS workspace_id UUID;

-- ============================================================================
-- STEP 2: Migrate existing data
-- ============================================================================

-- For existing tasks, try to extract tenant_id/workspace_id from payload JSON
-- If not available, use a default tenant (adjust as needed for your data)
UPDATE tasks 
SET 
    tenant_id = COALESCE(
        (payload->>'tenant_id')::UUID,
        '00000000-0000-0000-0000-000000000000'::UUID
    ),
    workspace_id = COALESCE(
        (payload->>'workspace_id')::UUID,
        '00000000-0000-0000-0000-000000000000'::UUID
    )
WHERE tenant_id IS NULL OR workspace_id IS NULL;

-- ============================================================================
-- STEP 3: Add constraints
-- ============================================================================

-- Make columns NOT NULL after data migration
ALTER TABLE tasks 
ALTER COLUMN tenant_id SET NOT NULL,
ALTER COLUMN workspace_id SET NOT NULL;

-- ============================================================================
-- STEP 4: Create indexes for performance
-- ============================================================================

-- Composite index for filtering by tenant/workspace
CREATE INDEX IF NOT EXISTS idx_tasks_tenant_workspace 
ON tasks(tenant_id, workspace_id);

-- Composite index for common query patterns
CREATE INDEX IF NOT EXISTS idx_tasks_tenant_workspace_status 
ON tasks(tenant_id, workspace_id, status, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_tasks_tenant_workspace_type 
ON tasks(tenant_id, workspace_id, task_type);

-- ============================================================================
-- STEP 5: Add foreign key constraints (if tenants/workspaces tables exist)
-- ============================================================================

-- Note: Uncomment if you have tenants and workspaces tables
-- ALTER TABLE tasks 
-- ADD CONSTRAINT fk_tasks_tenant 
-- FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE;

-- ALTER TABLE tasks 
-- ADD CONSTRAINT fk_tasks_workspace 
-- FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE;

-- ============================================================================
-- STEP 6: Add RLS policies for tenant isolation
-- ============================================================================

-- Enable RLS on tasks table
ALTER TABLE tasks ENABLE ROW LEVEL SECURITY;

-- Drop existing policies if they exist (make migration idempotent)
DROP POLICY IF EXISTS tasks_tenant_isolation ON tasks;
DROP POLICY IF EXISTS tasks_service_role_all ON tasks;

-- Policy: Users can only see tasks in their tenant
CREATE POLICY tasks_tenant_isolation ON tasks
    FOR ALL
    USING (tenant_id = current_setting('app.current_tenant_id', TRUE)::UUID);

-- Policy: Service role can see all tasks (for admin operations)
-- Note: service_role may not exist, so this might fail - that's okay
DO $$ 
BEGIN
    CREATE POLICY tasks_service_role_all ON tasks
        FOR ALL
        TO service_role
        USING (true);
EXCEPTION
    WHEN undefined_object THEN
        RAISE NOTICE 'service_role does not exist, skipping service role policy';
END $$;

-- Success message
DO $$ BEGIN
    RAISE NOTICE 'Migration 018 completed: Added tenant_id and workspace_id to tasks table with indexes and RLS policies!';
END $$;
