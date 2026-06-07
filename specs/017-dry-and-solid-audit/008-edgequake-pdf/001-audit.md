# edgequake-pdf — DRY & SOLID Audit

**Crate path:** `edgequake/crates/edgequake-pdf`  
**LOC:** ~306 (src)  
**Role:** PDF conversion facade (`PdfConverter` trait, EdgeParse + Vision backends)

---

## Executive Summary

Clean trait + factory pattern for ~300 LOC. Debt is **boundary leakage**: vision config split across factory and call time, fallback orchestration lives in API processor, and `PdfParserBackend` enum pulls pdf dependency into core types. **Crate existence is borderline** — justified only if multi-backend roadmap continues.

---

## DRY Violations

| ID | P | Violation | Evidence | Remediation |
|----|---|-----------|----------|-------------|
| PDF-DRY-001 | **P2** | Dual pdf stack dependency in API | API depends on `edgequake-pdf` AND `edgequake-pdf2md` | Route all through pdf crate |
| PDF-DRY-002 | **P2** | `PdfParserBackend` in core types | `edgequake-core/.../workspace.rs:3` imports pdf crate enum | Move enum to core or shared types |
| PDF-DRY-003 | **P1** | Vision config split across layers | Factory: `create_pdf_converter(backend, llm)`; runtime: `VisionConversionConfig` in `vision.rs:44-51`; API builds both in `pdf_processing.rs:471-516` | Single config object at factory time |

---

## SOLID Violations

| ID | P | Principle | Violation | Evidence |
|----|---|-----------|-----------|----------|
| PDF-SOLID-S-001 | **P1** | SRP | Fallback policy in API, not pdf crate | `processor/pdf_processing.rs:460-538` (~80 LOC vision→EdgeParse) |
| PDF-SOLID-I-001 | **P2** | ISP | `PdfConversionConfig` bundles EdgeParse + vision fields | `backend/mod.rs:74-81` |
| PDF-SOLID-D-001 | **P1** | DIP | Vision needs LLM at factory AND config at convert | `VisionPdfConverter` stores `Option<Arc<dyn LLMProvider>>`; convert fails without nested config |
| PDF-SOLID-O-001 | **P3** | OCP | New backend = edit enum + match | Acceptable at this size |

---

## Backend Abstraction (Sound)

```rust
// backend/mod.rs — clean trait boundary
pub trait PdfConverter: Send + Sync {
    async fn convert(&self, pdf_bytes: &[u8], config: &PdfConversionConfig)
        -> Result<String, PdfConversionError>;
    fn backend_name(&self) -> &'static str;
}
```

**Weakness:** Vision is thin pass-through to `edgequake-pdf2md` with no error taxonomy mapping.

---

## Remediation Plan

| P | Action |
|---|--------|
| **P1** | Move fallback into pdf crate: `create_pdf_converter_with_fallback()` |
| **P1** | Single vision config (provider + model + dpi) passed to factory |
| **P2** | Move `PdfParserBackend` to core; remove direct pdf2md dep from API |
| **P3** | Split config: `EdgeParseConfig` / `VisionConfig` enums |
| **P3** | **Merge candidate** into `edgequake-api` or `edgequake-pipeline` if no new backends planned |

---

## Crate Existence Verdict

| Keep | Merge |
|------|-------|
| 2+ backends strategic | Single consumer, ~306 LOC, abstraction marginal |

---

## Verification

```bash
cargo test -p edgequake-pdf --lib
# After fallback move: API pdf_processing.rs should call factory only
```
