-- Folder, pinned, unfiled, and archived conversation pages sorted by
-- created_at or title walk the user-wide sort index and throw away rows
-- that fail the filter. Migration 176 covers those sorts with no extra
-- filter. Migrations 179–181 and the archived index cover the same filters
-- ordered by updated_at. On 15,000 conversations for one user, with the
-- matching rows at the wrong end of the sort, each page removed 10,001 rows
-- (0.792–2.802 ms, 290–489 buffers). These indexes stop at LIMIT
-- (0.029–0.055 ms, 5–7 buffers). A generic plan of `folder_id = $3`,
-- `is_pinned = $3`, and `is_archived = $3` uses the composites.
-- `folder_id IS NULL` is a literal, so the unfiled indexes are partial.
-- The list SQL is unchanged.

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user_folder_created
    ON public.conversations (tenant_id, user_id, folder_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user_folder_title
    ON public.conversations (tenant_id, user_id, folder_id, title);

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user_pinned_created
    ON public.conversations (tenant_id, user_id, is_pinned, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user_pinned_title
    ON public.conversations (tenant_id, user_id, is_pinned, title);

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user_unfiled_created
    ON public.conversations (tenant_id, user_id, created_at DESC)
    WHERE folder_id IS NULL;

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user_unfiled_title
    ON public.conversations (tenant_id, user_id, title)
    WHERE folder_id IS NULL;

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user_archived_created
    ON public.conversations (tenant_id, user_id, is_archived, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user_archived_title
    ON public.conversations (tenant_id, user_id, is_archived, title);
