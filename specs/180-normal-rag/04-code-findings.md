# 04 — Code findings

Parent: [README](README.md) · Prev: [03-product-spec](03-product-spec.md) · Next: [05-profile-law](05-profile-law.md)

Line numbers were read on 2026-10-30 from the working tree. Paths are relative to the repository root. `edgequake-llm` is the crates.io crate **0.10.9**, not `crates/edgequake-llm/` (that directory holds a CHANGELOG only).

## Ingest

### F-180-01 — Chunk-only exists, and it is fleet-wide

[edgequake/crates/edgequake-pipeline/src/pipeline/config.rs](../../edgequake/crates/edgequake-pipeline/src/pipeline/config.rs) lines 483–519.

`IngestProfile::{Full, ChunkOnly}` reads `EDGEQUAKE_INGEST_PROFILE`. `ChunkOnly` sets entity and relationship extraction and their embeddings to false, and leaves chunk embeddings on. There is no workspace field. `from_env_str` maps any other word, including typos, to `Full`.

**Implication:** a per-workspace profile must not reuse this silent default. Unknown words at the API are 400 ([LAW-180-1](01-first-principles.md)). After migration, the column wins over the env ([LAW-180-12](01-first-principles.md)).

### F-180-02 — Zero entities is a partial failure

[edgequake/crates/edgequake-api/src/processor/text_insert/persist.rs](../../edgequake/crates/edgequake-api/src/processor/text_insert/persist.rs) lines 500–509.

```text
entity_count == 0 && chunk_count > 0  →  status "partial_failure"
```

A normal document would be marked failed on every success. [LAW-180-14](01-first-principles.md).

### F-180-03 — The pipeline also warns on zero entities

[edgequake/crates/edgequake-pipeline/src/pipeline/processing.rs](../../edgequake/crates/edgequake-pipeline/src/pipeline/processing.rs) line 368 sets `stats.error_details` when `entity_count == 0` and `chunk_count > 0`. The extract gate at line 283 is the right inner check (`enable_entity_extraction || enable_relationship_extraction`). The warning is not gated.

### F-180-04 — The factory always prepares extraction

[edgequake/crates/edgequake-api/src/workspace_pipeline_factory.rs](../../edgequake/crates/edgequake-api/src/workspace_pipeline_factory.rs) line 147 branches only for `ExtractionMode::Decision`. The LLM arm still builds an extraction provider. A normal profile needs a third arm that builds an embedder and no chat LLM. It must not be encoded as `ExtractionMode`, because inherit words include `none` (F-180-08) and extraction mode is a graph strategy.

### F-180-05 — Extract usage is judged before the factory

[edgequake/crates/edgequake-api/src/processor/workspace_resolver.rs](../../edgequake/crates/edgequake-api/src/processor/workspace_resolver.rs) lines 139 and 178 call `judge_ingest`, which checks `Usage::Extract`. A normal workspace with no extract model would fail ingest before chunking.

### F-180-06 — Summary LLM is resolved on every persist

[persist.rs](../../edgequake/crates/edgequake-api/src/processor/text_insert/persist.rs) lines 196–217 judge `Usage::Summary` and resolve a summary LLM. Lines 456–470 (worker) overwrite `llm_provider` / `llm_model` in stats with the extract role, so a chunk-only document would display a model it never called.

### F-180-07 — Merge and community refresh run after an empty extraction

[edgequake/crates/edgequake-pipeline/src/persistence/ingestion_persister.rs](../../edgequake/crates/edgequake-pipeline/src/persistence/ingestion_persister.rs) lines 792–800 merge extractions. Lines 802–856, when `stats.errors == 0`, spawn community refresh (`schedule_community_index_refresh_with_extras` at line 856). An empty merge still takes that branch.

Embedding flags themselves are already separate: [embeddings.rs](../../edgequake/crates/edgequake-pipeline/src/pipeline/helpers/embeddings.rs) lines 657–659.

### F-180-08 — Do not name a mode `none`

[edgequake/crates/edgequake-pipeline/src/extraction_mode.rs](../../edgequake/crates/edgequake-pipeline/src/extraction_mode.rs) line 34: `INHERIT_WORDS = ["", "inherit", "none"]`. The same list is in [workspaces_types/extraction_mode.rs](../../edgequake/crates/edgequake-api/src/handlers/workspaces_types/extraction_mode.rs) line 25. A profile or mode called `none` is read as inherit. Wire values are `graph` and `normal`.

## Query

### F-180-09 — Mix is the default everywhere that matters

[edgequake/crates/edgequake-query/src/modes.rs](../../edgequake/crates/edgequake-query/src/modes.rs) lines 89–91: `QueryMode::Mix` is `#[default]`. The API, chat handler, MCP schema, and UI settings store repeat Mix. Adaptive intent routing does not run when the request already set a mode, which the API always does.

### F-180-10 — Graph steps wrap naive retrieval

[edgequake/crates/edgequake-query/src/engine_impl/query_entry/query_pipeline.rs](../../edgequake/crates/edgequake-query/src/engine_impl/query_entry/query_pipeline.rs) line 404 calls `expand_entities_named_in_query`. Line 859 calls `graph_seed_admit::admit_into_context` after retrieval, including Naive. Keyword extraction and `validate_keywords` run in `pipeline_prepare` for every non-bypass query.

### F-180-11 — Default fusion drops dense hits; RRF then hits `min_score`

[edgequake/crates/edgequake-query/src/sparse_retrieval.rs](../../edgequake/crates/edgequake-query/src/sparse_retrieval.rs) lines 63–70: anything other than `rrf` is sparse-first. Lines 176–188: RRF uses weights `[1.0, 1.25]` and `fusion::RRF_K`, then line 188 retains `score >= min_score`. Max RRF score is `(1 + 1.25) / 61 ≈ 0.037`.

[fusion.rs](../../edgequake/crates/edgequake-query/src/fusion.rs) line 60: `RRF_K = 60`. The formula at lines 65–83 is the one normal retrieval must call.

### F-180-12 — `min_score` defaults to 0.1

[edgequake/crates/edgequake-query/src/engine_impl/mod.rs](../../edgequake/crates/edgequake-query/src/engine_impl/mod.rs) lines 194–200. Env `EDGEQUAKE_MIN_ENTITY_SCORE`, default `0.1`. This threshold is meaningful for cosine and for graph scores. It is not meaningful for RRF.

### F-180-13 — Lexical search is English `ts_rank_cd`

[fts.rs](../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector/fts.rs) lines 69–78: `ts_rank_cd` and `websearch_to_tsquery('english', $1)`. Migration [136_spec091_chunk_content_tsv.sql](../../edgequake/migrations/136_spec091_chunk_content_tsv.sql) lines 4–8 generates `content_tsv` with `to_tsvector('english', …)`. Tests in `fts.rs` assert this is not BM25. `websearch_to_tsquery` AND-combines terms, so a long question often matches nothing (R-180-4).

### F-180-14 — No document-kind filter

[query_types.rs](../../edgequake/crates/edgequake-api/src/handlers/query_types.rs) lines 40–69: `DocumentFilter` is `date_from`, `date_to`, `document_pattern`, `document_ids`.

### F-180-15 — `modalities` is a chunk label, not a file type

[vector.rs](../../edgequake/crates/edgequake-storage/src/traits/vector.rs) line 66: `MetadataFilter.modalities`. Values used in retrieval are chart / figure / table / equation. Storage can filter them. The API never passes a user-chosen kind.

## API, MCP, UI

### F-180-16 — MCP search forces a subgraph

[search.rs](../../edgequake/crates/edgequake-api/src/mcp/project/search.rs) line 115 sets `include_subgraph: true`. [profile.rs](../../edgequake/crates/edgequake-api/src/mcp/project/profile.rs) line 34 starts instructions with "EdgeQuake is a Graph-RAG store." Entity tools have no profile check.

### F-180-22 — The wizard tunes extraction, not the profile

[wizard-state.ts](../../edgequake_webui/src/lib/onboarding/wizard-state.ts) lines 194–195: create steps start at `workspace-basics`, then `models`, then optional `extraction`. There is no profile step. `extractionMode` is `'inherit' | 'llm' | 'decision'` (line 84).

## Embeddings and schema

### F-180-17 — The embedding trait is text, and Vertex `:predict` is wrong for this model

`edgequake-llm` 0.10.9 `EmbeddingProvider::embed(&self, texts: &[String])`. Gemini/Vertex `embed_vertex_ai` posts `:predict` with a text instance and hard-codes `task_type = RETRIEVAL_DOCUMENT`. The factory drops the dimension argument. Missing Vertex credentials log a warning and return `MockProvider`. Access tokens from `gcloud` are static. Google's current contract for `gemini-embedding-2` is `:embedContent` (R-180-5).

### F-180-18 — No multimodal embedding card

[models.toml](../../edgequake/models.toml) lines 1239–1254: `embeddinggemma`, `supports_vision = false`, dimension 768. `gemini-embedding-001` exists for Gemini and Vertex. `gemini-embedding-2` does not. `ModelType::Multimodal` means a vision chat model, not an embedding model that accepts images. `ModelFacts` has no `image_embedding` flag.

### F-180-19 — HNSW widths are 768, 1024, and 1536

[132_spec091_unconstrained_halfvec.sql](../../edgequake/migrations/132_spec091_unconstrained_halfvec.sql) lines 64–80 create partial HNSW indexes for those widths on `chunk_embeddings`. 3072, the vendor default for `gemini-embedding-2`, has no ANN index. `halfvec` can store it (limit 4000). A query at 3072 would be correct and sequential.

### F-180-20 — Workspace config is JSONB, except two provider columns

[workspace.rs](../../edgequake/crates/edgequake-core/src/types/multitenancy/workspace.rs) lines 115–140: `metadata` plus typed embedding fields rebuilt from JSON. There is no `rag_profile`. Latest migration is 224 (`compat_serve_max = 224` in [manifest.toml](../../edgequake/migrations/manifest.toml)). Immutability of embedding model is advisory (`embedding_rebuild_required`), not a trigger.

### F-180-21 — Images become text, then vectors

PDF and standalone images go through a vision LLM. Descriptions become `MultimodalChunk` text and are embedded as strings. `document_mm_assets` stores pixels. Nothing writes an image vector. Retrieval modality stamping is a label for charts and tables, not an embedding.

## What is already reusable

| Piece | Reuse |
|-------|--------|
| SPEC-160 resolver shape | Pure parse, explicit source, 400 on unknown. Do not add a third extraction mode. |
| `IngestProfile::apply_to` | The flag effects. Call it from capabilities, not from a fleet env read in the hot path. |
| `fusion::reciprocal_rank_fusion` | The only RRF implementation. |
| `rebuild-embeddings` | Model change inside one space. |
| `MetadataFilter.modalities` and chunk modality column | Keep for chart/table. Add `source_modality` beside it. |
| SPEC-174 clearance intersection | Kind filter must join that set. |
| Typed `chunk_embeddings` | Image vectors are rows with the same model id. |
