-- Conversation folder pages are
-- `WHERE tenant_id = $1 AND user_id = $2 AND folder_id = $3 ORDER BY updated_at DESC`.
-- `idx_conversations_tenant_user` is `(tenant_id, user_id, updated_at DESC)`, so a
-- folder of older conversations walks newer rows and throws them away. On 15,000
-- conversations for one user, with the 5,000 oldest in the folder, that filter
-- removed 10,001 rows (5.204 ms, 10,706 buffers). This index stops at LIMIT.

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user_folder_updated
    ON public.conversations (tenant_id, user_id, folder_id, updated_at DESC);
