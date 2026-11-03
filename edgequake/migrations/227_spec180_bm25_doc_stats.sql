-- SPEC-180 follow-up: BM25 document statistics must count documents that have terms.
--
-- WHY this migration exists (measured, not theorised):
--
-- Migration 225's triggers added one to `workspaces.bm25_doc_count` for every chunk row,
-- including a chunk whose content is empty. An image chunk in a normal workspace is exactly
-- that shape: `source_modality = 'image'`, `content = ''` (the pixels are embedded, the text is
-- empty on purpose — see `multimodal_admission::resolve_normal_image`). So a workspace holding
-- four photographs and no text reported:
--
--     bm25_doc_count = 4, bm25_total_len = 0
--     avgdl = greatest(0,1)::float8 / greatest(4,1) = 0.25
--
-- `avgdl` is the BM25 average document length, and it divides into every length-normalised
-- score: `tf + k1 * (1 - b + b * len / avgdl)`. With `avgdl = 0.25` a three-term chunk has
-- `len / avgdl = 12` instead of ~1, which multiplies the denominator by roughly twelve and
-- collapses that chunk's score. A workspace that started as pictures and later received text
-- therefore ranked its text badly — silently, because the numbers are plausible.
--
-- The delete trigger had the mirror defect: it subtracted one per removed chunk, so deleting
-- one of those four empty chunks drove the count to 3 even though none of them had ever
-- contributed a term, and `GREATEST(..., 0)` hid the drift instead of surfacing it.
--
-- The rule BM25 actually wants: N and avgdl are computed over documents that contain terms.
-- A document with no indexed term is not in the lexicon, so counting it inflates `idf` and
-- distorts the length norm. This migration makes insert, delete and rebuild agree on that.
--
-- Expand-only: both triggers are `CREATE OR REPLACE FUNCTION`; no column, index or constraint
-- changes, and no existing row is rewritten. Existing workspaces are repaired in place at the
-- end, because their counters were written by the old functions.

-- ── Insert: count only the chunks that actually produced a term ─────────────
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

    -- `l.len IS NULL` is a chunk that produced no term (empty content, stopwords only, or an
    -- image chunk). It must not enter N or the length sum, or avgdl is wrong for every query.
    UPDATE public.workspaces w
       SET bm25_doc_count = w.bm25_doc_count + a.chunks,
           bm25_total_len = w.bm25_total_len + a.len
      FROM (
          SELECT n.workspace_id,
                 count(*) FILTER (WHERE l.len IS NOT NULL)::integer AS chunks,
                 coalesce(sum(l.len) FILTER (WHERE l.len IS NOT NULL), 0)::bigint AS len
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

-- ── Delete: subtract exactly what insert added ──────────────────────────────
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

    -- Same filter as insert, so the two directions cannot drift apart: a chunk that never
    -- contributed a term is not subtracted here either.
    UPDATE public.workspaces w
       SET bm25_doc_count = GREATEST(w.bm25_doc_count - a.chunks, 0),
           bm25_total_len = GREATEST(w.bm25_total_len - a.len, 0)
      FROM (
          SELECT o.workspace_id,
                 count(*) FILTER (WHERE l.len IS NOT NULL)::integer AS chunks,
                 coalesce(sum(l.len) FILTER (WHERE l.len IS NOT NULL), 0)::bigint AS len
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

-- ── Rebuild: derive N from the postings, never from the chunk count ─────────
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
    -- N is the number of chunks that carry a term, which is exactly the number of distinct
    -- `chunk_terms.chunk_id` — not `count(*) FROM chunks`, which included the empty ones.
    UPDATE public.workspaces
       SET bm25_doc_count = (SELECT count(DISTINCT chunk_id) FROM public.chunk_terms WHERE workspace_id = p_workspace),
           bm25_total_len = coalesce((SELECT sum(tf) FROM public.chunk_terms WHERE workspace_id = p_workspace), 0)
     WHERE workspace_id = p_workspace;
END $$;

-- ── Repair the workspaces the old functions already counted ────────────────
-- Recomputed from the postings themselves, so this is the same answer a fresh insert would
-- give. `bm25_total_len` needs no repair (it summed `tf`, which empty chunks never had), but
-- it is recomputed in the same statement so the pair is consistent by construction.
UPDATE public.workspaces w
   SET bm25_doc_count = s.chunks,
       bm25_total_len = s.len
  FROM (
      SELECT ws.workspace_id,
             (SELECT count(DISTINCT ct.chunk_id)::integer
                FROM public.chunk_terms ct
               WHERE ct.workspace_id = ws.workspace_id) AS chunks,
             (SELECT coalesce(sum(ct.tf), 0)::bigint
                FROM public.chunk_terms ct
               WHERE ct.workspace_id = ws.workspace_id) AS len
        FROM public.workspaces ws
       WHERE ws.rag_profile = 'normal'
  ) s
 WHERE w.workspace_id = s.workspace_id
   AND (w.bm25_doc_count IS DISTINCT FROM s.chunks
     OR w.bm25_total_len IS DISTINCT FROM s.len);
