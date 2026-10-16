-- Conversation search inside a folder is
-- `tenant_id = $1 AND user_id = $2 AND folder_id = $3
--  AND to_tsvector('english', title) @@ plainto_tsquery('english', $4)
--  ORDER BY updated_at DESC LIMIT 50`.
-- Migration 186 indexes `(tenant_id, user_id, title)`. A generic plan
-- bitmaps every title match for the user (10,050 index entries) and only
-- then intersects `idx_conversations_folder` (1.159 ms). Putting `folder_id`
-- in the same GIN returns the 50 hits (0.169 ms, 24 buffers). `btree_gin`
-- is already installed by migration 186. The list SQL is unchanged.

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user_folder_title_fts
    ON public.conversations USING gin (
        tenant_id public.uuid_ops,
        user_id public.uuid_ops,
        folder_id public.uuid_ops,
        to_tsvector('english', title::text)
    );
