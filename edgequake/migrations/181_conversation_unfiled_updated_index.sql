-- Unfiled conversation pages are
-- `WHERE tenant_id = $1 AND user_id = $2 AND folder_id IS NULL ORDER BY updated_at DESC`.
-- `idx_conversations_tenant_user` is `(tenant_id, user_id, updated_at DESC)`, so
-- a page of older unfiled rows walks newer filed rows and throws them away.
-- On 15,000 conversations for one user, with the 10,001 newest filed, that
-- filter removed 10,001 rows (2.196 ms, 10,078 buffers). `folder_id IS NULL`
-- is a literal in the list, so this partial index is usable by a generic plan
-- and stops at LIMIT (0.046 ms, 53 buffers).

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user_unfiled_updated
    ON public.conversations (tenant_id, user_id, updated_at DESC)
    WHERE folder_id IS NULL;
