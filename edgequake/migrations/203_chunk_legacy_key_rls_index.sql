-- Id-filtered chunk lookup compared `metadata->>'legacy_chunk_key'`.
-- `jsonb ->>` (`jsonb_object_field_text`) is not leakproof, so under forced
-- row security that predicate cannot be an index condition. As
-- `edgequake_tenant_access`, each key bitmap-scanned
-- `idx_chunks_tenant_workspace` and filtered the key afterwards:
-- 1,446 keys, 18.217 s, 1,063,203 buffer hits. The vector statement
-- timeout is 1.75 s, so mix-mode chunk rerank was cancelled.
--
-- A stored text column makes `legacy_chunk_key = wanted.key` leakproof.
-- `idx_chunks_tenant_legacy_key` then satisfies the tenant policy and the
-- key together. The same 1,446 keys are an index scan at 0.006 ms each
-- (Index Cond: tenant_id = current_tenant_id() AND legacy_chunk_key = key).

ALTER TABLE public.chunks
    ADD COLUMN legacy_chunk_key text
    GENERATED ALWAYS AS (metadata ->> 'legacy_chunk_key') STORED;

CREATE INDEX idx_chunks_tenant_legacy_key
    ON public.chunks (tenant_id, legacy_chunk_key)
    WHERE legacy_chunk_key IS NOT NULL;
