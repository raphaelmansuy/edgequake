-- SPEC-180: Normal RAG profile (chunks + dense + BM25, no knowledge graph).
-- Expand-only. Existing rows become graph / text / english with no rewrite of workspaces.
--
-- Tokenizer (one, stored in the table, never repeated in the application):
--   chunk_terms holds the lexemes of to_tsvector(workspaces.fts_regconfig, content).
--   tf is the number of positions of the lexeme. Queries use the same function.
-- Writers do not change: statement triggers on public.chunks keep the postings and
-- the per-workspace document frequencies in the same transaction as the chunk rows.

-- ── workspaces: immutable profile columns ───────────────────────────────────
ALTER TABLE public.workspaces
    ADD COLUMN IF NOT EXISTS rag_profile text NOT NULL DEFAULT 'graph',
    ADD COLUMN IF NOT EXISTS embedding_modalities text NOT NULL DEFAULT 'text',
    ADD COLUMN IF NOT EXISTS fts_regconfig text NOT NULL DEFAULT 'english',
    ADD COLUMN IF NOT EXISTS bm25_doc_count integer NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS bm25_total_len bigint NOT NULL DEFAULT 0;

DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'workspaces_rag_profile_chk') THEN
        ALTER TABLE public.workspaces ADD CONSTRAINT workspaces_rag_profile_chk
            CHECK (rag_profile IN ('graph', 'normal'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'workspaces_embedding_modalities_chk') THEN
        ALTER TABLE public.workspaces ADD CONSTRAINT workspaces_embedding_modalities_chk
            CHECK (embedding_modalities IN ('text', 'text,image'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'workspaces_fts_regconfig_chk') THEN
        ALTER TABLE public.workspaces ADD CONSTRAINT workspaces_fts_regconfig_chk
            CHECK (fts_regconfig IN ('english', 'simple', 'french', 'german', 'spanish'));
    END IF;
END $$;

CREATE OR REPLACE FUNCTION public.eq_workspace_profile_immutable() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.rag_profile IS DISTINCT FROM OLD.rag_profile THEN
        RAISE EXCEPTION 'rag_profile_immutable: rag_profile cannot change after create'
            USING ERRCODE = 'check_violation';
    END IF;
    IF NEW.embedding_modalities IS DISTINCT FROM OLD.embedding_modalities THEN
        RAISE EXCEPTION 'embedding_modalities_immutable: embedding_modalities cannot change after create'
            USING ERRCODE = 'check_violation';
    END IF;
    IF NEW.fts_regconfig IS DISTINCT FROM OLD.fts_regconfig THEN
        RAISE EXCEPTION 'fts_regconfig_immutable: fts_regconfig cannot change after create'
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END $$;

DROP TRIGGER IF EXISTS workspaces_rag_profile_immutable ON public.workspaces;
CREATE TRIGGER workspaces_rag_profile_immutable
    BEFORE UPDATE ON public.workspaces
    FOR EACH ROW EXECUTE FUNCTION public.eq_workspace_profile_immutable();

-- ── documents.doc_kind: generated, so no writer can drift from content_type ─
ALTER TABLE public.documents
    ADD COLUMN IF NOT EXISTS doc_kind text
    GENERATED ALWAYS AS (
        CASE
            WHEN content_type LIKE 'image/%' THEN 'image'
            WHEN content_type = 'application/pdf' THEN 'pdf'
            WHEN content_type IN ('text/markdown', 'text/x-markdown') THEN 'markdown'
            WHEN content_type LIKE 'text/%' THEN 'text'
            WHEN content_type IN (
                'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
                'application/msword',
                'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
                'application/vnd.ms-excel',
                'application/vnd.openxmlformats-officedocument.presentationml.presentation',
                'application/vnd.ms-powerpoint'
            ) THEN 'office'
            ELSE 'other'
        END
    ) STORED;

CREATE INDEX IF NOT EXISTS idx_documents_workspace_doc_kind
    ON public.documents (workspace_id, doc_kind);

-- ── chunks.source_modality: what the dense vector was computed from ─────────
ALTER TABLE public.chunks
    ADD COLUMN IF NOT EXISTS source_modality text NOT NULL DEFAULT 'text';

DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'chunks_source_modality_chk') THEN
        ALTER TABLE public.chunks ADD CONSTRAINT chunks_source_modality_chk
            CHECK (source_modality IN ('text', 'image'));
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_chunks_workspace_source_modality
    ON public.chunks (workspace_id, source_modality);

-- ── BM25 postings and per-workspace document frequency ──────────────────────
CREATE TABLE IF NOT EXISTS public.chunk_terms (
    workspace_id uuid NOT NULL REFERENCES public.workspaces(workspace_id) ON DELETE CASCADE,
    chunk_id uuid NOT NULL,
    term text NOT NULL,
    tf integer NOT NULL CHECK (tf > 0),
    PRIMARY KEY (chunk_id, term)
);

CREATE INDEX IF NOT EXISTS chunk_terms_ws_term
    ON public.chunk_terms (workspace_id, term);

CREATE TABLE IF NOT EXISTS public.workspace_term_stats (
    workspace_id uuid NOT NULL REFERENCES public.workspaces(workspace_id) ON DELETE CASCADE,
    term text NOT NULL,
    df integer NOT NULL CHECK (df >= 0),
    PRIMARY KEY (workspace_id, term)
);

-- Insert: write postings, then lock the workspace row (NO KEY UPDATE, so FK checks of
-- parallel ingests are not blocked), then add df. Terms are sorted to keep lock order stable.
CREATE OR REPLACE FUNCTION public.eq_bm25_after_chunk_insert() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM new_rows n
        JOIN public.workspaces w ON w.workspace_id = n.workspace_id
        WHERE w.rag_profile = 'normal'
    ) THEN
        RETURN NULL;
    END IF;

    INSERT INTO public.chunk_terms (workspace_id, chunk_id, term, tf)
    SELECT n.workspace_id, n.id, t.lexeme, GREATEST(cardinality(t.positions), 1)
    FROM new_rows n
    JOIN public.workspaces w ON w.workspace_id = n.workspace_id AND w.rag_profile = 'normal'
    CROSS JOIN LATERAL unnest(to_tsvector(w.fts_regconfig::regconfig, coalesce(n.content, ''))) AS t(lexeme, positions, weights)
    ON CONFLICT (chunk_id, term) DO NOTHING;

    UPDATE public.workspaces w
       SET bm25_doc_count = w.bm25_doc_count + a.chunks,
           bm25_total_len = w.bm25_total_len + a.len
      FROM (
          SELECT n.workspace_id, count(*)::integer AS chunks,
                 coalesce(sum(l.len), 0)::bigint AS len
          FROM new_rows n
          LEFT JOIN (
              SELECT ct.chunk_id, sum(ct.tf) AS len
              FROM public.chunk_terms ct
              WHERE ct.chunk_id IN (SELECT id FROM new_rows)
              GROUP BY ct.chunk_id
          ) l ON l.chunk_id = n.id
          GROUP BY n.workspace_id
      ) a
     WHERE w.workspace_id = a.workspace_id AND w.rag_profile = 'normal';

    INSERT INTO public.workspace_term_stats (workspace_id, term, df)
    SELECT ct.workspace_id, ct.term, count(*)::integer
    FROM public.chunk_terms ct
    WHERE ct.chunk_id IN (SELECT id FROM new_rows)
    GROUP BY ct.workspace_id, ct.term
    ORDER BY ct.workspace_id, ct.term
    ON CONFLICT (workspace_id, term)
    DO UPDATE SET df = public.workspace_term_stats.df + EXCLUDED.df;

    RETURN NULL;
END $$;

-- Delete: read the postings of the removed chunks (cascade from documents and workspaces
-- also lands here), lower df, lower the aggregates, then drop the postings.
CREATE OR REPLACE FUNCTION public.eq_bm25_after_chunk_delete() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM public.chunk_terms ct WHERE ct.chunk_id IN (SELECT id FROM old_rows)
    ) AND NOT EXISTS (
        SELECT 1 FROM old_rows o
        JOIN public.workspaces w ON w.workspace_id = o.workspace_id
        WHERE w.rag_profile = 'normal'
    ) THEN
        RETURN NULL;
    END IF;

    UPDATE public.workspaces w
       SET bm25_doc_count = GREATEST(w.bm25_doc_count - a.chunks, 0),
           bm25_total_len = GREATEST(w.bm25_total_len - a.len, 0)
      FROM (
          SELECT o.workspace_id, count(*)::integer AS chunks,
                 coalesce(sum(l.len), 0)::bigint AS len
          FROM old_rows o
          LEFT JOIN (
              SELECT ct.chunk_id, sum(ct.tf) AS len
              FROM public.chunk_terms ct
              WHERE ct.chunk_id IN (SELECT id FROM old_rows)
              GROUP BY ct.chunk_id
          ) l ON l.chunk_id = o.id
          GROUP BY o.workspace_id
      ) a
     WHERE w.workspace_id = a.workspace_id AND w.rag_profile = 'normal';

    UPDATE public.workspace_term_stats s
       SET df = GREATEST(s.df - d.c, 0)
      FROM (
          SELECT ct.workspace_id, ct.term, count(*)::integer AS c
          FROM public.chunk_terms ct
          WHERE ct.chunk_id IN (SELECT id FROM old_rows)
          GROUP BY ct.workspace_id, ct.term
      ) d
     WHERE s.workspace_id = d.workspace_id AND s.term = d.term;

    DELETE FROM public.workspace_term_stats s
     WHERE s.df = 0
       AND s.workspace_id IN (SELECT workspace_id FROM old_rows);

    DELETE FROM public.chunk_terms ct WHERE ct.chunk_id IN (SELECT id FROM old_rows);
    RETURN NULL;
END $$;

DROP TRIGGER IF EXISTS chunks_bm25_after_insert ON public.chunks;
CREATE TRIGGER chunks_bm25_after_insert
    AFTER INSERT ON public.chunks
    REFERENCING NEW TABLE AS new_rows
    FOR EACH STATEMENT EXECUTE FUNCTION public.eq_bm25_after_chunk_insert();

DROP TRIGGER IF EXISTS chunks_bm25_after_delete ON public.chunks;
CREATE TRIGGER chunks_bm25_after_delete
    AFTER DELETE ON public.chunks
    REFERENCING OLD TABLE AS old_rows
    FOR EACH STATEMENT EXECUTE FUNCTION public.eq_bm25_after_chunk_delete();

-- Repair: recompute the postings and aggregates of one workspace from chunks.content.
CREATE OR REPLACE FUNCTION public.rebuild_bm25(p_workspace uuid) RETURNS void
LANGUAGE plpgsql AS $$
DECLARE
    cfg regconfig;
BEGIN
    SELECT fts_regconfig::regconfig INTO cfg
      FROM public.workspaces WHERE workspace_id = p_workspace AND rag_profile = 'normal'
      FOR NO KEY UPDATE;
    IF cfg IS NULL THEN
        RETURN;
    END IF;
    DELETE FROM public.chunk_terms WHERE workspace_id = p_workspace;
    DELETE FROM public.workspace_term_stats WHERE workspace_id = p_workspace;
    INSERT INTO public.chunk_terms (workspace_id, chunk_id, term, tf)
    SELECT c.workspace_id, c.id, t.lexeme, GREATEST(cardinality(t.positions), 1)
    FROM public.chunks c
    CROSS JOIN LATERAL unnest(to_tsvector(cfg, coalesce(c.content, ''))) AS t(lexeme, positions, weights)
    WHERE c.workspace_id = p_workspace;
    INSERT INTO public.workspace_term_stats (workspace_id, term, df)
    SELECT workspace_id, term, count(*)::integer
    FROM public.chunk_terms WHERE workspace_id = p_workspace
    GROUP BY workspace_id, term;
    UPDATE public.workspaces
       SET bm25_doc_count = (SELECT count(*) FROM public.chunks WHERE workspace_id = p_workspace),
           bm25_total_len = coalesce((SELECT sum(tf) FROM public.chunk_terms WHERE workspace_id = p_workspace), 0)
     WHERE workspace_id = p_workspace;
END $$;

-- ── RLS: a term row is visible only inside its workspace (same shape as chunk_embeddings) ─
DO $$
DECLARE t text; predicate text;
BEGIN
    FOREACH t IN ARRAY ARRAY['chunk_terms', 'workspace_term_stats'] LOOP
        predicate := 'workspace_id = public.current_workspace_id() AND EXISTS (SELECT 1 FROM public.workspaces w WHERE w.workspace_id = public.current_workspace_id() AND w.tenant_id = public.current_tenant_id())';
        EXECUTE format('ALTER TABLE public.%I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('ALTER TABLE public.%I FORCE ROW LEVEL SECURITY', t);
        IF NOT EXISTS (SELECT 1 FROM pg_policies WHERE schemaname = 'public' AND tablename = t AND policyname = 'tenant_access_allow') THEN
            EXECUTE format('CREATE POLICY tenant_access_allow ON public.%I FOR ALL TO edgequake_tenant_access USING (true) WITH CHECK (true)', t);
        END IF;
        IF NOT EXISTS (SELECT 1 FROM pg_policies WHERE schemaname = 'public' AND tablename = t AND policyname = 'tenant_access_guard') THEN
            EXECUTE format('CREATE POLICY tenant_access_guard ON public.%I AS RESTRICTIVE FOR ALL TO PUBLIC USING (%s) WITH CHECK (%s)', t, predicate, predicate);
        END IF;
    END LOOP;
END $$;

DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'edgequake_tenant_access') THEN
        GRANT SELECT, INSERT, UPDATE, DELETE ON public.chunk_terms, public.workspace_term_stats
            TO edgequake_tenant_access;
    END IF;
END $$;

-- ── HNSW for 3072 (gemini-embedding-2 native size); 768 and 1536 exist since 132 ─
CREATE INDEX IF NOT EXISTS idx_chunk_embeddings_hnsw_d3072
    ON public.chunk_embeddings
    USING hnsw ((embedding::halfvec(3072)) halfvec_cosine_ops)
    WITH (m = 16, ef_construction = 128)
    WHERE dimensions = 3072;
