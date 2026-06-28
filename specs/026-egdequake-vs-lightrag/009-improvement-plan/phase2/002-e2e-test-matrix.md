# 002 — Phase 2 E2E & Contract Test Matrix

**Cross-ref:** [001 Plan](./001-phase2-ingestion-parity-plan.md) · [003 Ingestion Comparison](../../003-ingestion/001-ingestion-comparison.md)

**Date:** 2026-06-27  
**Status:** ✅ Implemented (2026-06-27)  
**Principle:** Tests are the parity specification — every P-02..P-11 deliverable maps to at least one automated test.

---

## 1. Test layers

```text
  Layer 4  E2E (edgequake-api/tests/)     Full HTTP → worker → Postgres → graph
  Layer 3  Integration (edgequake-pipeline/tests/)   Pipeline + persist with mock LLM
  Layer 2  Contract (*/tests/contract_spec026_*)   Algorithm parity vs LightRAG fixtures
  Layer 1  Unit (module #[cfg(test)])     Pure functions: IR, registry, section_context
```

Run order in CI: Layer 1 → 2 → 3 → 4 (fail fast).

---

## 2. Fixture corpus

Location: `edgequake/crates/edgequake-pipeline/tests/fixtures/spec026/`

| File | Purpose | Source |
|------|---------|--------|
| `plain_en.txt` | Recursive split baseline | Synthetic |
| `plain_zh_mixed.txt` | CJK separator cascade | LightRAG `CHUNK_R_SEPARATORS` doc example |
| `structured_manual.md` | Heading breadcrumbs (3 levels) | Synthetic (mirrors LR parser tests shape) |
| `long_doc_80kb.txt` | Adaptive size → 800 tokens | SPEC-025 adaptive thresholds |
| `long_doc_150kb.txt` | Adaptive size → 600 tokens | SPEC-025 adaptive thresholds |
| `lightrag_r_chunks.json` | Expected chunk boundaries for `plain_en.txt` | Generated once from LightRAG `R` strategy |

**Generation script (Phase 1 dependency):** `scripts/spec026_export_lightrag_chunks.py` — run against local LightRAG with frozen tokenizer settings; commit JSON artifacts.

---

## 3. Contract tests (Layer 2)

Crate: `edgequake-pipeline`

### `contract_spec026_recursive_chunking.rs`

| Test | Asserts | P-ID |
|------|---------|:----:|
| `recursive_default_separators_match_lightrag` | Separator list equals LR default | P-02 |
| `recursive_produces_multiple_chunks_on_paragraphs` | `\n\n` splits before char fallback | P-02 |
| `recursive_cjk_splits_on_fullwidth_punctuation` | `。！？` boundaries honored | P-02 |
| `recursive_overlap_tokens_applied` | Adjacent chunks share overlap text | P-02 |
| `recursive_boundary_overlap_vs_lightrag_fixture` | ≥90% start-offset match vs `lightrag_r_chunks.json` | P-02 |
| `fixed_adaptive_sizes_unchanged` | 30k/80k/150k byte thresholds | P-02 |
| `registry_selects_strategy_by_enum` | `ChunkStrategy::Recursive` → strategy name | P-02 |

### `contract_spec026_markdown_ir.rs`

| Test | Asserts | P-ID |
|------|---------|:----:|
| `markdown_ir_builds_heading_stack` | `# A`, `## B`, body → path `["A","B"]` | P-03 |
| `markdown_ir_resets_stack_on_h1` | New H1 clears deeper levels | P-03 |
| `markdown_chunking_splits_at_headings` | No chunk spans unrelated sections | P-03 |
| `markdown_chunk_carries_section_metadata` | `TextChunk.section.heading_path` populated | P-03 |
| `section_context_format_matches_lightrag` | Contains `---Section Context---` block | P-04 |
| `section_context_truncates_long_paths` | Port `_truncate_section_context` token budget | P-04 |
| `extractor_prompt_includes_section_when_present` | Mock LLM receives context prefix | P-04 |

Crate: `edgequake-core`

### `contract_spec026_llm_roles.rs`

| Test | Asserts | P-ID |
|------|---------|:----:|
| `resolve_extract_role_uses_configured_provider` | Distinct model string in pipeline | P-08 |
| `resolve_query_role_falls_back_to_workspace_default` | Missing query role → default | P-08 |
| `resolve_summary_role_for_merge` | Merger gets summary role provider | P-08 |
| `role_priority_matches_lightrag_semantics` | role → workspace → global | P-08 |

Crate: `edgequake-api`

### `contract_spec026_admission_staging.rs`

| Test | Asserts | P-ID |
|------|---------|:----:|
| `admit_writes_staging_keys_not_final` | After 202, final `{id}-metadata` absent | P-11 |
| `promote_copies_staging_to_final` | After success, final keys exist | P-11 |
| `rollback_deletes_staging_on_failure` | Simulated worker fail → no staging keys | P-11 |
| `failed_doc_has_no_orphan_content_kv` | `doc_content` absent for failed doc | P-11 |

---

## 4. Integration tests (Layer 3)

Crate: `edgequake-pipeline`

| Test file | Scenario |
|-----------|----------|
| `integration_spec026_chunk_strategy_pipeline.rs` | Full `Pipeline::process` with each strategy; entity count &gt; 0 |
| `integration_spec026_section_context_extraction.rs` | MD doc → extracted entity description references section topic |

Crate: `edgequake-api` (lib or processor unit)

| Test | Scenario |
|------|----------|
| `processor/text_insert/promote_tests.rs` | Promote/rollback pure KV logic with memory adapter |

---

## 5. E2E tests (Layer 4)

Crate: `edgequake-api/tests/`  
Pattern: follow `e2e_spec024_text_upload_async.rs` — `create_test_app_with_workers`, poll until `completed`.

### `e2e_spec026_ingestion_parity.rs`

| Test | Flow | Asserts |
|------|------|---------|
| `text_upload_recursive_strategy_default` | POST `/documents` no strategy | `chunking_strategy` metadata = `recursive`; graph entities present |
| `text_upload_recursive_strategy` | POST with `"chunk_strategy":"recursive"` | metadata + chunk count within fixture band |
| `recursive_splits_more_paragraphs_than_fixed_e2e` | Explicit fixed vs recursive + small token budget | recursive ≥3 chunks; fixed metadata preserved |
| `markdown_upload_auto_strategy` | POST `.md` with headings | chunks have section paths in KV |
| `file_upload_default_recursive_strategy` | POST `/documents/upload` multipart, no strategy field | metadata = `recursive` |
| `file_upload_recursive_strategy` | POST `/documents/upload` multipart explicit recursive | Same as JSON path |
| `batch_upload_mixed_strategies` | Batch with per-file metadata override | Each doc correct strategy in metadata |

### `e2e_spec026_admission_saga.rs`

| Test | Flow | Asserts |
|------|------|---------|
| `worker_failure_leaves_no_final_kv` | Inject failing extractor mock via test hook | status=`failed`; grep KV: no `{doc_id}-content` without `indexed` |
| `worker_success_promotes_staging` | Happy path | final KV keys exist; staging keys absent |
| `duplicate_hash_during_staging_rejected` | Two uploads same hash while first in-flight | Second returns duplicate-processing |

### `e2e_spec026_llm_roles.rs`

| Test | Flow | Asserts |
|------|------|---------|
| `workspace_extract_role_used_in_ingest` | Workspace config extract=mock-A | Provider metric / mock call tag = A |
| `workspace_query_role_used_in_query` | Query after ingest with query=mock-B | Query path uses B |

**Note:** Mock provider hooks already exist in `e2e_provider_tracking_stats.rs` — extend, do not duplicate.

---

## 6. Regression guard (must stay green)

| Existing test | Why |
|---------------|-----|
| `e2e_spec024_text_upload_async.rs` | Async SSOT |
| `contract_spec025_adaptive_chunking.rs` | Adaptive sizes |
| `spec017_pipeline_contract.rs` | Chunker strategy trait |
| `resource_safety_proof.rs` | No memory regressions |
| Phase 1 `compare_lightrag_edgequake.py` ingest stage | Corpus-level parity |

---

## 7. CI wiring

```yaml
# Suggested job fragment (conceptual)
spec026-phase2:
  steps:
    - cargo test -p edgequake-pipeline --test 'contract_spec026_*'
    - cargo test -p edgequake-core --test contract_spec026_llm_roles
    - cargo test -p edgequake-api --test 'contract_spec026_*'
    - cargo test -p edgequake-api --test 'e2e_spec026_*'
```

Optional nightly: run LightRAG chunk export diff (non-blocking warning if &lt;90% overlap).

---

## 8. SOLID / DRY test audit checklist

Before Phase 2 exit, verify:

| Check | Method |
|-------|--------|
| No duplicate e2e upload boilerplate | Shared `common::upload_document_with_options()` helper |
| One fixture directory | Only `fixtures/spec026/` |
| One compare script for LR chunks | `scripts/spec026_export_lightrag_chunks.py` |
| Admission tests use SSOT admit fn | Call `document_admission` module, not reimplemented HTTP |
| Section context tests import one formatter | `prompts::section_context` only |

---

## 9. Exit criteria (Phase 2 complete)

All must pass on `main`:

- [x] 7+ contract tests in `contract_spec026_recursive_chunking.rs`
- [x] 7+ contract tests in `contract_spec026_markdown_ir.rs`
- [x] 4+ contract tests in `contract_spec026_llm_roles.rs`
- [x] 4+ contract tests in `contract_spec026_admission_staging.rs`
- [x] 5+ e2e tests in `e2e_spec026_ingestion_parity.rs`
- [x] 3+ e2e tests in `e2e_spec026_admission_saga.rs`
- [x] 2+ e2e tests in `e2e_spec026_llm_roles.rs`
- [x] LightRAG R boundary overlap ≥90% on `plain_en.txt`
- [x] Zero SPEC-024/025 ingest regressions (`e2e_spec024_text_upload_async` green)
- [x] Manual review: no new persist path outside `IngestionPersister` + staging promote

---

## 10. Traceability matrix

| Priority | Deliverable | Primary test(s) |
|:--------:|-------------|-----------------|
| P-02 | Recursive + registry | `contract_spec026_recursive_chunking`, `e2e_spec026_ingestion_parity` |
| P-03 | Markdown IR | `contract_spec026_markdown_ir`, `markdown_upload_auto_strategy` |
| P-04 | Section context | `section_context_*`, `extractor_prompt_includes_section` |
| P-08 | LLM roles | `contract_spec026_llm_roles`, `e2e_spec026_llm_roles` |
| P-11 | Admission saga | `contract_spec026_admission_staging`, `e2e_spec026_admission_saga` |
