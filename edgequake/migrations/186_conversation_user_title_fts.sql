-- Conversation title search is
-- `tenant_id = $1 AND user_id = $2
--  AND to_tsvector('english', title) @@ plainto_tsquery('english', $3)
--  ORDER BY updated_at DESC LIMIT $4`.
-- `idx_conversations_title_fts` is a GIN on the title alone. A generic plan
-- bitmaps every title that matches the word, then filters the user. On
-- 10,000 matches that belong to another user plus 50 that belong to the
-- caller, that removed 10,000 rows (1.922 ms, 577 buffers).
-- `btree_gin` is trusted contrib (PostgreSQL 13+). RDS and Aurora list it
-- and let a role with CREATE install it; it does not need
-- shared_preload_libraries and it does not need Apache AGE. Pin it to
-- public, the same way migration 170 pins `pg_trgm`. The user-scoped GIN
-- returns the 50 hits (1.259 ms, 24 buffers). The list SQL is unchanged.

DO $$
DECLARE
    ext_schema text;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_catalog.pg_extension WHERE extname = 'btree_gin') THEN
        CREATE EXTENSION btree_gin WITH SCHEMA public;
    END IF;
    SELECT n.nspname INTO ext_schema
    FROM pg_catalog.pg_extension e
    JOIN pg_catalog.pg_namespace n ON n.oid = e.extnamespace
    WHERE e.extname = 'btree_gin';
    IF ext_schema IS DISTINCT FROM 'public' THEN
        ALTER EXTENSION btree_gin SET SCHEMA public;
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_conversations_tenant_user_title_fts
    ON public.conversations USING gin (
        tenant_id public.uuid_ops,
        user_id public.uuid_ops,
        to_tsvector('english', title::text)
    );
