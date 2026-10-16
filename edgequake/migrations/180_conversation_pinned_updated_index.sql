-- Pinned conversation pages are
-- `WHERE tenant_id = $1 AND user_id = $2 AND is_pinned = $3 ORDER BY updated_at DESC`.
-- `idx_conversations_pinned` is `(tenant_id, user_id, is_pinned) WHERE is_pinned`
-- and has no sort column. `idx_conversations_tenant_user` is
-- `(tenant_id, user_id, updated_at DESC)`, so a page of older pinned rows walks
-- newer rows and throws them away. On 15,000 conversations for one user, with
-- the 4,999 oldest pinned, that filter removed 10,001 rows (3.889 ms, 10,476
-- buffers). The list binds `is_pinned`, so a partial `WHERE is_pinned` index
-- is not implied by a generic plan. This index matches
-- `idx_conversations_archived` and stops at LIMIT (0.065 ms, 52 buffers).

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user_pinned_updated
    ON public.conversations (tenant_id, user_id, is_pinned, updated_at DESC);
