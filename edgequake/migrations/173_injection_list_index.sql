-- Workspace injection lists filter `metadata->>'source_type' = 'injection'`
-- and then take a created_at page. That predicate is not in
-- idx_documents_workspace_created, so a miss walks every document in the
-- workspace. A partial index contains only injection rows.

CREATE INDEX IF NOT EXISTS idx_documents_workspace_injection
    ON public.documents (workspace_id, created_at DESC, id)
    WHERE (metadata->>'source_type') = 'injection';
