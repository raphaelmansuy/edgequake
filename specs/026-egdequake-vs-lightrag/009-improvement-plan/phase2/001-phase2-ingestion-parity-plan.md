# 001 — Phase 2 Ingestion Parity Plan (4 weeks)

**Cross-ref:** [README](./README.md) · [002 E2E Matrix](./002-e2e-test-matrix.md) · [001 Improvement Plan](../001-improvement-plan.md)

**Date:** 2026-06-27  
**Status:** ✅ Implemented (2026-06-27)  
**Goal:** Close LightRAG ingestion gaps (C-03, N-12) while preserving EdgeQuake durability extensions (C-05).  
**Principle:** Extend existing SSOT modules — no parallel ingestion paths, no LightRAG monolith ports.

---

## 1. Current State (baseline)

### EdgeQuake strengths to preserve

```text
  SSOT (post SPEC-025)                    Do NOT fork
  ────────────────────                    ────────────
  document_admission.rs                   upload handlers
  build_ingestion_pipeline()              per-route pipeline builders
  DefaultIngestionPersister               ad-hoc persist in handlers
  processor/text_insert/*                 monolithic text_insert.rs
  adaptive_chunking.rs                    duplicate size logic
  SC2 saga (vectors → graph → compensate) remove compensation
```

### Gaps vs LightRAG (June 2026 reference)


| Gap                          | LightRAG source                                 | EdgeQuake today                              |
| ---------------------------- | ----------------------------------------------- | -------------------------------------------- |
| Recursive character chunking | `chunker/recursive_character.py`, strategy `R`  | ✅ `RecursiveCharacterChunking` + registry (custom Rust split; LR separator cascade) |
| Markdown IR + breadcrumbs    | `parser/markdown/`, `ParagraphSemanticChunking` | ✅ `markdown_ir/` + `MarkdownChunking` strategy                                      |
| Section context in extract   | `operate.py::_truncate_section_context`         | ✅ `prompts/section_context.rs` injected in extractor + gleaning                     |
| LLM role separation          | `llm_roles.py`                                  | ✅ `edgequake-core/llm_roles.rs`; extract in factory, query in resolver              |
| Admission orphan KV          | N/A (in-memory text)                            | ✅ Staging KV + promote/rollback saga (P-11)                                           |


### Existing foundation to extend (not replace)


| Module                                         | Reuse                                                          |
| ---------------------------------------------- | -------------------------------------------------------------- |
| `edgequake-pipeline/src/chunker/`              | `ChunkingStrategy` trait + `Chunker::with_strategy` (SPEC-017) |
| `edgequake-pipeline/src/adaptive_chunking.rs`  | Size/overlap SSOT for `fixed` strategy                         |
| `edgequake-pipeline/src/ingestion_pipeline.rs` | Add `chunk_strategy` to `IngestionPipelineOptions`             |
| `edgequake-api/.../document_admission.rs`      | Staging key prefix refactor                                    |
| `workspace_pipeline_factory.rs`                | Role-aware provider resolution                                 |


---

## 2. June 2026 component choices

Research date: 2026-06-27. Prefer battle-tested crates with token-aware splitting.


| Need                   | Recommendation                                                                              | Rationale                                                                                                       | Rejected                                                        |
| ---------------------- | ------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- |
| Recursive split engine | **`text-splitter` v0.32+** (planned) | **Implemented:** pure-Rust `RecursiveCharacterChunking` with LightRAG separator cascade (no new dep yet) | `chunkedrs` (immature) |
| Token counting         | Existing `text_utils::estimate_tokens` + optional tiktoken for parity tests                 | Embedding safety already tuned for 2048-token models                                                            | Full tiktoken in hot path (cost/latency)                        |
| Markdown AST           | **Lightweight custom IR** (heading stack + block list)                                      | LightRAG markdown parser is Python-only; port algorithm not codebase                                            | Pulldown-cmark full HTML pipeline (overkill for chunk metadata) |
| Separator cascade      | Port LightRAG default from `env.example` / `FileProcessingPipeline.md`                      | Byte-level parity with `R` strategy                                                                             | English `.?!` in cascade (LightRAG deliberately excludes)       |
| LLM roles              | Extend `WorkspaceService` config                                                            | Multi-tenant SSOT; mirrors `llm_roles.py` priority                                                              | Global env-only overrides                                       |


**LightRAG reference separators (strategy R):**

```text
["\n\n", "\n", "。", "！", "？", "；", "，", " ", ""]
```

**Default chunk sizes (both systems, Jun 2026):** 1200 tokens target, ~100 token overlap — EdgeQuake adaptive scales **fixed** base size by doc bytes; recursive uses same size table.

**Explicitly deferred:** `V` (semantic_vector), `P` (paragraph_semantic full port) — embed-per-sentence cost; revisit Phase 3+ after eval data.

---

## 3. SOLID / DRY architecture

### 3.1 Open/Closed — Chunking Strategy Registry

```text
  edgequake-pipeline/src/chunker/
  ├── types.rs              (existing ChunkingStrategy trait)
  ├── strategies/
  │   ├── fixed.rs          (rename TokenBasedChunking + adaptive wiring)
  │   ├── recursive.rs      (NEW — text-splitter + LR separator cascade)
  │   └── markdown.rs       (NEW — MarkdownSplitter + heading metadata)
  ├── registry.rs           (NEW — resolve strategy from ChunkStrategy enum)
  └── mod.rs                (export registry::resolve_chunker)

  ChunkStrategy enum (API + IngestionPipelineOptions):
    Fixed | Recursive (default) | Markdown

  build_ingestion_pipeline(..., options.chunk_strategy)
       └── registry::resolve_chunker(strategy, options) → Chunker
```

**SRP:** Each strategy file owns one algorithm. Registry owns selection only.

**DIP:** Pipeline depends on `ChunkingStrategy` trait, not concrete splitters.

### 3.2 Single persist path (unchanged)

All strategies flow through:

```text
  Chunker → Extractor → Embedding → DefaultIngestionPersister
```

No strategy-specific persist branches.

### 3.3 Markdown IR — minimal port

```text
  markdown_ir/
  ├── parse.rs       walk lines → Vec<MarkdownBlock { kind, level, text, offset }>
  ├── breadcrumb.rs  heading stack → "A > B > C" path per block
  └── chunk.rs       split at heading boundaries; attach SectionMetadata to TextChunk
```

**DRY:** PDF markdown from `edgequake-pdf2md` and API `.md` uploads share `markdown_ir::chunk`.

### 3.4 Section context SSOT

```text
  edgequake-pipeline/src/prompts/section_context.rs   (NEW)
    format_section_context(heading_path: &[String]) → "---Section Context---\n..."
    truncate_section_context(s: &str, max_tokens: usize)  // port LR logic

  LLMExtractor / GleaningExtractor:
    if chunk.section_context.is_some() → prepend to extraction prompt
```

One formatter; extraction and gleaning both call it.

### 3.5 LLM roles

```text
  edgequake-core/src/workspace/llm_roles.rs   (NEW)
    LlmRole: Extract | Query | Summary
    resolve_llm_for_role(workspace, role) → Arc<dyn LLMProvider>

  workspace_pipeline_factory:
    extract role → build_ingestion_pipeline
    query engine bootstrap → query role
    merger summarization → summary role (existing merge path)
```

Fallback chain (LightRAG parity): role-specific model → workspace default → global default.

### 3.6 Admission saga (P-11)

```text
  CURRENT                         TARGET
  ───────                         ──────

  HTTP → KV (final keys)          HTTP → KV (staging:* keys only)
       → enqueue worker                → enqueue worker
       → worker persist                → worker persist
  fail = orphan KV + Failed doc       → success: promote staging → final
                                      → fail: delete staging:* + Failed doc
```

**Key naming:**

```text
  staging:{doc_id}-metadata
  staging:{doc_id}-content
  staging:hash:{workspace}:{sha256}   → doc_id mapping (promote or delete)
```

Promote = copy/rename to final keys in one logical operation; delete staging on any worker failure before graph write.

**SRP:** `document_admission.rs` — admit + staging writes. `text_insert/finalize.rs` — promote or rollback staging.

---

## 4. API surface (minimal)

Extend existing upload JSON (backward compatible defaults):

```json
{
  "content": "...",
  "title": "spec.md",
  "chunk_strategy": "recursive",
  "chunk_options": {
    "chunk_token_size": 1200,
    "chunk_overlap_token_size": 100,
    "separators": ["\\n\\n", "\\n", "。", " ", ""]
  }
}
```


| Field            | Default   | Notes                                                                         |
| ---------------- | --------- | ----------------------------------------------------------------------------- |
| `chunk_strategy` | `"fixed"` | Maps to adaptive fixed token (current behavior)                               |
| `"recursive"`    | —         | LightRAG `R` parity                                                           |
| `"markdown"`     | —         | Auto when `mime_type` is `text/markdown` or `.md` extension unless overridden |


Persist chosen strategy in doc metadata (`chunking_strategy`) for observability.

---

## 5. Week-by-week execution

### Week 1 — P-02 Chunking registry


| Day | Task                                                                        | Owner module                               |
| --- | --------------------------------------------------------------------------- | ------------------------------------------ |
| 1–2 | Add `text-splitter` dep; implement `RecursiveCharacterChunking`             | `chunker/strategies/recursive.rs`          |
| 2   | Extract `FixedTokenChunking` from `TokenBasedChunking`; wire adaptive sizes | `chunker/strategies/fixed.rs`              |
| 3   | `registry.rs` + `ChunkStrategy` enum; extend `IngestionPipelineOptions`     | `ingestion_pipeline.rs`                    |
| 4   | API: accept `chunk_strategy` in `DocumentAdmissionInput` → task metadata    | `document_admission.rs`, `documents_types` |
| 5   | Contract tests vs LightRAG fixture texts                                    | `contract_spec026_recursive_chunking.rs`   |


**Week 1 gate:** Same input text → recursive produces ≥90% chunk boundary overlap with LightRAG `R` on 5 fixture docs.

### Week 2 — P-03 Markdown IR + P-04 Section context


| Day | Task                                                                                     | Owner module                          |
| --- | ---------------------------------------------------------------------------------------- | ------------------------------------- |
| 1–2 | `markdown_ir` parser + breadcrumb stack                                                  | `edgequake-pipeline/src/markdown_ir/` |
| 2–3 | `MarkdownChunking` strategy; extend `TextChunk` with `section_path: Option<Vec<String>>` | `chunker/types.rs`                    |
| 3   | `section_context.rs` + inject in `LLMExtractor`                                          | `prompts/`, `extractor/`              |
| 4   | Wire auto-select `markdown` strategy for MD uploads                                      | `document_admission.rs`               |
| 5   | Golden tests: heading path preserved on chunks                                           | `contract_spec026_markdown_ir.rs`     |


**Week 2 gate:** Chunks from `spec026_fixtures/structured_manual.md` carry correct 3-level breadcrumbs; extraction prompt contains `---Section Context---` when path present.

### Week 3 — P-08 LLM roles + P-11 saga design


| Day | Task                                                             | Owner module                        |
| --- | ---------------------------------------------------------------- | ----------------------------------- |
| 1–2 | `LlmRole` resolution in workspace config + migration             | `edgequake-core`, storage migration |
| 2–3 | Wire roles into `workspace_pipeline_factory` and query bootstrap | `edgequake-api`, `edgequake-query`  |
| 3–4 | Staging KV schema + admit path refactor                          | `document_admission.rs`             |
| 4–5 | Promote/rollback in `text_insert/finalize.rs` + failure paths    | `processor/text_insert/`            |


**Week 3 gate:** Workspace with distinct extract/query models uses correct provider in ingest vs query contract tests.

### Week 4 — Integration, E2E, hardening


| Day | Task                                                            | Owner module                      |
| --- | --------------------------------------------------------------- | --------------------------------- |
| 1   | End-to-end: upload → worker → graph with each chunk strategy    | `e2e_spec026_ingestion_parity.rs` |
| 2   | Admission saga E2E: forced worker failure → no orphan final KV  | `e2e_spec026_admission_saga.rs`   |
| 3   | Head-to-head ingest: shared corpus chunk count ±10% vs LightRAG | Phase 1 compare script            |
| 4   | Metrics: `chunk_strategy`, `section_context_used` counters      | `edgequake-observability`         |
| 5   | Docs + Phase 2 exit review                                      | this spec                         |


**Week 4 gate:** All exit criteria in [002-e2e-test-matrix.md](./002-e2e-test-matrix.md).

---

## 6. Data model changes

### TextChunk extension

```rust
// chunker/types.rs
pub struct SectionMetadata {
    pub heading_path: Vec<String>,   // ["Install", "Prerequisites"]
    pub heading_level: u8,           // 2 for ##
}

pub struct TextChunk {
    // ... existing fields ...
    pub section: Option<SectionMetadata>,
}
```

Persist `section` in chunk KV JSON (backward compatible — optional field).

### Workspace config (LLM roles)

```json
{
  "llm_roles": {
    "extract": { "provider": "ollama", "model": "gemma3:latest" },
    "query":   { "provider": "openai", "model": "gpt-5-nano" },
    "summary": { "provider": "ollama", "model": "gemma3:latest" }
  }
}
```

Null role entries fall back to workspace default (LightRAG `llm_roles.py` semantics).

---

## 7. Risk register


| Risk                                             | Mitigation                                                                                             |
| ------------------------------------------------ | ------------------------------------------------------------------------------------------------------ |
| `text-splitter` token count ≠ embedder tokenizer | Keep char-estimate cap + embed batch validation; contract tests use tolerance bands                    |
| Staging promote race on duplicate hash           | Hold hash→doc mapping in staging until promote; transactional promote order: content → metadata → hash |
| Markdown IR diverges from LightRAG parser        | Golden fixtures exported from LightRAG `tests/parser/markdown/` (5 files minimum)                      |
| LLM role migration breaks existing workspaces    | Default all roles to current workspace provider; opt-in per role                                       |
| Chunk strategy API abuse (huge separators JSON)  | Validate separators max 16 entries, max 8 chars each; reject in admission                              |


---

## 8. What we explicitly do NOT do


| Item                                       | Reason                                       |
| ------------------------------------------ | -------------------------------------------- |
| Semantic vector chunking (`V`)             | Embed-per-sentence cost; Phase 3+            |
| Full LightRAG parser registry (13 formats) | Phase 3 DOCX; scope control                  |
| Second persist path for markdown           | Violates DRY / one-way ingest                |
| Remove adaptive chunking                   | EdgeQuake extension; applies to `fixed` only |
| Remove SC2 saga compensation               | EdgeQuake advantage (C-05)                   |
| Sync ingestion on HTTP                     | Already async SSOT (SPEC-024)                |


---

## 9. Success criteria (Phase 2 exit)


| Criterion                | Target                                                                | Verification          |
| ------------------------ | --------------------------------------------------------------------- | --------------------- |
| Chunk strategies shipped | `fixed` + `recursive` + `markdown`                                    | API + metadata        |
| LightRAG R parity        | ≥90% boundary overlap on 5 fixtures                                   | contract test         |
| Section context          | In extract prompt when heading path set                               | unit + e2e            |
| LLM roles                | 3 roles with fallback chain                                           | contract test         |
| Admission saga           | Zero orphan **final** KV on forced failure                            | e2e                   |
| SOLID                    | No new upload persist paths; registry only selection                  | code review checklist |
| DRY                      | Single `build_ingestion_pipeline`, single `section_context` formatter | grep audit            |
| Regression               | All SPEC-024/025 ingest e2e green                                     | CI                    |
| Observability            | `chunk_strategy` label on ingest metrics                              | `/metrics` scrape     |


---

## 10. Post-Phase 2 parity score (expected)


| Dimension            | Before                 | After              |
| -------------------- | ---------------------- | ------------------ |
| Chunking vs LightRAG | **D** (1/4 strategies) | **B+** (3/4; no V) |
| Markdown ingestion   | **C**                  | **B**              |
| Prompt engineering   | **C+**                 | **B+**             |
| Admission durability | **B** (N-12)           | **A**              |
| LLM ops flexibility  | **C+**                 | **B+**             |


**Net ingestion grade:** C+ → **B** (per [003-ingestion](../../003-ingestion/001-ingestion-comparison.md) rubric).

---

## 11. Implementation checklist

```text
Week 1
  [x] RecursiveCharacterChunking (custom Rust; text-splitter deferred)
  [x] ChunkStrategy registry + IngestionPipelineOptions
  [x] API chunk_strategy field → task metadata
  [x] contract_spec026_recursive_chunking.rs (11 tests + LightRAG boundary fixture)

Week 2
  [x] markdown_ir module + MarkdownChunking strategy
  [x] TextChunk.section metadata + KV persist
  [x] section_context.rs + extractor injection
  [x] contract_spec026_markdown_ir.rs (8 tests)

Week 3
  [x] LlmRole resolution (workspace metadata `llm_roles`; no DB migration)
  [x] workspace_pipeline_factory (extract) + query resolver (query role)
  [x] Staging KV admission refactor
  [x] text_insert promote/rollback

Week 4
  [x] e2e_spec026_ingestion_parity.rs (7 tests)
  [x] e2e_spec026_admission_saga.rs (3 tests)
  [x] contract_spec026_admission_staging.rs (5 tests)
  [x] e2e_spec026_llm_roles.rs (2 tests)
  [x] Metrics: edgequake_ingestion_chunk_strategy_total, section_context labels
  [x] Phase 1 compare script chunk diff (`scripts/spec026_export_lightrag_chunks.py`)
  [x] Phase 2 exit review (this update)
```

---

## 12. Implementation map (code)

| Deliverable | Primary modules | Tests |
|-------------|-----------------|-------|
| P-02 Chunk registry | `edgequake-pipeline/src/chunker/registry.rs`, `recursive.rs`, `markdown_chunking.rs` | `contract_spec026_recursive_chunking.rs` |
| P-03 Markdown IR | `edgequake-pipeline/src/markdown_ir/` | `contract_spec026_markdown_ir.rs` |
| P-04 Section context | `edgequake-pipeline/src/prompts/section_context.rs`, `extractor/llm.rs` | same + e2e markdown upload |
| P-08 LLM roles | `edgequake-core/src/llm_roles.rs`, `workspace_pipeline_factory.rs`, `providers/resolver.rs` | `contract_spec026_llm_roles.rs`, `e2e_spec026_llm_roles.rs` |
| P-11 Admission saga | `document_admission.rs`, `services/staging_admission.rs`, `text_insert/finalize.rs` | `contract_spec026_admission_staging.rs`, `e2e_spec026_admission_saga.rs` |
| Observability | `edgequake-observability/src/metrics.rs`, `text_insert/finalize.rs` | metrics scrape test |

**Deferred (explicit):** `text-splitter` crate adoption (optional hardening); summary-role wiring in merger (metadata resolution ready, merge path unchanged); LightRAG `V` semantic vector chunking.

---

## 13. Re-assessment (2026-06-27 — recursive default + hardening pass 2)

### Gaps closed this pass

| Gap | Fix | Verification |
|-----|-----|--------------|
| **Default strategy was `fixed` not LightRAG `R`** | `ChunkStrategy::default()` → `Recursive`; `resolve_for_upload` + `IngestionPipelineOptions::from_document_size` SSOT | `default_strategy_is_recursive`, `text_upload_recursive_strategy_default`, `file_upload_default_recursive_strategy` |
| Recursive merge swallowed paragraphs | Ported LightRAG `_split_text_with_spans` + `_merge_splits_with_spans`; leading `\n\n` +2 tokens in merge budget | `recursive_boundary_overlap_vs_lightrag_fixture` (≥90% offset overlap) |
| Source span offsets lost in `Chunker` | `ChunkResult.start_offset` / `end_offset`; recursive preserves LightRAG spans | Golden fixture `[0, 44, 101]` at `chunk_size=15` |
| `chunking_strategy` metadata wrong post-process | `init_chunk_stats` emits `ChunkStrategy` enum | E2E metadata assertions |
| File/batch upload chunk fields duplicated | `MultipartUploadFields` SSOT + `effective_chunk_fields()` | `file_upload.rs`, `batch_upload.rs` |
| Batch upload ignored `chunk_strategy` | Batch multipart forwards chunk fields per file | `cargo build -p edgequake-api` |
| Weak recursive E2E | Default + explicit strategy paths; `chunk_count` with `chunk_options` | 7 tests in `e2e_spec026_ingestion_parity` |
| Missing CJK fixture | `plain_zh_mixed.txt` + CJK token heuristic in `recursive_token_len` | `recursive_cjk_mixed_fixture_splits` |

### Recursive parity score (updated)

| Criterion | Before pass | After pass |
|-----------|-------------|------------|
| **Default upload strategy** | ❌ `fixed` | ✅ `recursive` (LightRAG R) |
| Separator cascade | ✅ | ✅ |
| keep_separator merge semantics | ❌ | ✅ |
| Token-budget merge | ❌ (char + min gate bug) | ✅ |
| Boundary offset golden compare | ❌ | ✅ (`lightrag_r_chunks.json`) |
| API → worker → metadata | ⚠️ | ✅ |
| Multipart chunk_strategy (file + batch) | ⚠️ file only | ✅ |
| E2E proof of multi-chunk recursive | ⚠️ | ✅ |

### Remaining (unchanged)

- Nightly CI hook for `scripts/spec026_export_lightrag_chunks.py` (script added; wire in CI optional)
- `text-splitter` crate (optional; custom Rust splitter passes contract + e2e)
- `LlmRole::Summary` in merger path