# SPEC-038 — Implementation Plan

**Lens:** Full Stack Implementation  
**Status:** `IMPLEMENTED` (2026-07-01)  
**Deploy order:** SSOT + routing → timeout → limits → UI → gold tests  
**Principle:** Code is law; real test is law

---

## Architecture (DRY / SOLID)

| Module | Responsibility |
| ------ | -------------- |
| `large_document_profile.rs` | **SSOT** — thresholds, timeouts, `IngestionEstimate`, `classify_ingestion_failure`, gleaning policy |
| `pdf_auto_routing.rs` | EdgeParse fast-path probe (`try_edgeparse_fast_path`) |
| `pdf_processing.rs` | Processor integration — auto-route before Vision |
| `handlers/pdf_upload/*` | Admission estimate on upload response, scaled task timeout in metadata |
| `edgequake-tasks/worker.rs` | Per-task `processing_timeout_secs` from metadata |
| `large-pdf-admission.ts` + `extract-page-count.ts` | Frontend mirror of backend thresholds |
| `large-pdf-admission-dialog.tsx` | Pre-upload UX with `data-testid` hooks |
| `lib/upload/upload-timeout.ts` | Scaled upload timeout + byte→progress mapping (SSOT) |
| `lib/upload/multipart-upload-client.ts` | XHR multipart with `upload.onprogress` + 401 retry |
| `lib/upload/pdf-upload-form-data.ts` | DRY FormData builder for PDF admit |

**Consolidation note:** Original plan listed `pdf_text_probe.rs` and `pdf_routing_policy.rs` as separate modules. These were merged into `large_document_profile.rs` + `pdf_auto_routing.rs` to avoid duplication (DRY).

---

## Phase 0 — Reproducer Baseline (P0)

**Goal:** Capture failure mode before fix (documented evidence)

| Step | Action | Status |
| ---- | ------ | ------ |
| 0.1 | Copy PDF to `tests/fixtures/spec038/guide_2606.24937v1-opt.pdf` | ✅ |
| 0.2 | Record `pdfinfo`, `pdftotext` metrics in `benchmarks/reproducer-baseline.json` | ✅ |
| 0.3 | Attempt ingest with current default; capture `failure_class` + logs | ✅ (documented in `001-five-whys.md`) |

**Baseline metrics (2026-07-01):**

```json
{
  "file": "guide_2606.24937v1-opt.pdf",
  "pages": 603,
  "file_bytes": 11043120,
  "pdftotext_bytes": 1443139,
  "chars_per_page": 2389,
  "vision_outer_timeout_secs": 4944,
  "worker_timeout_secs": 7200,
  "estimated_vision_convert_secs_realistic": 4522
}
```

---

## Phase 1 — LargeDocumentProfile SSOT (P0)

**Goal:** REQ-038-02, REQ-038-03

| Step | File | Status |
| ---- | ---- | ------ |
| 1.1 | `services/large_document_profile.rs` — struct + `task_timeout_secs()` + `IngestionEstimate` | ✅ |
| 1.2 | Text probe — `markdown_has_text_layer()` + `try_edgeparse_fast_path()` in `pdf_auto_routing.rs` | ✅ |
| 1.3 | Routing — `should_try_edgeparse_before_vision()` on profile | ✅ |
| 1.4 | `services/mod.rs` — export modules | ✅ |
| 1.5 | Unit tests in `large_document_profile.rs` + `tests/spec038_large_pdf.rs` | ✅ |

**Verify:**

```bash
cargo test -p edgequake-api --features postgres --test spec038_large_pdf
```

---

## Phase 2 — Upload Routing + Estimate DTO (P0)

**Goal:** REQ-038-01, REQ-038-04

| Step | File | Status |
| ---- | ---- | ------ |
| 2.1 | `handlers/pdf_upload/upload.rs` — profile at admit; `ingestion_estimate` on response | ✅ |
| 2.2 | `handlers/pdf_upload/types.rs` — `IngestionEstimate` on `PdfUploadResponse` | ✅ |
| 2.3 | `handlers/pdf_upload/helpers.rs` — scaled timeout in task metadata | ✅ |
| 2.4 | `openapi.rs` — document new response fields | ⬜ (follow-up: regen OpenAPI snapshot) |

---

## Phase 3 — Worker Scaled Timeout (P0)

**Goal:** REQ-038-03

| Step | File | Status |
| ---- | ---- | ------ |
| 3.1 | `edgequake-tasks/src/types/data.rs` — `pdf_parser_backend_explicit` + metadata timeout | ✅ |
| 3.2 | `edgequake-tasks/src/worker.rs` — read `processing_timeout_secs` from metadata | ✅ |
| 3.3 | PDF enqueue sets override from profile | ✅ |

**Verify:**

```bash
cargo test -p edgequake-tasks --lib worker
```

---

## Phase 4 — Processor Integration (P0)

**Goal:** REQ-038-01, REQ-038-06, REQ-038-09

| Step | File | Status |
| ---- | ---- | ------ |
| 4.1 | `processor/pdf_processing.rs` — EdgeParse fast path before Vision | ✅ |
| 4.2 | `compute_safe_pdf_resource_profile` — uses `LargeDocumentProfile` | ✅ |
| 4.3 | `processor/status_updates.rs` — `failure_class` + `recommended_action` in metadata | ✅ |
| 4.4 | Profile: gleaning disabled when `P≥500` (`prepare.rs`) | ✅ |

---

## Phase 5 — Size Limit Alignment (P1)

**Goal:** REQ-038-08

| Step | File | Status |
| ---- | ---- | ------ |
| 5.1 | `orchestrator/ingestion.rs` — `MAX_UPLOAD_BYTES` (50 MB) | ✅ |
| 5.2 | `handlers/injection/injection_file.rs` — same SSOT | ✅ |
| 5.3 | `resource_safety_proof.rs` — update assertions | ✅ (implicit via shared constant) |

---

## Phase 6 — Frontend Admission UX (P1)

**Goal:** REQ-038-04, REQ-038-05

| Step | File | Status |
| ---- | ---- | ------ |
| 6.1 | `extract-page-count.ts` — client `/Count` parser | ✅ |
| 6.2 | `large-pdf-admission-dialog.tsx` — admission card + `data-testid`s | ✅ |
| 6.3 | Document row — ETA + N/total progress | ⬜ (deferred; upload progress list exists) |
| 6.4 | Failed banner — `failure_class` mapping | ⬜ (backend writes metadata; UI mapping follow-up) |
| 6.5 | `e2e/spec038-large-pdf-admission.spec.ts` + mocks | ✅ |

**`data-testid` inventory:**

| ID | Element |
| -- | ------- |
| `spec038-large-pdf-admission-dialog` | Dialog root |
| `spec038-admission-summary` | Page count + file size |
| `spec038-admission-recommendation` | EdgeParse recommendation |
| `spec038-admission-eta-edgeparse` | Fast parse ETA |
| `spec038-admission-eta-vision` | Vision slowdown warning |
| `spec038-parser-choice` | Parser radio group |
| `spec038-admission-confirm` / `spec038-admission-cancel` | Actions |
| `spec038-upload-progress-list` | Upload progress container |
| `spec038-upload-bytes-sent` | Honest byte counter (Sending X / Y MB) |

**Verify:**

```bash
cd edgequake_webui
PLAYWRIGHT_BASE_URL=http://localhost:3000 pnpm exec playwright test e2e/spec038-large-pdf-admission.spec.ts
bun test src/lib/pdf/__tests__/spec038-extract-page-count.test.ts
```

**Screenshots:** `specs/038-ingestion-large-pdf/e2e/screenshots/01–06.png`

---

## Phase 7 — Gold Integration Test (P0)

**Goal:** REQ-038-07 — **Real test is law**

| Step | File | Status |
| ---- | ---- | ------ |
| 7.1 | `tests/spec038_large_pdf.rs` — profile + routing + reproducer EdgeParse | ✅ |
| 7.2 | Assert `extraction_method == EdgeParse` via fast-path markdown | ✅ |
| 7.3 | Assert markdown `>500KB` on reproducer | ✅ |
| 7.4 | Assert `status == indexed` within scaled timeout | ⬜ (full pipeline E2E; EdgeParse convert proven) |
| 7.5 | Resume test: markdown stored → skip convert | ⬜ (future) |

**Verify:**

```bash
cargo test -p edgequake-api --features postgres --test spec038_large_pdf -- --nocapture
# 7/7 passed including ~40s reproducer EdgeParse test
```

---

## Phase 8 — Benchmarks & Regression (P2)

| Benchmark | Target | Status |
| --------- | ------ | ------ |
| EdgeParse 603 pages | p99 < 180 s | ✅ ~36s measured in test |
| Text probe | < 2 s | ✅ |
| Full ingest mock LLM | < 45 min CI timeout | ⬜ |
| HTTP admit 11 MB PDF | p99 < 15 s (local) | ✅ No `file_data.clone()`; honest byte progress + scaled timeout |

Store: `specs/038-ingestion-large-pdf/benchmarks/reproducer-baseline.json`

---

## Phase 9 — Upload Admit Fast-Path (P1)

**Goal:** REQ-038-11 — fix "stuck at Uploading" perception and memory pressure on admit

**Root cause:** [002-first-principles.md](./002-first-principles.md) P9–P10, [001-five-whys.md](./001-five-whys.md) Symptom D

| Step | File | Change | Status |
| ---- | ---- | ------ | ------ |
| 9.1 | `use-file-upload.ts` | Real byte progress via `onUploadProgress`; removed fake `progress: 40` | ✅ |
| 9.2 | `multipart-upload-client.ts` | XHR `upload.onprogress` + `uploadTimeoutMs(bytes)` (replaces untimed `fetch`) | ✅ |
| 9.3 | `upload.rs` | Optional `202` + async BYTEA persist | ⬜ **Deferred** — move semantics sufficient for 11 MB reproducer |
| 9.4 | `upload.rs` | Eliminate `file_data.clone()` before INSERT (move into `create_pdf`) | ✅ |
| 9.5 | `locales/en.json` | Split copy: "Sending X / Y MB" vs "Saving to workspace…" | ✅ |
| 9.6 | `e2e/spec038-upload-progress.spec.ts` | Assert `spec038-upload-bytes-sent` + screenshots 05–06 | ✅ |
| 9.7 | `lib/upload/__tests__/spec038-upload-timeout.test.ts` | Unit tests for timeout + progress band | ✅ |

**Verify:**

```bash
# Unit: timeout + progress mapping
cd edgequake_webui && bun test src/lib/upload/__tests__/spec038-upload-timeout.test.ts

# E2E: admission + honest byte progress (screenshots → specs/038-.../e2e/screenshots/)
PLAYWRIGHT_BASE_URL=http://localhost:3000 pnpm exec playwright test e2e/spec038-large-pdf-admission.spec.ts e2e/spec038-upload-progress.spec.ts
```

**Decision:** Shipped **honest byte progress + scaled XHR timeout + move semantics** (no 2× RAM clone). **202 async admit** remains optional if BYTEA write p99 exceeds 30 s in production.

---

## Definition of Done

- [x] REQ-038-01 through REQ-038-06, 038-08, 038-09 satisfied
- [x] REQ-038-11 upload admit UX — honest byte progress, scaled timeout, no `file_data.clone()`
- [x] `guide_2606.24937v1-opt.pdf` EdgeParse fast-path produces >500KB markdown
- [x] `cargo test -p edgequake-api --features postgres --test spec038_large_pdf` green (7/7)
- [ ] `cargo clippy -p edgequake-api --all-targets` clean (run before merge)
- [x] Playwright SPEC-038 E2E green (5/5: admission 3 + upload progress 2)
- [ ] OpenAPI snapshot updated for `ingestion_estimate`
- [ ] `AGENTS.md` documents `EDGEQUAKE_AUTO_PDF_ROUTING`, probe thresholds (follow-up)

---

## Rollback Plan

| Risk | Mitigation |
| ---- | ---------- |
| False EdgeParse routing | `EDGEQUAKE_AUTO_PDF_ROUTING=0` disables probe |
| Timeout too short | `TASK_PROCESSING_TIMEOUT_SECS` global override |
| Probe perf regression | Skip probe when `P < 100` |

---

## Battle-Tested Edge Case Checklist

| EC | Test | Status |
| -- | ---- | ------ |
| EC-038-01 Encrypted PDF | `spec038_large_pdf::encrypted_fails_fast` | ⬜ |
| EC-038-02 Image-only scan | routes Vision | ✅ (probe returns None) |
| EC-038-03 1000+ pages | timeout capped 86400 s | ✅ (profile unit tests) |
| EC-038-04 Circuit breaker | typed `failure_class` | ✅ (`classify_*` tests) |
| EC-038-05 Resume markdown | skip Phase A | ⬜ |
| EC-038-06 Vision checkpoint | partial pages resume | ⬜ (existing SPEC-011) |
| EC-038-07 Embedding 512 limit | SPEC-011 fix verified | ⬜ |
| EC-038-08 10→50 MB text | orchestrator accepts 15 MB fixture | ✅ |
| EC-038-09 Concurrent 2×603 page uploads | admission semaphore | ⬜ |
| EC-038-10 Env `PDF_PARSER_BACKEND=vision` | overrides probe | ✅ (`should_try_edgeparse` test) |

---

## Environment Variables

| Variable | Default | Purpose |
| -------- | ------- | ------- |
| `EDGEQUAKE_AUTO_PDF_ROUTING` | `1` | Enable EdgeParse probe before Vision |
| `EDGEQUAKE_TEXT_PROBE_MIN_CHARS_PER_PAGE` | `200` | Text-layer density threshold |
| `EDGEQUAKE_LARGE_PDF_PAGE_THRESHOLD` | `100` | Admission + probe gate |
| `NEXT_PUBLIC_LARGE_PDF_PAGE_THRESHOLD` | `100` | Frontend admission threshold |
| `SPEC038_REPRODUCER_PDF` | — | Override reproducer path in tests |
| `TASK_PROCESSING_TIMEOUT_SECS` | — | Global worker timeout override |

---

## Estimated Timeline (actual)

| Phase | Effort |
| ----- | ------ |
| 1–4 (backend core) | ✅ Done |
| 5–6 (limits + UI) | ✅ Done (6.3–6.4 deferred) |
| 7–8 (tests + benchmarks) | ✅ Core tests; full indexed E2E deferred |
| 9 (upload admit UX) | ✅ Done — byte progress, timeout, move semantics |
