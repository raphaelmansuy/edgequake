# OODA-44: Act - Deprecation Complete

## Date: 2026-02-05

## Summary

Successfully deprecated `ExtractionEngine` (lopdf backend) with migration guidance.

---

## Changes Made

### 1. Added `#[deprecated]` to ExtractionEngine

File: `backend/extraction_engine.rs`

```rust
#[deprecated(
    since = "0.2.0",
    note = "Use PdfiumBackend instead for accurate font style detection. ..."
)]
pub struct ExtractionEngine { ... }
```

### 2. Updated Module Documentation

File: `backend/mod.rs`

Added comprehensive ASCII diagram showing:

- ✅ RECOMMENDED: PdfiumBackend
- ⚠️ DEPRECATED: ExtractionEngine
- 🧪 TESTING: MockBackend

### 3. Suppressed Internal Warnings

Files updated with `#![allow(deprecated)]`:

- `backend/extraction_engine.rs` - Module itself
- `extractor.rs` - Fallback code
- `bin/debug_merge.rs` - Debug tool
- `bin/trace_page1.rs` - Debug tool
- `backend/mod.rs` - Re-export

---

## Test Results

```
test result: ok. 445 passed; 0 failed; 0 ignored
```

All tests pass. No deprecation warnings in normal builds.

---

## User Impact

When users **directly use** `ExtractionEngine`:

```
warning: use of deprecated struct `ExtractionEngine`
  = note: Use PdfiumBackend instead for accurate font style detection
```

When users **use default API** (PdfExtractor):

- No warnings (pdfium is preferred automatically)

---

## Removal Timeline

- v0.2.0: Deprecated (current)
- v0.3.0: Planned removal of ExtractionEngine
- Migration: Use PdfiumBackend via default features

---

## OODA-44 Complete ✅
