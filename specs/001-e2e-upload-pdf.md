# Mission: E2E PDF Upload and Processing Fix

## ⚠️ CRITICAL SAFETY MANDATE ⚠️

**YOU MUST RE-READ THIS ENTIRE MISSION FILE AT THE START OF EVERY OODA ITERATION.**

Failure to re-read causes alignment drift → catastrophic safety issues → user frustration → system unreliability.

FULLY READ THIS FILE EACH ITERATION TO ENSURE MISSION ALIGNMENT.

## Task

Your mission is to test and make the upload and the processing of PDF fully works in edgequake. Today we are facing this issue: when you upload `zz_test_docs/lighrag_2410.05779v3.pdf` using documents page, you can upload the document but when you go to documents for this uploaded page you only see the PDF but not the markdown side by side → it seems the content is not extracted, no error → just a blank page with no markdown content. The extraction of KG and embedding don't work. Tested with TenantATest Default workspace. Ensure it works or fail with an explicit error display.

Test using mcp playwright e2e tests and manual testing. Don't takes screenshots as it bloat your session. Repeat until your proven fix works.

Find the best way to package pdfium dynamic library for macOS, Linux and Windows so that the PDF extraction works out of the box for developers using the monorepo and docker setup.

## Context

- **Location**: EdgeQuake monorepo
- **Test Document**: `zz_test_docs/lighrag_2410.05779v3.pdf`
- **Workspace**: TenantATest / Default Workspace
- **Frontend**: Next.js app at `edgequake_webui/`
- **Backend**: Rust API at `edgequake/crates/edgequake-api/`
- **PDF Crate**: `edgequake/crates/edgequake-pdf/`

## Root Cause Analysis (Iteration 1)

**PRIMARY ROOT CAUSE**: `PDFIUM_DYNAMIC_LIB_PATH` environment variable is NOT set when the backend server starts.

### Evidence Chain

1. `PdfiumExtractor::new()` in `edgequake/crates/edgequake-pdf/src/backend/pdfium.rs:151-174`:
   - Checks `PDFIUM_DYNAMIC_LIB_PATH` env var → NOT SET
   - Checks `/usr/local/lib/libpdfium.dylib` → NOT FOUND
   - Checks `/opt/homebrew/lib/libpdfium.dylib` → NOT FOUND
   - Returns `Err(PdfError::Backend("libpdfium not found..."))`

2. `PdfiumBackend::with_config()` in `pdfium_backend.rs:113`:
   - Calls `PdfiumExtractor::new()` which fails → returns error

3. `PdfExtractor::with_config()` in `extractor.rs:191-203`:
   - Catches the error silently: `Err(e) => { tracing::warn!("..."); Box::new(MockBackend::new()) }`
   - Falls back to `MockBackend`

4. `MockBackend::extract()` returns `Document::new()` → empty document → empty markdown

5. Empty markdown stored in DB → frontend shows blank side-by-side viewer

### Library Location

- `libpdfium.dylib` exists at: `edgequake/crates/edgequake-pdf/lib/lib/libpdfium.dylib`
- Neither `Makefile` nor `.env.example` sets `PDFIUM_DYNAMIC_LIB_PATH`

### Secondary Issues

1. **Silent fallback**: Error is only logged as `warn!`, user sees no error
2. **No error propagation**: Frontend shows blank instead of "PDF extraction failed"
3. **Missing env var in Makefile**: `backend-dev` target doesn't set `PDFIUM_DYNAMIC_LIB_PATH`

---

## Process: OODA Loop (50 iterations minimum)

Execute iterative OODA cycles. Each iteration produces 4 files:

You Must absolutely read your mission every iteration! It is vital to avoid alignment drift. You can forget previous iterations, but never forget your mission.

Mission file: `specs/001-e2e-upload-pdf.md`

You Must always produce the 4 files per iteration, as shown below:

1 - observe.md → Map the territory. Never make assumptions about code structure or function. Always verify against the actual codebase.
2 - orient.md → Analyze your findings and define possible solutions using First Principles as your north star.
3 - decide.md → Prioritize specific changes to be made based on signal value and impact.
4 - act.md → Implement the decided changes with precision, update the documentation, and reference specific file:line numbers and commit SHAs.

```
001-e2e-upload-pdf/ooda/
├── iteration_01/
│   ├── observe.md
│   ├── orient.md
│   ├── decide.md
│   └── act.md
├── iteration_02/
│   └── ...
└── summary.md
```

### Per-Iteration Requirements

| Step        | Output                                                     |
| ----------- | ---------------------------------------------------------- |
| **Observe** | Code analysis, feature inventory, dependency mapping       |
| **Orient**  | Gap analysis, documentation quality assessment             |
| **Decide**  | Specific changes prioritized by signal value               |
| **Act**     | Implementation with commit (`OODA-XX: <decision summary>`) |

### Constraints

1. **Re-read mission** every iteration: mission file `specs/001-e2e-upload-pdf.md`
2. **Continue** from existing iterations—never restart
3. **Reference** real code: file paths, line numbers, commit SHAs
4. **Use ASCII diagrams** for architecture/flow visualization
5. **Split large files** for maintainability, Use Single Responsibility Principle (SRP)
6. **Optimize** Rust build speed (latest toolchain)
7. **Document amendments** in WHY, high signal value, and precise terms in comments in the codebase.
8. **You must perform tests** and deliver evidence that all tests are passing after your changes.
