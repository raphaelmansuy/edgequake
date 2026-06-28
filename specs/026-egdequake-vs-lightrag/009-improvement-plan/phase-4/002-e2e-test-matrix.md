# 002 — Phase 4 E2E & Contract Test Matrix

**Cross-ref:** [001 Plan](./001-phase4-scale-multimodal-plan.md) · [003 Brutal Assessment](./003-brutal-lightrag-assessment.md) · [004 Parity Implementation Plan](./004-multimodal-parity-implementation-plan.md)

**Date:** 2026-06-27  
**Status:** ✅ Implemented (2026-06-27)  
**Principle:** Tests are the parity specification — every P-07 / P-12 deliverable maps to at least one automated test.

---

## 1. Test layers

```text
  Layer 4  E2E (edgequake-api/tests/)           HTTP → VLM mock → worker → graph
  Layer 3  Integration (edgequake-tasks/tests/)   Delivery bridge round-trip
  Layer 2  Contract (*/tests/contract_spec026_*)  Pure logic: vision JSON, delivery
  Layer 1  Unit (module #[cfg(test)])           Prompts, options parsing, role resolve
```

Run order in CI: Layer 1 → 2 → 3 → 4 (fail fast).

---

## 2. Fixture corpus

| File | Purpose | Source |
|------|---------|--------|
| `tests/fixtures/spec026/tiny.png` | 1×1 PNG for upload E2E | Synthetic |
| `tests/fixtures/spec026/mock_vlm_sarah.md` | Mock VLM output with entity | Synthetic |
| `tests/fixtures/spec026/multimodal_image_analysis.json` | Expected structured VLM JSON | LightRAG schema subset |
| `tests/fixtures/spec026/mineru_drawing_tag.md` | `<drawing path="drawing.png"/>` | Phase 4e |
| `tests/fixtures/spec026/drawing.png` | 128×128 PNG asset (copy of tiny.png) | Phase 4e |

---

## 3. Contract tests (Layer 2)

Crate: `edgequake-api`

### `contract_spec026_multimodal.rs`

| Test | Asserts | P-ID |
|------|---------|:----:|
| `image_analysis_json_parses_lightrag_schema` | `{name,type,description}` required keys | P-07 |
| `image_analysis_invalid_type_falls_back_to_other` | Unknown type → `Other` | P-07 |
| `vision_markdown_includes_name_heading` | Emitted markdown has `# {name}` | P-07 |
| `multimodal_process_options_parse_ite` | `"ite"` → images+tables+equations | P-07 |
| `resolve_vlm_role_prefers_llm_roles_vlm` | Workspace override wins | P-07 |

Crate: `edgequake-core`

### `contract_spec026_llm_roles_vlm.rs`

| Test | Asserts | P-ID |
|------|---------|:----:|
| `resolve_vlm_falls_back_to_vision_fields` | `vision_llm_provider` used | P-07 |
| `resolve_vlm_falls_back_to_default_llm` | Last resort workspace llm | P-07 |

### `contract_spec026_multimodal_pdf.rs`

| Test | Asserts | P-ID |
|------|---------|:----:|
| `scan_inline_image_refs_finds_drawing_tag` | `<drawing/>` detected | P-07 |
| `multimodal_process_options_default_disables_images` | default opts.images=false | P-07 |
| `enrich_markdown_leaves_plain_text_unchanged_without_i_flag` | no `i` → no-op | P-07 |
| `probe_and_validate_reject_tiny_png` | 1×1 below VLM_MIN (default **64**) | P-07 |
| `enrich_skips_tiny_data_uri_without_vlm_call` | LightRAG tiny skip | P-07 |

### `contract_spec026_multimodal_stage.rs` (Phase 4d)

| Test | Asserts | P-ID |
|------|---------|:----:|
| `metadata_roundtrip_process_options` | KV field `multimodal_process_options` | 4d |
| `gates_default_disabled_like_lightrag` | `VLM_PROCESS_ENABLE` default **off** (LightRAG) | 4d/4h |
| `analyze_stage_skips_without_i_flag` | SSOT stage no-op without `i` | 4d |
| `analyze_stage_enriches_data_uri_with_i_flag` | SSOT stage replaces data-URI | 4d |

### `contract_spec026_multimodal_json_recovery.rs` (Phase 4e)

| Test | Asserts | P-ID |
|------|---------|:----:|
| `extracts_fenced_json_object` | Markdown fence strip | 4e |
| `parse_rejects_non_json` | Fail-closed parse | 4e |

### `contract_spec026_multimodal_assets.rs` (Phase 4e)

| Test | Asserts | P-ID |
|------|---------|:----:|
| `drawing_tag_loads_asset_from_fixture_dir` | E16 path → VLM | 4e |

### `contract_spec026_multimodal_strict_fail.rs` (Phase 4e)

| Test | Asserts | P-ID |
|------|---------|:----:|
| `vlm_disabled_with_i_strict_returns_hard_error` | E21 | 4e |
| `invalid_json_strict_fails_after_retry` | E20 | 4e |

### `contract_spec026_multimodal_tables.rs` (Phase 4f)

| Test | Asserts | P-ID |
|------|---------|:----:|
| `scan_manifest_items_finds_html_table` | E30 discovery | 4f |
| `scan_manifest_items_finds_equation_with_id` | E33 discovery | 4f |
| `table_analyze_success_when_t_enabled` | Extract role `t` | 4f |
| `equation_analyze_success_when_e_enabled` | Extract role `e` | 4f |

### `e2e_spec026_multimodal.rs` (updated)

| Test | Asserts |
|------|---------|
| `image_upload_vlm_describe_completes` | + KV `{doc_id}-multimodal-manifest` with 1 item |

### `contract_spec026_multimodal_chunks.rs` (Phase 4g)

| Test | Asserts | P-ID |
|------|---------|:----:|
| `build_mm_chunks_respects_process_options_filter` | E04 stale guard | 4g |
| `build_mm_chunks_rejects_failed_enabled_modality` | LightRAG failure defensive recheck | 4h |
| `mm_chunk_labels_match_lightrag_contract` | `[Image Name]` labels | 4g |

### `contract_spec026_multimodal_overwrite.rs` (Phase 4h)

| Test | Asserts | P-ID |
|------|---------|:----:|
| `reanalyze_overwrites_prior_image_description` | E05 image overwrite | 4h |
| `reanalyze_table_overwrites_prior_sidecar_result` | E05 table overwrite | 4h |

### `contract_spec026_multimodal_context.rs` (Phase 4j)

| Test | Asserts | P-ID |
|------|---------|:----:|
| `find_target_span_table_with_id_anywhere_in_attrs` | Table id attr order | 4j |
| `find_target_span_table_cite_marker` | Cite refid locator | 4j |
| `drawing_surrounding_kept_within_block` | Leading/trailing paragraph bounds | 4j |
| `table_surrounding_strips_sibling_tables` | Sibling table strip | 4j |
| `from_item_wires_token_budget_surrounding` | Analyzer context wiring | 4j |

### `contract_spec026_multimodal_prompt_cache.rs` (Phase 4i)

| Test | Asserts | P-ID |
|------|---------|:----:|
| `table_prompt_includes_additional_context_block` | LightRAG ADDITIONAL CONTEXT vars | 4i |
| `analysis_cache_skips_second_llm_call` | E44 KV cache hit on re-analyze | 4i |

### `contract_spec026_multimodal_sidecar.rs` (Phase 4k)

| Test | Asserts | P-ID |
|------|---------|:----:|
| `mm_chunks_and_modality_relations_from_sidecars` | Nested sidecar/heading/llm_cache_list + graph injection | 4k |
| `parse_mm_display_name_matches_chunk_format` | LightRAG `_parse_mm_display_name` contract | 4k |
| `blocks_jsonl_loader_keeps_first_content_row` | `load_content_rows_by_blockid` first-wins | 4k |

### `contract_spec026_multimodal_cache.rs` (Phase 4h — unit in `cache.rs`)

| Test | Asserts | P-ID |
|------|---------|:----:|
| `cache_disabled_by_default` | No keys unless `EDGEQUAKE_MM_ANALYSIS_CACHE=1` | 4h |
| `attaches_key_when_enabled` | `llm_cache_list` populated on success | 4h |

### `e2e_spec026_multimodal_retrieval.rs` (Phase 4g)

| Test | Asserts |
|------|---------|
| `vlm_image_mm_chunks_indexed_and_local_query_hits_content` | KV chunks + local query `source_chunk_ids` → VLM text |

Crate: `edgequake-tasks`

### `contract_spec026_task_delivery.rs`

| Test | Asserts | P-ID |
|------|---------|:----:|
| `local_delivery_sends_to_channel` | Default path unchanged | P-12 |
| `bridged_delivery_notifies_and_queues` | Both notifier + channel | P-12 |
| `storage_hydrating_loads_task_by_track_id` | External worker path | P-12 |
| `notify_only_skips_local_send` | API-only notify mode | P-12 |
| `delivery_mode_from_env_parses` | `local`/`bridged`/`notify_only` | P-12 |

---

## 4. E2E tests (Layer 4)

Crate: `edgequake-api/tests/`  
Pattern: `create_test_app_with_workers`, preload mock VLM response then extraction JSON.

### `e2e_spec026_multimodal.rs`

| Test | Flow | Asserts |
|------|------|---------|
| `image_upload_vlm_describe_completes` | POST `/documents/upload` PNG | status=completed; metadata `multimodal=true` |
| `image_upload_entities_extracted` | Same + mock VLM with Sarah Chen | graph node present |
| `image_upload_metadata_ingest_mode` | PNG upload | `ingest_mode=vlm_describe` |
| `tiny_image_upload_skips_vlm_with_default_limits` | POST 1×1 PNG (default min pixels) | `ingest_mode=vlm_skipped`; no VLM call |
| `text_upload_not_multimodal` | POST `/documents` text | no `multimodal` flag |

All tests use `#[serial]` (shared env + worker pool).

### `e2e_spec026_multimodal_pdf.rs`

| Test | Flow | Asserts |
|------|------|---------|
| `data_uri_enriched_markdown_ingest_extracts_entities` | enrich (i) → POST text | Sarah Chen in graph |
| `data_uri_tiny_image_skips_vlm_enrich` | enrich with default min pixels | data-URI placeholder kept |
| `drawing_tag_without_i_flag_skips_vlm` | enrich without `i` | markdown unchanged |
| `drawing_tag_with_i_flag_emits_sidecar_stub` | enrich with `i`, no bytes | sidecar pending comment |

### `e2e_spec026_multimodal_pdf_pipeline.rs` (Phase 4d)

| Test | Flow | Asserts |
|------|------|---------|
| `analyze_stage_then_ingest_extracts_entities` | `run_multimodal_analyze_stage` (i) → POST text | Sarah Chen in graph |

### `e2e_spec026_multimodal_reanalyze.rs` (Phase 4h)

| Test | Asserts |
|------|---------|
| `reanalyze_endpoint_updates_seeded_table_content` | E06 HTTP + KV content overwrite |
| `reanalyze_returns_404_for_missing_document` | fail-closed |

### Shared test SSOT

| Module | Purpose |
|--------|---------|
| `tests/common/spec026_multimodal.rs` | TINY_PNG, mock VLM JSON, `enable_vlm_process_in_tests`, multipart + markdown fixtures |
| `tests/common/spec026_delivery.rs` | Hydrating worker spawn for `notify_only` E2E |
| `tests/fixtures/spec026/` | `tiny.png`, `multimodal_image_analysis.json`, `mock_vlm_sarah.md` |

### `e2e_spec026_task_delivery.rs`

| Test | Flow | Asserts |
|------|------|---------|
| `bridged_delivery_processes_text_upload` | Env `EDGEQUAKE_TASK_DELIVERY=bridged` | document completes |
| `storage_hydrating_worker_processes_task` | Env `notify_only` + hydrating workers | document completes |

Both delivery E2E tests use `#[serial]` (env + global worker pool).

---

## 5. Regression guard (must stay green)

| Existing test | Why |
|---------------|-----|
| `e2e_spec026_ingestion_parity.rs` | Phase 2 chunking |
| `e2e_spec026_admission_saga.rs` | Staging saga |
| `e2e_spec024_text_upload_async.rs` | Async SSOT |

---

## 6. Exit criteria

```text
  [x] All contract_spec026_multimodal tests pass
  [x] All contract_spec026_task_delivery tests pass
  [x] All contract_spec026_llm_roles_vlm tests pass
  [x] e2e_spec026_multimodal.rs (5 tests) green
  [x] e2e_spec026_multimodal_pdf.rs (4 tests) green
  [x] e2e_spec026_task_delivery.rs (2 tests) green
  [x] contract_spec026_multimodal_pdf.rs (5 tests) green
  [x] tests/fixtures/spec026/ corpus present
  [x] Phase 2 E2E suite still green (ingestion_parity + admission_saga)
  [ ] cargo clippy --all-targets clean (blocked: pre-existing edgequake-query)
```

---

## 7. CI wiring

Add to existing SPEC-026 job after Phase 2 tests:

```bash
cargo test -p edgequake-core --test contract_spec026_llm_roles_vlm
cargo test -p edgequake-tasks --test contract_spec026_task_delivery
cargo test -p edgequake-api --test contract_spec026_multimodal
cargo test -p edgequake-api --test contract_spec026_multimodal_pdf
cargo test -p edgequake-api --test e2e_spec026_multimodal
cargo test -p edgequake-api --test e2e_spec026_multimodal_pdf
cargo test -p edgequake-api --test e2e_spec026_task_delivery
```

NATS live tests: `#[ignore]` + optional nightly with `EDGEQUAKE_NATS_URL`.
