-- SPEC-027 phase 34: Default tenant/workspace + membership backfill for auth users.

SET search_path = public;

INSERT INTO tenants (tenant_id, name, slug, is_active, metadata, settings, created_at, updated_at)
VALUES (
    '00000000-0000-0000-0000-000000000002'::uuid,
    'Default',
    'default',
    TRUE,
    '{"plan": "pro", "max_workspaces": 100, "max_users": 100, "description": "Default tenant"}'::jsonb,
    '{}'::jsonb,
    NOW(),
    NOW()
)
ON CONFLICT (tenant_id) DO NOTHING;

INSERT INTO workspaces (workspace_id, tenant_id, name, slug, description, is_active, metadata, settings, created_at, updated_at)
VALUES (
    '00000000-0000-0000-0000-000000000003'::uuid,
    '00000000-0000-0000-0000-000000000002'::uuid,
    'Default Workspace',
    'default',
    'Default knowledge base',
    TRUE,
    '{}'::jsonb,
    '{}'::jsonb,
    NOW(),
    NOW()
)
ON CONFLICT (workspace_id) DO NOTHING;

INSERT INTO memberships (tenant_id, workspace_id, user_id, role, is_active)
SELECT
    '00000000-0000-0000-0000-000000000002'::uuid,
    '00000000-0000-0000-0000-000000000003'::uuid,
    u.user_id,
    CASE
        WHEN lower(u.role) = 'admin' THEN 'admin'
        WHEN lower(u.role) = 'readonly' THEN 'readonly'
        ELSE 'member'
    END,
    TRUE
FROM users u
WHERE NOT EXISTS (
    SELECT 1
    FROM memberships m
    WHERE m.user_id = u.user_id
      AND m.tenant_id = '00000000-0000-0000-0000-000000000002'::uuid
      AND m.workspace_id = '00000000-0000-0000-0000-000000000003'::uuid
);

COMMENT ON TABLE memberships IS
    'Tenant/workspace membership — synced from auth user persist + bootstrap backfill (SPEC-027 phase 34)';
