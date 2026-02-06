# OODA Iteration 1 – Orient

## Date: 2026-02-06

## Analysis: Why PDF Extraction Silently Fails

### First Principles Decomposition

1. **PDF extraction REQUIRES libpdfium at runtime** — it's a native shared library
2. **The library search path is not configured** — 3 locations checked, none have it
3. **The fallback is SILENT** — `warn!` log only, no error surfaced to user
4. **The project already bundles the library** — at `edgequake/crates/edgequake-pdf/lib/lib/libpdfium.dylib`

### Root Cause Tree

```
No Markdown Content in Viewer
├── markdown_content is empty string or null
│   ├── MockBackend returns empty Document
│   │   ├── PdfiumBackend fails to initialize
│   │   │   ├── PDFIUM_DYNAMIC_LIB_PATH not set ← ROOT CAUSE #1
│   │   │   └── libpdfium not in system lib paths ← ROOT CAUSE #2
│   │   └── Silent fallback to MockBackend ← DESIGN FLAW #1
│   └── No error propagation to user ← DESIGN FLAW #2
└── Frontend shows PDF-only (no side-by-side)
    └── hasMarkdown = false for empty string ← CORRECT BEHAVIOR (symptom)
```

### Solution Options

| #   | Solution                                                                  | Effort | Impact | Risk                            |
| --- | ------------------------------------------------------------------------- | ------ | ------ | ------------------------------- |
| A   | Add auto-discovery of bundled libpdfium in PdfiumExtractor                | Low    | HIGH   | Low — additive change           |
| B   | Set PDFIUM_DYNAMIC_LIB_PATH in Makefile                                   | Low    | Medium | Low — env var                   |
| C   | Propagate PdfiumBackend error instead of silent fallback                  | Medium | HIGH   | Medium — changes error handling |
| D   | Add frontend error display when markdown is empty but PDF was "processed" | Medium | HIGH   | Low — UI change                 |
| E   | Copy libpdfium to system path during build                                | Medium | Medium | Medium — system modification    |

### Recommended Approach

**Do A + B + C + D in this iteration:**

1. **A**: Add the project's bundled library path to the PdfiumExtractor search list
2. **B**: Also set `PDFIUM_DYNAMIC_LIB_PATH` in Makefile for explicit configuration
3. **C**: Instead of silently falling back to MockBackend, FAIL if PdfiumBackend can't init (the empty doc from MockBackend is never useful in production)
4. **D**: Frontend should show explicit error when processed PDF has no markdown

### Risk Assessment

- **A** is the safest fix — just adds one more search path
- **B** is a belt-and-suspenders approach
- **C** changes behavior: instead of empty results, errors surface — this is BETTER for UX
- **D** ensures even if C somehow misses an edge case, the user sees an error message
