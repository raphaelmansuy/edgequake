# 003 — Brutal Assessment: EdgeQuake Phase 4 vs LightRAG Multimodal

**Cross-ref:** [001 Plan](./001-phase4-scale-multimodal-plan.md) · [002 E2E Matrix](./002-e2e-test-matrix.md) · [004 Parity Implementation Plan](./004-multimodal-parity-implementation-plan.md)  
**LightRAG reference:** `/Users/raphaelmansuy/Github/03-working/LightRAG` (June 2026)  
**EdgeQuake reference:** `/Users/raphaelmansuy/Github/03-working/edgequake` (Phase 4 branch)  
**Date:** 2026-06-27  
**Tone:** Honest. The plan says “✅ Implemented.” This document explains what that actually means.

---

## 1. Executive verdict

| Dimension | Score | One-line truth |
|-----------|:-----:|----------------|
| **LightRAG analyze-stage parity** | **~35%** | Image VLM JSON schema + `i` gate only; not a port of `analyze_multimodal`. |
| **LightRAG sidecar / IR model** | **~5%** | No `blocks.jsonl`, no `drawings.json`, no asset dir, no per-item status. |
| **Production readiness (multimodal)** | **~55%** | Standalone image upload works; PDF inline is partial; resume/reprocess paths leak. |
| **Test honesty vs LightRAG** | **~45%** | Good contract/E2E for happy path + tiny skip; missing failure modes LightRAG tests heavily. |
| **EdgeQuake-only wins** | **Strong** | Postgres task SSOT, WebUI image routing, workspace VLM priority, admission saga. |

**Bottom line:** Phase 4 successfully **collapsed** LightRAG’s analyze stage into “VLM → markdown → existing text pipeline.” That is a valid product choice, not a parity port. Calling it “LightRAG multimodal gap closed” in marketing terms is **misleading**. Calling it “Phase 4a/4b shipped, 4c deferred” is **accurate**.

---

## 2. What EdgeQuake genuinely got right

These are real contributions, not checkbox theater.

### 2.1 Correct algorithm subset for standalone images

LightRAG has **no** first-class “upload a PNG and ingest it” API. EdgeQuake does:

```text
POST /documents/upload (image/*)
  → multimodal_admission.rs
  → vlm_provider_resolver.rs (workspace vision first)
  → vision_content.rs (LightRAG image_analysis JSON)
  → document_admission saga
```

That is a legitimate **EdgeQuake extension** documented in plan §9. The WebUI fix (`perform-file-upload.ts`) was necessary and correct.

### 2.2 VLM role resolution is cleaner for multi-tenant

LightRAG: global `VLM_LLM_BINDING` + `role_llm_funcs["vlm"]`.  
EdgeQuake: `LlmRole::Vlm` with workspace `llm_roles.vlm` → `vision_llm_*` → env → default LLM.

For enterprise workspaces this is **better** than LightRAG’s global env model, not worse.

### 2.3 Structured JSON → markdown is faithful

`vision_content.rs` matches LightRAG `prompt_multimodal.py` `image_analysis` schema (`name`, `type`, `description`), including `Other` fallback for unknown types. Contract tests prove it.

### 2.4 Task delivery (P-12) is architecturally sound

`enqueue_with_delivery` + Postgres SSOT + `StorageHydratingTaskQueue` is the right pattern for horizontal scale. LightRAG has **no** equivalent — in-process queues only. NATS is deferred, but bridged/hydrating E2E proves the abstraction.

### 2.5 Test SSOT is improving

`tests/common/spec026_multimodal.rs` and `spec026_delivery.rs` reduce duplication. Phase 2 regression (`ingestion_parity`, `admission_saga`) still passes. That matters.

---

## 3. What is missing — the uncomfortable list

### 3.1 There is no `analyze_multimodal` — only a inline enrich hook

LightRAG:

```text
q_parse  → sidecars always written (*.drawings.json, assets/)
q_analyze → analyze_multimodal(doc_id, blocks_path, process_options)
q_process → _build_mm_chunks_from_sidecars + normal chunking
```

EdgeQuake:

```text
PDF convert → enrich_markdown_with_vlm (optional, if `i`)
           → replace data-URI / stub <drawing/> in markdown string
           → normal text chunking + entity extract
```

**Missing entirely:**

| LightRAG concept | EdgeQuake status |
|------------------|------------------|
| Sidecar files (`*.parsed/`) | ❌ Not implemented |
| Per-item `llm_analyze_result` with `status` (`success` / `skipped` / `failed`) | ❌ |
| `_build_mm_chunks_from_sidecars` (dedicated multimodal chunks) | ❌ |
| Re-run analysis without re-parse | ❌ |
| `llm_cache_list` / analysis cache | ❌ |
| Fail-fast on analyze stage (`MultimodalAnalysisError` → doc FAILED) | ❌ Soft skip / placeholder instead |

EdgeQuake **inlines** VLM prose into markdown and hopes recursive chunking treats it like any other paragraph. LightRAG **indexes multimodal items as first-class chunks** with modality metadata and relation hooks. Retrieval behavior will diverge under load.

### 3.2 `process_options` is a lie for `t` and `e`

`MultimodalProcessOptions` parses `i/t/e`. Only `images` is wired:

- **`t` (tables):** LightRAG runs **extract** role on `tables.json` with `table_analysis` prompt. EdgeQuake: **nothing**.
- **`e` (equations):** LightRAG runs **extract** on `equations.json` with `equation_analysis` prompt. EdgeQuake: **nothing**.

Shipping `process_options` on PDF upload without table/equation handlers is **API surface with no behavior**. Users passing `ite` will think they get LightRAG parity. They do not.

### 3.3 `<drawing/>` tags are stubs, not analysis

When PDF markdown contains LightRAG-native placeholders:

```html
<drawing id="im-001" format="png" caption="Chart" />
```

EdgeQuake emits:

```html
<!-- multimodal:drawing:im-001 (VLM pending sidecar asset) -->
```

No asset loader. No path resolution. No VLM call. Plan labels this “Phase 4c,” but **most real MinerU/Docling PDF output uses drawing tags with external assets**, not data-URIs. **The primary PDF multimodal path is unimplemented.**

Data-URI enrichment (test fixture path) works. Production PDF paths often will not.

### 3.4 Surrounding context is dead code

`multimodal_context.rs` computes `(leading, trailing)` but **`describe_image` never receives it**. LightRAG injects `leading` / `trailing` into every multimodal prompt (token-budgeted, table-tag stripped, markup sanitized). EdgeQuake VLM calls are **image-only, context-blind**.

For figures referenced as “see chart above,” EdgeQuake quality will be worse than LightRAG even when VLM runs.

### 3.5 JSON robustness: LightRAG retries; EdgeQuake parses once

LightRAG `analyze_multimodal`:

- `json_repair.loads`
- Greedy `{…}` extraction
- LaTeX backslash repair
- **One conformance retry** on schema failure
- Hard fail document on second failure

EdgeQuake `parse_image_analysis_json`:

- Single `serde_json::from_str` on brace-trimmed text
- No repair, no retry
- Upload path: **warn + placeholder markdown** (document still “completes”)

This is a **philosophical divergence**: LightRAG treats bad VLM JSON as analyze-stage failure; EdgeQuake treats it as degraded ingest. Neither is wrong, but they are **not parity**.

### 3.6 `VLM_PROCESS_ENABLE` master switch — missing

LightRAG: if `process_options` includes `i` but VLM is disabled → **`MultimodalAnalysisError`, document FAILED**.

EdgeQuake: no equivalent gate. Missing vision provider → placeholder text, `ingest_mode=vlm_describe` may still apply on partial paths, document “succeeds” with useless body.

Operators cannot fail loud when VLM is misconfigured.

---

## 4. Behavioral divergences that will bite in production

### 4.1 `VLM_MIN_IMAGE_PIXEL` default: 4096 vs 64

| | LightRAG | EdgeQuake |
|---|----------|-----------|
| Default | **64** (`DEFAULT_MM_IMAGE_MIN_PIXEL`) | **4096** |
| Test override | Tests use 32px threshold | E2E sets `VLM_MIN_IMAGE_PIXEL=1` to force VLM on 1×1 fixture |

EdgeQuake skips **far more** images by default than LightRAG. A document full of small icons/diagrams will silently produce `vlm_skipped` stubs in EdgeQuake while LightRAG analyzes them.

The plan claims “LightRAG parity” on this constant. **The defaults are not parity.**

### 4.2 PNG-only dimension probing

`probe_image_dimensions` reads PNG IHDR only. JPEG/WebP uploads pass `(0, 0)` into validation → **pixel gate is bypassed** unless bytes exceed max size.

LightRAG reads dimensions from sidecar metadata / file probe at analysis time for all raster formats.

### 4.3 PDF resume path skips multimodal enrich

Confirmed in `pdf_processing.rs`: when `should_resume_from_checkpoint` and stored markdown exists, processing jumps to `process_text_insert` **without** `enrich_markdown_with_vlm`.

Scenario: first run converts PDF but entity extraction fails; retry resumes from DB markdown **never enriched** even if `process_options=i` was requested.

LightRAG re-enqueues analyze stage independently of parse resume semantics.

### 4.4 Reprocess drops `multimodal_process_options`

`reprocess.rs` and `bulk_ops/mod.rs` set `multimodal_process_options: None`. Reprocessed PDFs **will not** run inline VLM enrich.

### 4.5 WebUI never sends `process_options`

API accepts `process_options` on PDF multipart. WebUI always uploads PDF with `enable_vision: true` only. **No UI path enables inline image VLM (`i`)** for PDFs.

---

## 5. Test coverage — honest scorecard

### 5.1 What tests actually prove

| Claim | Proven? | How |
|-------|:-------:|-----|
| Image upload → VLM mock → entity in graph | ✅ | `e2e_spec026_multimodal` |
| JSON schema subset | ✅ | `contract_spec026_multimodal` |
| Tiny image skip (with EdgeQuake default 4096) | ✅ | `tiny_image_upload_skips_vlm_*` |
| Data-URI enrich → ingest | ✅ | `e2e_spec026_multimodal_pdf` (enrich in test, not PDF upload) |
| `i` gate disables enrich | ✅ | contract + E2E |
| Bridged / hydrating task delivery | ✅ | `e2e_spec026_task_delivery` |
| Drawing tag → sidecar stub | ✅ | contract + E2E (proves **stub**, not VLM) |

### 5.2 What tests do NOT prove (but LightRAG tests do)

| LightRAG test / behavior | EdgeQuake |
|--------------------------|-----------|
| `test_analyze_multimodal_invalid_json_hard_fails` | ❌ No test; soft-fail behavior instead |
| `test_analyze_multimodal_overwrites_already_analyzed_items` | ❌ |
| `test_build_mm_chunks_respects_process_options_filter` | ❌ No multimodal chunks |
| `test_mm_chunks_and_modality_relations_from_sidecars` | ❌ |
| Table → extract role (not VLM) | ❌ |
| `VLM_PROCESS_ENABLE=false` + `i` → hard fail | ❌ |
| Full PDF upload → convert → enrich → graph E2E | ❌ |
| Surrounding context in prompts | ❌ |
| NATS delivery | ❌ Deferred |

**E2E honesty issue:** `e2e_spec026_multimodal_pdf::data_uri_enriched_markdown_ingest_extracts_entities` calls `enrich_markdown_with_vlm` **in the test**, then POSTs text. It does **not** exercise `pdf_processing.rs` hook end-to-end. The plan implies PDF inline E2E; the test proves **analyze→process substring**, not PDF pipeline integration.

### 5.3 Test env hacks mask production behavior

Multimodal E2E sets `VLM_MIN_IMAGE_PIXEL=1` so 1×1 fixture PNG reaches mock VLM. Production default is 4096. **Tests green ≠ production VLM runs on typical inline images.**

All multimodal E2E uses `#[serial]` because global env + worker pool are shared. Fine for CI, but hides parallelism bugs.

---

## 6. Architecture comparison (ASCII)

```text
LightRAG (full multimodal)
══════════════════════════
  Upload doc
      │
      ▼
  [PARSE]  always write sidecars + assets
      │
      ▼
  [ANALYZE]  gated by i/t/e
      │         ├── VLM (drawings) + surrounding context
      │         ├── EXTRACT (tables)
      │         └── EXTRACT (equations)
      │         per-item status + cache + retry JSON
      ▼
  [PROCESS]  text chunks + mm_chunks from sidecars
      │
      ▼
  Graph / vector index


EdgeQuake Phase 4 (collapsed)
═════════════════════════════
  Upload image ──► VLM ──► markdown ──► admission ──► chunk ──► graph
                         (no sidecar)

  Upload PDF ──► convert ──► enrich_markdown_with_vlm? (if `i`, non-resume)
                    │              │
                    │              ├── data-URI: VLM ✅
                    │              └── <drawing/>: stub ❌
                    ▼
               text_insert ──► chunk ──► graph
               (same path as plain text; no mm_chunks)
```

---

## 7. P-12 task delivery vs LightRAG

| | LightRAG | EdgeQuake |
|---|----------|-----------|
| Horizontal workers | In-process asyncio queues | Postgres SSOT + notify + hydrate |
| NATS / external bus | N/A | Designed, not shipped |
| Worker failure recovery | Pipeline requeue | `TaskRuntime` + existing orphan recovery |

**Verdict:** P-12 is an EdgeQuake **lead**, not a LightRAG catch-up item. The delivery abstraction is real; production NATS wiring is still a checklist item.

---

## 8. Should the plan say “✅ Implemented”?

| Workstream | Honest status |
|------------|---------------|
| P-07a Image VLM ingest | ✅ **Shipped** (with soft-fail semantics) |
| P-07b PDF inline images | ⚠️ **Partial** — data-URI only; drawing/sidecar path stubbed |
| P-07b tables/equations (`t`/`e`) | ❌ **Not started** (parser exists, zero handlers) |
| P-12 Task delivery | ✅ **Pattern proven**; NATS ❌ |
| Phase 4c (plan §8) | ❌ **Correctly deferred** — but it is most of LightRAG parity |

Recommended plan status line:

> **Status:** Phase 4a ✅ · Phase 4b ⚠️ partial · Phase 4c ❌ · LightRAG analyze parity ~35%

---

## 9. Prioritized recommendations

### P0 — Correctness / user trust

1. **Fix PDF resume + reprocess** to call `enrich_markdown_with_vlm` when `process_options` includes `i` (or persist enriched markdown at first convert).
2. **Align `VLM_MIN_IMAGE_PIXEL` default with LightRAG (64)** or document divergence prominently in API/docs; 4096 is not “parity.”
3. **JPEG/WebP dimension probing** or fail closed on unknown dimensions.
4. **WebUI:** expose `process_options` on PDF upload (minimum: “Analyze inline images” toggle → `i`).

### P1 — Real PDF multimodal

5. **Sidecar asset loader** for `<drawing path="…"/>` — without this, PDF multimodal is a demo feature.
6. **Wire `surrounding_context` into `describe_image` prompts** (even 512-char stub beats nothing).
7. **Full E2E:** upload real PDF fixture → convert → enrich → entity in graph (no test-local enrich call).

### P2 — LightRAG parity depth

8. **Table/equation handlers** using extract role (not VLM) — copy LightRAG prompt keys from `prompt_multimodal.py`.
9. **JSON repair + one retry** on VLM response (port `_json_extract` / conformance retry subset).
10. **`VLM_PROCESS_ENABLE` equivalent** — fail document when `i` requested but no vision provider.
11. **Multimodal chunk type** (optional) — if retrieval quality gap observed vs LightRAG.

### P3 — Ops

12. **NATS notifier** behind feature flag.
13. **Analysis cache** keyed by image hash + model (cost control).

---

## 10. Final word

EdgeQuake Phase 4 is **not** a Rust port of LightRAG multimodal. It is a **pragmatic shortcut**: run VLM once, emit markdown, reuse the text ingestion highway. That shortcut is defensible for time-to-market and plays to EdgeQuake strengths (Postgres, workspaces, WebUI, saga admission).

It is **not** defensible to claim analyze-stage parity, sidecar parity, or `ite` parity. The code, tests, and production edge cases (resume, reprocess, drawing tags, default pixel gate) all say otherwise.

**Use LightRAG as the north star for Phase 4c and retrieval quality tuning. Do not pretend Phase 4 already arrived there.**

---

## Appendix A — Key file mapping

| LightRAG | EdgeQuake | Parity |
|----------|-----------|:------:|
| `pipeline.py::analyze_multimodal` | `multimodal_markdown.rs::enrich_markdown_with_vlm` | Partial |
| `prompt_multimodal.py` (image) | `vision_content.rs` | High |
| `prompt_multimodal.py` (table/equation) | — | None |
| `llm_roles.py` vlm | `llm_roles.rs::Vlm` + `vlm_provider_resolver.rs` | High (+ workspace) |
| `multimodal_context.py` | `multimodal_context.rs` | Stub only |
| `sidecar/writer.py` | — | None |
| `sidecar/placeholders.py` | `inline_images.rs` (scan only) | Low |
| `parse_process_options` | `MultimodalProcessOptions::from_option_str` | Parse only |
| In-proc task queues | `edgequake-tasks/delivery/*` | EdgeQuake lead |
| Standalone image upload | `multimodal_admission.rs` | EdgeQuake lead |

## Appendix B — LightRAG test files worth porting next

1. `tests/pipeline/test_pipeline_analyze_multimodal.py` — JSON retry, VLM kwargs, cache
2. `tests/pipeline/test_multimodal_surrounding_context.py` — context injection
3. `tests/pipeline/test_pipeline_release_closure.py::test_build_mm_chunks_*` — only if mm_chunks adopted
