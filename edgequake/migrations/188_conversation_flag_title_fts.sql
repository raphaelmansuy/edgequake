-- Pinned and archived conversation search is
-- `tenant_id = $1 AND user_id = $2 AND is_pinned = $3
--  AND to_tsvector('english', title) @@ plainto_tsquery('english', $4)
--  ORDER BY updated_at DESC LIMIT 50`
-- (the same shape with `is_archived`).
-- Migration 186 indexes `(tenant_id, user_id, title)`. A generic plan bitmaps
-- every title match for the user and then throws away the other flag.
-- On 10,000 title matches with 50 pinned, that removed 9,950 rows
-- (2.725 ms, 464 buffers). Archived search did the same (2.211 ms).
-- One GIN on the user, both flags, and the title returns the 50 hits
-- (pinned 0.205 ms / 24 buffers, archived 0.158 ms / 25 buffers).
-- `btree_gin` is already installed by migration 186. The list SQL is unchanged.

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user_flags_title_fts
    ON public.conversations USING gin (
        tenant_id public.uuid_ops,
        user_id public.uuid_ops,
        is_pinned public.bool_ops,
        is_archived public.bool_ops,
        to_tsvector('english', title::text)
    );
