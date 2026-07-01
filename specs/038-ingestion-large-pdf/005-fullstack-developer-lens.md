# SPEC-038 — Full Stack Developer Lens

**Lens:** Full Stack Developer  
**Method:** DRY + SRP; minimal diff; code is law  
**Deploy order:** Backend SSOT → worker timeout → upload routing → UI → tests

---

## Architecture Overview

```text
  HTTP Upload                    Worker                         Pipeline
  ───────────                    ──────                         ────────

  pdf_upload/upload.rs           pdf_processing.rs              text_insert/
       │                              │                         prepare.rs
       ▼                              ▼                              │
  extract_page_count()          LargeDocumentProfile                  │
       │                         (NEW — SSOT)                        │
       ▼                              │                              ▼
  probe_text_layer() ─────────► PdfRoutingPolicy              build_ingestion_pipeline()
  (NEW)                              │                              │
       │                              ▼                              ▼
  IngestionEstimate DTO         compute_safe_pdf_resource_profile   Pipeline::process
  (NEW)                       (EXTEND existing)                   (existing)
       │                              │
       ▼                              ▼
  Response 202                  scaled task timeout
                                (NEW hook in worker)
```

---

## New Module: `large_document_profile.rs` (SRP)

**Location:** `edgequake-api/src/services/large_document_profile.rs`

```rust
/// SSOT for large-PDF admission, timeout, concurrency, and UX estimates.
pub struct LargeDocumentProfile {
    pub page_count: usize,
    pub file_size_bytes: u64,
    pub text_char_count: usize,      // from probe
    pub text_chars_per_page: f64,
    pub has_text_layer: bool,
    pub recommended_backend: PdfParserBackend,
}

impl LargeDocumentProfile {
    pub fn from_pdf_bytes(pdf: &[u8], page_count: usize) -> Self { ... }

    pub fn task_timeout_secs(&self, backend: PdfParserBackend, provider: &str) -> u64 { ... }

    pub fn ingestion_estimate(&self, backend: PdfParserBackend, provider: &str) -> IngestionEstimate { ... }
}
```

**DRY rule:** `pdf_processing.rs`, `upload.rs`, and UI DTOs **must not** duplicate timeout math.  
They call `profile.task_timeout_secs()` and `vision_outer_timeout_secs()` only through this module.

---

## Text Layer Probe (REQ-038-01)

**Location:** `edgequake-api/src/services/pdf_text_probe.rs` (or inside `edgequake-pdf`)

```rust
/// Fast O(pages) sample: extract text from first + middle + last page via pdfium.
/// Full probe optional for page_count < 200.
pub fn probe_text_layer(pdf_bytes: &[u8], page_count: usize) -> TextProbeResult {
    // text_char_count, has_text_layer, confidence
}
```

**Routing policy:**

```rust
pub fn resolve_backend(
    profile: &LargeDocumentProfile,
    user_override: Option<PdfParserBackend>,
    workspace_default: Option<PdfParserBackend>,
) -> PdfParserBackend {
    if let Some(b) = user_override { return b; }
    if profile.has_text_layer && profile.text_chars_per_page >= MIN_CHARS_PER_PAGE {
        return PdfParserBackend::EdgeParse;
    }
    workspace_default.or_else(PdfParserBackend::from_env).unwrap_or(PdfParserBackend::Vision)
}
```

**Constants (env-tunable):**

| Env | Default | Purpose |
| --- | ------- | ------- |
| `EDGEQUAKE_TEXT_PROBE_MIN_CHARS_PER_PAGE` | `200` | Born-digital threshold |
| `EDGEQUAKE_LARGE_PDF_PAGE_THRESHOLD` | `100` | Show admission UX |

---

## Scaled Worker Timeout (REQ-038-03)

**Problem:** Fixed `7200 s` in `WorkerPoolConfig`.

**Fix:** Per-task timeout override on `Task` metadata:

```rust
// edgequake-tasks/src/types/task.rs — add optional field
pub processing_timeout_override_secs: Option<u64>,
```

Set at PDF enqueue time:

```rust
// upload.rs — after profile computed
task.processing_timeout_override_secs = Some(profile.task_timeout_secs(backend, provider));
```

**Formula (SSOT in `LargeDocumentProfile`):**

```text
T_convert = match backend {
    EdgeParse => 60 + pages × 0.5s,
    Vision    => vision_outer_timeout_secs(provider, pages),
}
T_extract = ceil(chunks / max_concurrent) × median_chunk_secs
T_buffer  = 600  // embed + merge headroom
T_task    = min(T_convert + T_extract + T_buffer, 86400)
T_task    = max(T_task, 7200)  // floor for small docs
```

For reproducer (603 pages, EdgeParse, mock):

```text
T_convert ≈ 60 + 302 ≈ 362 s
T_extract ≈ ceil(603/16) × 25 ≈ 944 s
T_task ≈ 1906 s ≈ 32 min  (well under old cap)
```

For reproducer (603 pages, Vision, cloud):

```text
T_convert ≈ 4944 s
T_extract ≈ 944 s
T_task ≈ 6488 s + merge → set cap 10800 s (3 h) not 7200
```

---

## Extend Existing Functions (Don't Duplicate)

| Existing | Extension |
| -------- | --------- |
| `compute_safe_pdf_resource_profile()` | Accept `LargeDocumentProfile` ref |
| `vision_outer_timeout_secs()` | Called only from profile SSOT |
| `should_resume_pdf_conversion()` | Unchanged |
| `resolve_backend()` in upload types | Delegate to `PdfRoutingPolicy` |

---

## API Changes

### Upload response — add estimate

```json
{
  "pdf_id": "...",
  "page_count": 603,
  "recommended_backend": "edgeparse",
  "ingestion_estimate": {
    "total_seconds_pessimistic": 1906,
    "convert_seconds": 362,
    "extract_seconds": 944,
    "backend": "edgeparse"
  }
}
```

### Document metadata — add failure class

```json
{
  "status": "failed",
  "failure_class": "timeout_phase_convert",
  "failure_phase": "converting",
  "recommended_action": "reprocess_edgeparse"
}
```

OpenAPI: extend `PdfUploadResponse`, `DocumentMetadata` schemas.

---

## Frontend Changes

| File | Change |
| ---- | ------ |
| Upload component | Render admission card when `page_count ≥ threshold` |
| Document row | Parse `stage_message` counters; show ETA |
| Failed banner | Map `failure_class` → copy + CTA |
| Settings | Parser override already exists via workspace — surface in upload |

**Client-side page count:** Use `pdfjs-dist` `getDocument().numPages` on file select for instant preview (no upload required).

---

## Align Size Limits (REQ-038-08)

| Location | Current | Target |
| -------- | ------- | ------ |
| `orchestrator/ingestion.rs` | 10 MB | `MAX_UPLOAD_BYTES` (50 MB) or `ResourceBudget` SSOT |
| `injection_file.rs` | 10 MB | Same SSOT import |

```rust
// edgequake-core/src/resource/budget.rs — already SSOT
pub const MAX_UPLOAD_BYTES: usize = 50 * 1024 * 1024;
```

---

## SRP Module Map

| Module | Responsibility |
| ------ | -------------- |
| `pdf_text_probe.rs` | Measure text layer |
| `large_document_profile.rs` | Profile + timeout + estimate |
| `pdf_routing_policy.rs` | Backend resolution |
| `pdf_processing.rs` | Orchestrate phases (thin) |
| `ingest_admission.rs` | Document identity (unchanged) |
| `pipeline_progress_callback.rs` | Progress events (extend failure_class) |

---

## Test Plan (Real Test Is Law)

### Unit tests

| Test | File | Asserts |
| ---- | ---- | ------- |
| `probe_detects_born_digital` | `pdf_text_probe.rs` | reproducer bytes → `has_text_layer=true` |
| `routing_prefers_edgeparse` | `pdf_routing_policy.rs` | profile → EdgeParse |
| `timeout_scales_with_pages` | `large_document_profile.rs` | 603 vision > 7200 |
| `timeout_edgeparse_under_cap` | same | 603 edgeparse < 7200 |

### Integration tests (postgres feature)

| Test | File | Asserts |
| ---- | ---- | ------- |
| `spec038_reproducer_edgeparse_e2e` | `tests/spec038_large_pdf.rs` | Full ingest mock LLM → indexed |
| `spec038_resume_after_markdown` | same | Skip vision on retry |
| `spec038_failure_class_timeout` | same | Typed failure in metadata |

**Gold fixture:**

```bash
# Copy reproducer to test fixtures (git-lfs or CI cache)
cp guide_2606.24937v1-opt.pdf edgequake/crates/edgequake-api/tests/fixtures/spec038/
```

### Playwright E2E

| Spec | Asserts |
| ---- | ------- |
| `e2e/spec038-large-pdf-admission.spec.ts` | Admission card for 603-page fixture |
| `e2e/spec038-large-pdf-progress.spec.ts` | Mock WS progress shows N/603 |

---

## Edge Cases (Developer)

| EC | Handling |
| -- | -------- |
| `page_count=0` at upload | Probe uses pdfium; re-read after parse |
| Encrypted PDF | Fail fast at probe with `failure_class=encrypted` |
| Text probe false negative | User override Vision; log `routing_audit` |
| Text probe false positive (garbled text) | Quality check: entropy / printable ratio; fallback Vision |
| Partial Vision then timeout | Markdown checkpoint + resume path |
| `EDGEQUAKE_PDF_PARSER_BACKEND=vision` env | Env wins over auto-route (explicit ops choice) |

---

## Migration / Rollout

1. **Phase 1:** SSOT + routing (default EdgeParse for born-digital) — immediate reproducer fix  
2. **Phase 2:** Scaled timeout — prevents Phase B kill  
3. **Phase 3:** UI admission card — informed consent  
4. **Phase 4:** failure_class — supportability  

Feature flag (optional): `EDGEQUAKE_AUTO_PDF_ROUTING=1` (default on).

---

## Files Touched (Estimated)

| Crate | Files | LOC est. |
| ----- | ----- | -------- |
| edgequake-api | 6 new/modified | ~400 |
| edgequake-tasks | task.rs, worker.rs | ~80 |
| edgequake-pdf | text probe helper | ~120 |
| edgequake-core | ingestion.rs limit align | ~10 |
| edgequake_webui | upload + document row | ~200 |
| tests | spec038 + e2e | ~300 |

**Total:** ~1100 LOC — focused, not a rewrite.
