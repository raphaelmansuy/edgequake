# edgequake-pdf — Observability Audit

**Path:** `edgequake/crates/edgequake-pdf`  
**Tracing macros (src):** ~2  
**Role:** PDF conversion facade (delegates to edgequake-pdf2md)

---

## Executive Summary

**Near-silent crate** — vision/edgeparse backends have 1 tracing call each. PDF failures surface only at API handler level.

---

## Findings

| ID | P | Finding | Evidence | Remediation |
|----|---|---------|----------|-------------|
| PDF-OBS-001 | P2 | No conversion timing | `backend/vision.rs` | `info!(pages, duration_ms, backend)` |
| PDF-OBS-002 | P2 | Errors bubble without local log | Facade pattern | `warn!` before returning `Err` |
| PDF-OBS-003 | P3 | pdf2md opaque | External crate | Span in API `pdf_upload` handler |

---

## Note

Production PDF path often goes through **edgequake-pdf2md** directly in API (`Cargo.toml` edgequake-pdf2md). Observability for PDF must include API `handlers/pdf_upload/*`.

---

## Verify

```bash
rg 'tracing::' edgequake/crates/edgequake-pdf/src
```
