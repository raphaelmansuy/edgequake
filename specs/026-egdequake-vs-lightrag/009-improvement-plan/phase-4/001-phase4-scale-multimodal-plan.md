# 001 — Phase 4 Scale & Multimodal Plan (6 weeks)

**Cross-ref:** [README](./README.md) · [002 E2E Matrix](./002-e2e-test-matrix.md) · [003 Brutal LightRAG Assessment](./003-brutal-lightrag-assessment.md) · [004 Parity Implementation Plan](./004-multimodal-parity-implementation-plan.md) · [001 Improvement Plan](../001-improvement-plan.md)  
**LightRAG reference:** `lightrag/pipeline.py`, `lightrag/prompt_multimodal.py`, `lightrag/llm_roles.py`, `lightrag/multimodal_context.py`

**Date:** 2026-06-27  
**Status:** Phase 4a ✅ · Phase 4b ⚠️ partial · Phase 4c ❌ — see [003 Brutal Assessment](./003-brutal-lightrag-assessment.md)  
**Goal:** Close LightRAG multimodal ingest gap (P-07) and enable horizontal worker scale (P-12) while preserving EdgeQuake Postgres-native durability.  
**Principle:** Port LightRAG *algorithms* as traits/modules — no monolith ports, no duplicate upload paths.

---

## 1. Current State (baseline)

### EdgeQuake strengths to preserve

```text
  SSOT (post Phase 2)                     Do NOT fork
  ───────────────────                     ────────────
  document_admission.rs                   per-handler admission
  PostgresTaskStorage                     in-memory task loss
  ChannelTaskQueue (default)              remove without external bridge
  vision PDF via edgequake-pdf2md         rewrite PDF stack
  llm_roles (extract/query/summary/vlm)   global env-only overrides
```

### Gaps vs LightRAG (June 2026 reference)

| Gap | LightRAG source | EdgeQuake today |
|-----|-----------------|-----------------|
| Structured VLM image analysis | `prompt_multimodal.py` → JSON `{name,type,description}` | ✅ `vision_content.rs` |
| VLM LLM role + workspace priority | `llm_roles.py` → `RoleSpec("vlm", …)` | ✅ `LlmRole::Vlm` + `vlm_provider_resolver.rs` |
| Analyze stage (parse → VLM → process) | `pipeline.py::_analyze_worker` | ✅ Image upload (4a); PDF data-URI enrich (4b); drawing sidecar stub (4c) |
| Process options `i/t/e` | `parse_process_options` | ✅ `MultimodalProcessOptions` |
| Surrounding context for VLM | `multimodal_context.py` | ✅ Stub in `multimodal_context.rs` (Phase 4c full wiring) |
| External worker scale | In-proc queues only | ✅ Bridged + notify_only + hydrating worker (P-12); NATS deferred |
| Tiny image skip (VLM_MIN gate) | `test_analyze_multimodal_skips_tiny_image_without_vlm_call` | ✅ `probe_image_dimensions` + `vlm_skipped` ingest mode |

### Existing foundation extended

| Module | Role |
|--------|------|
| `services/vision_content.rs` | VLM prompts + JSON parse + markdown emit |
| `services/multimodal_admission.rs` | File/image → text + metadata (upload SSOT) |
| `services/vlm_provider_resolver.rs` | Workspace `vision_llm_*` → `llm_*` → env fallback |
| `services/multimodal_markdown.rs` | Post-PDF-convert inline VLM enrich |
| `services/multimodal_context.rs` | Surrounding-context stub for sidecar items |
| `services/vlm_limits.rs` | `VLM_MIN_IMAGE_PIXEL` / `VLM_MAX_IMAGE_BYTES` |
| `vision_env.rs` | Env-based vision provider SSOT |
| `edgequake-pdf/inline_images.rs` | `<drawing/>` + data-URI scan |
| `edgequake-tasks/delivery/` | Notifier, bridge, storage-hydrating |
| `edgequake_webui/src/lib/upload/` | PDF / image / text routing (DRY) |

---

## 2. LightRAG analyze stage — distilled algorithm

LightRAG runs three in-process queue workers per batch:

```text
  q_parse  ──► parse document → blocks.jsonl + sidecars
       │
  q_analyze ──► analyze_multimodal(doc_id, blocks_path)
       │           ├── gate: process_options i/t/e
       │           ├── enrich_sidecars_with_surrounding()
       │           ├── VLM per drawing/table/equation item
       │           └── write llm_analyze_result to sidecar
       │
  q_process ──► chunk + entity extract (multimodal chunks injected)
```

**EdgeQuake Phase 4a (image upload):** Collapse parse+analyze for standalone images:

```text
  POST /documents/upload (image/png)
       └── multimodal_admission::resolve_upload_content()
             └── vlm_provider_resolver (workspace vision first)
             └── vision_content::describe_image()  [VLM role]
                   └── markdown body → document_admission (existing saga)
```

**EdgeQuake Phase 4b (PDF inline):** After EdgeParse conversion:

```text
  processor/pdf_processing.rs
       └── enrich_markdown_with_vlm(markdown, process_options, …)
             ├── scan_inline_image_refs (drawing tags + data-URI)
             ├── gate: MultimodalProcessOptions.images ('i')
             ├── data-URI → describe_image → markdown block
             └── drawing tag (no bytes) → sidecar stub comment (4c)
```

---

## 3. SOLID / DRY architecture

### 3.1 Single Responsibility — Vision extraction

```text
  edgequake-api/src/
  ├── vision_env.rs                 (env vision provider SSOT)
  ├── services/
  │   ├── vision_content.rs         (VLM prompts + JSON parse + markdown emit)
  │   ├── vlm_provider_resolver.rs  (workspace vision → main llm → env chain)
  │   ├── multimodal_admission.rs   (file/image → text + metadata)
  │   ├── multimodal_markdown.rs    (post-convert inline enrich)
  │   ├── multimodal_context.rs     (surrounding context stub)
  │   └── vlm_limits.rs             (pixel/byte gates)
  │
  handlers/documents/upload/
  ├── file_upload.rs                (HTTP → multimodal_admission)
  └── batch_upload.rs               (same)

  edgequake_webui/src/lib/upload/
  ├── file-kind.ts                  (pdf | image | text)
  └── perform-file-upload.ts        (multipart vs JSON SSOT)
```

**SRP:** `vision_content` owns VLM I/O; `vlm_provider_resolver` owns provider selection; `multimodal_admission` owns upload routing; handlers own HTTP.

### 3.2 Open/Closed — Task delivery

```text
  edgequake-tasks/src/delivery/
  ├── mod.rs              (TaskDeliveryMode from env)
  ├── notifier.rs         (TaskNotifier trait)
  ├── bridge.rs           (Channel bridge for tests / hybrid)
  ├── storage_hydrating.rs (external worker: notify → load from Postgres)
  └── nats.rs             (optional feature — deferred)

  Flow (Postgres SSOT unchanged):
    enqueue: storage.create_task(task) → notifier.notify(track_id) → [optional] local.send(task)
    worker:  receive track_id → storage.get_task(track_id) → process

  Test SSOT (edgequake-api/tests/common/):
  ├── spec026_multimodal.rs   (fixtures: TINY_PNG, mock VLM JSON, markdown helpers)
  └── spec026_delivery.rs     (hydrating worker spawn for notify_only E2E)
```

**Dependency Inversion:** `TaskRuntime` depends on `TaskNotifier`, not NATS directly.

### 3.3 LlmRole::Vlm

Mirror LightRAG fallback chain:

```text
  workspace.llm_roles.vlm → workspace.vision_llm_* → workspace.llm_*
       → EDGEQUAKE_VISION_* env → startup vision_llm_provider → default text LLM
```

---

## 4. Workstreams & timeline

### Week 1–2 — P-07a Image VLM ingest ✅

| Task | Owner module | LightRAG ref |
|------|--------------|--------------|
| `LlmRole::Vlm` + resolver | `vlm_provider_resolver.rs` | `llm_roles.py` ROLES |
| Structured image prompt + JSON | `vision_content.rs` | `prompt_multimodal.py` image_analysis |
| Upload SSOT | `multimodal_admission.rs` | N/A (LR has no standalone image API) |
| WebUI binary image routing | `perform-file-upload.ts` | N/A |
| Metadata: `ingest_mode`, `multimodal` | `document_admission.rs` | `multimodal_processed` sentinel |
| Contract + E2E tests | `contract_spec026_multimodal.rs`, `e2e_spec026_multimodal.rs` | — |

### Week 3–4 — P-07b PDF inline images ✅ (minimal)

| Task | Owner module | LightRAG ref |
|------|--------------|--------------|
| Inline image scan | `edgequake-pdf/inline_images.rs` | `analyze_multimodal` item loop |
| Post-convert enrich hook | `multimodal_markdown.rs` + `pdf_processing.rs` | `_analyze_worker` |
| `MultimodalProcessOptions` on PDF upload | `pdf_upload` + `PdfProcessingData` | `process_options` i/t/e |
| Surrounding context stub | `multimodal_context.rs` | `multimodal_context.py` |
| Contract + E2E | `contract_spec026_multimodal_pdf.rs`, `e2e_spec026_multimodal_pdf.rs` | — |
| Local pdf2md path dep | workspace `Cargo.toml` + `[patch.crates-io]` | — |

### Week 4–6 — P-12 External task queue ✅ (NATS deferred)

| Task | Owner module | Notes |
|------|--------------|-------|
| `TaskNotifier` + `TaskDeliveryMode` | `edgequake-tasks/delivery/` | Postgres remains SSOT |
| `StorageHydratingTaskQueue` | same | Distributed worker receive path |
| `NatsTaskNotifier` (feature) | `nats.rs` | **Deferred** — bridged/hydrating prove pattern |
| Hydrating worker E2E | `spec026_delivery.rs` | `notify_only` + lifecycle parity |
| Contract + E2E | `contract_spec026_task_delivery.rs`, `e2e_spec026_task_delivery.rs` | — |

---

## 5. Configuration

| Variable | Default | Purpose |
|----------|---------|---------|
| `EDGEQUAKE_TASK_DELIVERY` | `local` | `local` \| `bridged` \| `notify_only` |
| `EDGEQUAKE_NATS_URL` | — | Required when NATS feature enabled |
| `EDGEQUAKE_NATS_SUBJECT` | `edgequake.tasks` | Notify subject |
| `EDGEQUAKE_VISION_*` | — | Env fallback for VLM provider |
| `VLM_MIN_IMAGE_PIXEL` | `4096` | Min pixels (LightRAG parity) |
| `VLM_MAX_IMAGE_BYTES` | `5242880` | Max image size for VLM |

---

## 6. Exit criteria

| Criterion | Target | Status |
|-----------|--------|--------|
| Image upload E2E | VLM text → entity in graph | ✅ |
| Structured metadata | `multimodal=true`, `ingest_mode=vlm_describe` | ✅ |
| VLM role resolution | Workspace `vision_llm_*` / `llm_roles.vlm` honored | ✅ |
| PDF inline enrich E2E | data-URI → VLM → entity in graph | ✅ |
| Task delivery E2E | bridged + hydrating (`notify_only`) | ✅ |
| WebUI image upload | multipart route, Playwright | ✅ |
| Regression | Phase 2 E2E green | ✅ (ingestion_parity + admission_saga) |
| Clippy | `--all-targets` clean on touched crates | ⚠️ pre-existing `edgequake-query` too_many_arguments |

---

## 7. Implementation checklist

```text
Week 1–2
  [x] LlmRole::Vlm + resolve fallback chain
  [x] vision_content.rs (structured JSON → markdown)
  [x] vlm_provider_resolver.rs (workspace vision priority)
  [x] multimodal_admission.rs (DRY file + batch upload)
  [x] WebUI upload SSOT (file-kind + perform-file-upload)
  [x] document_admission multimodal metadata
  [x] contract_spec026_multimodal.rs
  [x] e2e_spec026_multimodal.rs
  [x] tests/common/spec026_multimodal.rs (DRY fixtures)
  [x] tests/fixtures/spec026/ (tiny.png, mock JSON, mock markdown)

Week 3–4
  [x] inline_images.rs (drawing tag + data-URI scan)
  [x] MultimodalProcessOptions (i/t/e)
  [x] multimodal_markdown.rs + PDF processor hook
  [x] multimodal_context.rs surrounding-context stub
  [x] vlm_limits.rs
  [x] contract_spec026_multimodal_pdf.rs
  [x] e2e_spec026_multimodal_pdf.rs
  [x] Local edgequake-pdf2md path dependency

Week 4–6
  [x] delivery/ module (notifier, bridge, storage_hydrating)
  [ ] nats-queue optional feature — deferred
  [x] TaskRuntime delivery modes + channel_notifier accessor
  [x] tests/common/spec026_delivery.rs (hydrating worker E2E)
  [x] contract_spec026_task_delivery.rs
  [x] e2e_spec026_multimodal.rs (5 tests incl. tiny skip)
  [x] e2e_spec026_multimodal_pdf.rs (4 tests incl. tiny data-URI skip)
  [x] contract tiny-image gate tests (vlm_limits + multimodal_pdf)
  [x] e2e_spec026_task_delivery.rs (bridged + hydrating)
```

---

## 8. Phase 4c (deferred)

| Item | LightRAG ref | Notes |
|------|--------------|-------|
| Sidecar asset hydration for `<drawing/>` tags | `sidecar/placeholders.py` | Stub comment emitted today |
| Table/equation VLM prompts | `prompt_multimodal.py` t/e | `MultimodalProcessOptions` ready |
| Full surrounding context injection | `multimodal_context.py` | Stub returns empty lead/trail |
| NATS production bridge | — | `EDGEQUAKE_NATS_URL` feature flag |

---

## 9. What LightRAG does NOT have (EdgeQuake extensions)

| Extension | Why keep |
|-----------|----------|
| Postgres task SSOT | Durability across restarts |
| NATS/SQS delivery | Horizontal worker scale (P-12) |
| Staging admission saga | No KV orphans |
| Workspace tenancy + vision LLM fields | Enterprise isolation |
| WebUI multipart image upload | Standalone image ingest |

These remain **contributions EdgeQuake leads** — document in Phase 5 upstream recommendations.

---

## 10. LightRAG test parity map

| LightRAG test (`tests/pipeline/test_pipeline_release_closure.py`) | EdgeQuake test | Layer |
|-------------------------------------------------------------------|----------------|-------|
| `test_analyze_multimodal_skips_tiny_image_without_vlm_call` | `tiny_image_upload_skips_vlm_with_default_limits`, `enrich_skips_tiny_data_uri_without_vlm_call`, `data_uri_tiny_image_skips_vlm_enrich` | contract + E2E |
| `test_analyze_multimodal_unknown_image_type_folds_to_other` | `image_analysis_invalid_type_falls_back_to_other` | contract |
| `test_analyze_multimodal_overwrites_already_analyzed_items` | — | deferred (4c sidecar re-run) |
| `test_analyze_multimodal_invalid_json_hard_fails` | — | deferred (live VLM error path) |
| Image JSON schema `{name,type,description}` | `image_analysis_json_parses_lightrag_schema` | contract |
| `process_options` i/t/e gate | `multimodal_process_options_parse_ite`, `drawing_tag_without_i_flag_skips_vlm` | contract + E2E |
| External worker notify + hydrate | `storage_hydrating_worker_processes_task` | E2E |

**Test SSOT helpers** (`tests/common/`):

| Module | Exports |
|--------|---------|
| `spec026_multimodal.rs` | `TINY_PNG`, `MOCK_VLM_SARAH_JSON`, `allow_tiny_images_in_tests()`, `png_upload_request`, `text_upload_request`, `parse_accepted_upload`, markdown fixtures |
| `spec026_delivery.rs` | `spawn_hydrating_workers`, `wait_for_hydrating_workers_ready` |

All multimodal E2E tests use `#[serial]` — they share `VLM_MIN_IMAGE_PIXEL` env and the global worker pool.
