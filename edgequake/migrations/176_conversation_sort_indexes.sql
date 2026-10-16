-- Conversation lists are `WHERE tenant_id = $1 AND user_id = $2`.
-- The default order is `updated_at`, which `idx_conversations_tenant_user` serves.
-- `ORDER BY created_at` and `ORDER BY title` have no matching index, so each
-- page reads every conversation for that user and sorts it.

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user_created
    ON public.conversations (tenant_id, user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user_title
    ON public.conversations (tenant_id, user_id, title);
