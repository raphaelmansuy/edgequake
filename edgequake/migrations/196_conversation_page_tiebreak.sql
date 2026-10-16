-- Conversation and message pages order by a sort column and then the row id.
-- `idx_conversations_tenant_user` is `(tenant_id, user_id, updated_at DESC)`.
-- On 15,000 conversations for one user, `updated_at` has two values. A keyset
-- `(updated_at, conversation_id) < (cursor, id)` then filters 14,980 rows
-- (2.878 ms, 310 buffers). The first page with the same ORDER BY reads all
-- 15,000 (3.568 ms, 318 buffers). These indexes store the id tie-break, so
-- the row comparison is an index condition and LIMIT stops (0.045 ms, 4
-- buffers; first page 0.027 ms, 4 buffers).
--
-- Message `created_at` is unique in the measured thread, so
-- `idx_messages_conversation` already stops (0.106 ms, one row filtered).
-- `idx_messages_conversation_created_id` keeps the same stop when two
-- messages share `created_at` (0.045 ms, index condition).
--
-- Filtered pages use the same ORDER BY. Each filter index from 176 and
-- 179–183 gains the id column. Expand only. The older indexes stay.

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user_updated_id
    ON public.conversations (tenant_id, user_id, updated_at DESC, conversation_id DESC);

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user_created_id
    ON public.conversations (tenant_id, user_id, created_at DESC, conversation_id DESC);

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user_title_id
    ON public.conversations (tenant_id, user_id, title, conversation_id);

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user_folder_updated_id
    ON public.conversations (tenant_id, user_id, folder_id, updated_at DESC, conversation_id DESC);

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user_folder_created_id
    ON public.conversations (tenant_id, user_id, folder_id, created_at DESC, conversation_id DESC);

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user_folder_title_id
    ON public.conversations (tenant_id, user_id, folder_id, title, conversation_id);

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user_pinned_updated_id
    ON public.conversations (tenant_id, user_id, is_pinned, updated_at DESC, conversation_id DESC);

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user_pinned_created_id
    ON public.conversations (tenant_id, user_id, is_pinned, created_at DESC, conversation_id DESC);

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user_pinned_title_id
    ON public.conversations (tenant_id, user_id, is_pinned, title, conversation_id);

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user_archived_updated_id
    ON public.conversations (tenant_id, user_id, is_archived, updated_at DESC, conversation_id DESC);

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user_archived_created_id
    ON public.conversations (tenant_id, user_id, is_archived, created_at DESC, conversation_id DESC);

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user_archived_title_id
    ON public.conversations (tenant_id, user_id, is_archived, title, conversation_id);

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user_unfiled_updated_id
    ON public.conversations (tenant_id, user_id, updated_at DESC, conversation_id DESC)
    WHERE folder_id IS NULL;

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user_unfiled_created_id
    ON public.conversations (tenant_id, user_id, created_at DESC, conversation_id DESC)
    WHERE folder_id IS NULL;

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user_unfiled_title_id
    ON public.conversations (tenant_id, user_id, title, conversation_id)
    WHERE folder_id IS NULL;

CREATE INDEX IF NOT EXISTS idx_messages_conversation_created_id
    ON public.messages (conversation_id, created_at, message_id);
