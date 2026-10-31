# 01 — First principles

Parent: [README](README.md) · Prev: [00-why](00-why.md) · Next: [02-research](02-research.md)

A law is a rule a reviewer can check in a diff. If a change breaks a law, the change is wrong even when the tests that existed before it still pass.

## LAW-180-1 — The profile is the shape of the data

`rag_profile` is `graph` or `normal`. It is written at `POST /tenants/{tenant_id}/workspaces` and never updated.

- `PUT /workspaces/{id}` that includes `rag_profile` returns **409** `rag_profile_immutable`, whether the value is the same or different.
- The database trigger `workspaces_rag_profile_immutable` raises if the column would change. The API check is not the only lock.
- There is no migration job from graph to normal. The operator creates a new workspace and re-ingests.
- A document cannot carry its own profile ([LAW-180-13](#law-180-13--no-per-document-profile)).

WHY: [WHY-A](00-why.md). A setting that can drift from the stored graph is a lie.

## LAW-180-2 — Capabilities are queried, not scattered

`ProfileCapabilities` is a pure struct derived from `RagProfile`. It is the only place that maps the enum to booleans (`extracts_graph`, `runs_summary_llm`, `merges_graph`, `refreshes_communities`, `queries_graph`, `shows_graph_ui`, `allows_image_query`, …).

- Ingest, query, delete, MCP, and UI call methods on this struct.
- A new `if profile == normal` outside that struct is a review defect.
- Adding a third profile later means one new arm in the constructor, not a sweep of the worker.

This is the Open/Closed and DRY rule for the pack. The Dependency Inversion rule is: the worker depends on the capability struct, not on Postgres or on the enum's wire string.

WHY: [WHY-D](00-why.md). Flags inside a graph pipeline are how `ChunkOnly` leaked graph work ([F-180-01](04-code-findings.md)).

## LAW-180-3 — Normal ingest calls no chat LLM

For `rag_profile = normal` the worker does not:

- construct an extraction LLM, or call `judge_ingest` for `Usage::Extract`
- glean, extract entities, extract relations, or inject modality graph edges
- resolve a summary LLM or call `Usage::Summary`
- merge into graph storage, or schedule community refresh
- write entity or relationship embeddings

It does: chunk, embed chunks (text and, when enabled, images), write the lexical index, and mark the document completed.

A test counts chat-LLM calls during ingest and expects **zero** ([T-180-ingest-zero-chat](16-e2e-test-matrix.md)).

WHY: [WHY-A](00-why.md). The cost story is false if a hidden summary call remains ([F-180-06](04-code-findings.md)).

## LAW-180-4 — Embeddings fail closed

- If the configured embedding provider has no credential, ingest and query return an error. They do not silently use `MockProvider`.
- `gemini-embedding-2` is called with `:embedContent`. A `:predict` response is not accepted as success.
- Inputs over the model token limit are split or rejected. The worker does not rely on the vendor's silent truncation.
- Image bytes the model cannot accept (format, dimensions, corrupt file) fail that asset with a coded error. They are not dropped without a status.

WHY: [WHY-B](00-why.md). The Vertex factory in `edgequake-llm` 0.10.9 logs and returns a mock when credentials are missing ([F-180-17](04-code-findings.md)).

## LAW-180-5 — One vector space per workspace

A workspace has one embedding provider, one model, one dimension, and one modality set (`text` or `text,image`).

- Text chunks and image chunks in that workspace share the dimension. Cosine is defined.
- Changing the model or the dimension uses `POST /workspaces/{id}/rebuild-embeddings`. It does not mix dimensions in `chunk_embeddings`.
- `text,image` requires a catalog card with `embeds_image = true`. Otherwise create returns **422** `embedding_model_not_multimodal`.
- Recommended stored dimension for `gemini-embedding-2` is **1536** (an existing HNSW width). **768** is allowed (tests, smaller index). **3072** is allowed only after migration 225 builds the partial HNSW for 3072. Other widths in 128–3072 are rejected at create so the index set stays finite.

WHY: [WHY-B](00-why.md). MRL prefixes are comparable only inside one model and one width.

## LAW-180-6 — Fuse ranks, not raw scores

Normal retrieval:

1. Dense top-N by cosine.
2. BM25 top-N.
3. Reciprocal rank fusion, `score = Σ weight / (k + rank + 1)`, `k = 60`, default weights dense 1.0 and BM25 1.25 (the weights already chosen in [sparse_retrieval.rs](../../edgequake/crates/edgequake-query/src/sparse_retrieval.rs) line 178; they become config, not literals).
4. Optional reranker on the fused head.

`min_score` (cosine / entity threshold, default 0.1) is **not** applied to RRF scores. A missing leg returns the other leg. It does not return an empty list.

WHY: [WHY-C](00-why.md). [F-180-11](04-code-findings.md), [F-180-12](04-code-findings.md). RRF is the fusion Cormack et al. used because scores are not on one scale (R-180-3).

## LAW-180-7 — The lexical leg is BM25, without a required extension

BM25 (k1 = 1.5, b = 0.75) ranks candidates using corpus document frequency stored per workspace.

- The default backend loads candidates with Postgres full-text search using **OR** semantics (`to_tsquery` of stemmed terms, not `websearch_to_tsquery` AND), then rescores them with BM25 in the database or in a set-returning SQL function shipped by EdgeQuake. No `CREATE EXTENSION` beyond `vector`, `pg_trgm`, and `btree_gin`.
- If `pg_search` (or another BM25 access method) is already installed, `LexicalIndex` may use it. Detection is at boot. Absence is not an error.
- The text-search configuration is per workspace (`english` default, `simple` when the corpus is mixed-language). It is not hard-coded `'english'` in new SQL for normal workspaces.
- Graph workspaces keep today's `ts_rank_cd` path. This pack does not rewrite it.

WHY: [WHY-C](00-why.md). Extension BM25 is AGPL or preload-gated and is not in the EdgeQuake Postgres image (R-180-4).

## LAW-180-8 — Graph modes are refused; the default is normal retrieval

On a normal workspace:

- Omitted `mode` runs `NormalRetriever` (dense + BM25 + RRF + optional rerank). It does not run Mix.
- `mode=naive` is accepted as an alias of that retriever (the UI label "Chunks" keeps working).
- `mode=bypass` (chat with no retrieval) is accepted.
- `local`, `global`, `hybrid`, and `mix` return **422** `mode_not_supported_by_profile`.
- The pipeline does not call keyword-LLM extraction, `validate_keywords`, `admit_into_context`, or `expand_entities_named_in_query`.

On a graph workspace, modes are unchanged. Mix stays the default.

WHY: [WHY-D](00-why.md).

## LAW-180-9 — Shared space, labeled modality

- An image chunk and a text chunk are rows in `chunk_embeddings` for the same model id.
- `chunks.source_modality` is `text` or `image`. It does not replace `chunks.modality` (chart / figure / table / equation).
- A figure plus its caption is one embedding request with two parts (one aggregated vector), matching the vendor rule that multiple parts in one `Content` collapse to one vector (R-180-5).
- Query by image embeds the image alone. Query by text embeds the text with the query prefix. Fusion stays rank-based so a modality gap in cosine does not drop a hit ([09-retrieval-and-fusion](09-retrieval-and-fusion.md)).

## LAW-180-10 — Kind scope is a filter, and clearance still wins

`DocumentFilter` gains `doc_kinds`, `mime_types`, and `source_modalities`.

- They AND with dates and OR with the existing id / pattern membership, then intersect the clearance set from SPEC-174.
- An empty list means "no constraint," same as today's empty `document_ids`.
- SQL applies the predicate. The API does not load every document id into memory to filter kinds.
- The same filter works on graph workspaces. It does not require `rag_profile = normal`.

WHY: [WHY-E](00-why.md).

## LAW-180-11 — Graph surfaces are absent, not broken

On a normal workspace the UI does not show the knowledge-graph nav, entity counts, or "storing in knowledge graph" progress. MCP entity tools return **409** `profile_has_no_graph` with a one-line reason. They do not return an empty graph that the client interprets as "no entities yet."

WHY: [WHY-D](00-why.md).

## LAW-180-12 — Existing workspaces stay graph

Migration 225 sets `rag_profile` default `'graph'` and backfills existing rows to `graph` in the same `ADD COLUMN … DEFAULT`. No document is reprocessed. Query and ingest for those workspaces keep Mix and extraction.

`EDGEQUAKE_INGEST_PROFILE=chunk_only` no longer disables the graph on a workspace whose column is `graph`. Operators who used the fleet switch create a normal workspace. The env is documented as deprecated in `.env.example`.

## LAW-180-13 — No per-document profile

Upload form field `extraction_mode`, when the workspace is normal, returns **400** `extraction_mode_not_applicable`. Reprocess does not resurrect a graph from an old snapshot. The fingerprint stamps `rag_profile` so a stale snapshot from a different shape cannot replay ([F-180-08](04-code-findings.md) warns that the inherit word `none` must not be reused as a mode name).

## LAW-180-14 — Zero entities is success

When `ProfileCapabilities.extracts_graph` is false, `entity_count == 0 && chunk_count > 0` stores status `completed`. The anomaly log in [persist.rs](../../edgequake/crates/edgequake-api/src/processor/text_insert/persist.rs) lines 500–509 does not fire. `chunk_count == 0` is still a failure.

## LAW-180-15 — Every edge case has a named test

[14-edge-cases](14-edge-cases.md) and [16-e2e-test-matrix](16-e2e-test-matrix.md) use the same EC id. A new EC without a test name is an incomplete spec. Hermetic tests use `edgequake-fake-llm`. Live `gemini-embedding-2` tests are `#[ignore]` unless `GOOGLE_CLOUD_PROJECT` and a Vertex token are set. The project under test is `mobioos-506015`, set in the test harness, not inherited from an unrelated shell export.

## LAW-180-16 — Keywords are index terms

"Keywords" in the product sentence means the BM25 postings for the chunk (and for an image, the optional caption). There is no ingest-time LLM keyword extractor on a normal workspace. Query-time keyword LLM extraction is off for that profile ([LAW-180-8](#law-180-8--graph-modes-are-refused-the-default-is-normal-retrieval)).

## DRY and SOLID map

| Principle | Where it binds |
|-----------|----------------|
| SRP | `NormalRetriever` retrieves. `ProfileCapabilities` answers policy. `LexicalIndex` ranks terms. `MultimodalEmbedder` embeds parts. |
| OCP | A new profile extends the capability constructor. Gates do not grow a new branch. |
| LSP | `LexicalIndex` backends (built-in BM25, optional `pg_search`) are substitutable. Callers see ranked ids. |
| ISP | Query code depends on retrieval, not on graph storage traits, when `queries_graph` is false. |
| DIP | The worker takes capabilities and an embedder trait. It does not construct Vertex HTTP itself. |
| DRY | Resolver shape copies SPEC-160 (`parse`, explicit source, no silent fallback) but the profile is not a third `ExtractionMode`. See [05-profile-law](05-profile-law.md). |

## What is not a law

- The numeric default dimension 1536. It is a recommendation under [LAW-180-5](#law-180-5--one-vector-space-per-workspace). 768 and 3072 are legal.
- The BM25 weight 1.25. It is the current constant, now configurable. Changing it is an eval decision, not a law break, as long as fusion stays rank-based.
