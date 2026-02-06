# Mission: E2E PDF Upload and Processing Fix

## ⚠️ CRITICAL SAFETY MANDATE ⚠️

**YOU MUST RE-READ THIS ENTIRE MISSION FILE AT THE START OF EVERY OODA ITERATION.**

Failure to re-read causes alignment drift → catastrophic safety issues → user frustration → system unreliability.

FULLY READ THIS FILE EACH ITERATION TO ENSURE MISSION ALIGNMENT.

## Task

Your mission is to test and make the upload and the processing of PDF fully works in edgequake. Today we are facing this issue: when you upload `zz_test_docs/lighrag_2410.05779v3.pdf` using documents page, you can upload the document but when you go to documents for this uploaded page you only see the PDF but not the markdown side by side → it seems the content is not extracted, no error → just a blank page with no markdown content. The extraction of KG and embedding don't work. Tested with TenantATest Default workspace. Ensure it works or fail with an explicit error display.

Test using mcp playwright e2e tests and manual testing. Don't takes screenshots as it bloat your session. Repeat until your proven fix works.

Ensure the Upload PDF → Markdown extraction → Side-by-side viewer pipeline works perfectly for the `lighrag_2410.05779v3.pdf` test document. Ensure re-indexing the same document also works. Document your testing process, findings, and any code changes made to achieve this.
Ensure the data structure to manage documents and uploaded files is well designed and maintainable. Refactor if necessary to improve code quality and readability. SRP AND DRY principles must be followed. Refactor with confidence and ensure all tests pass after your changes.

Find the best way to package pdfium dynamic library for macOS, Linux and Windows so that the PDF extraction works out of the box for developers using the monorepo and docker setup.

### New Requirements (OODA-08+)

1. **Document Re-indexing**: Uploading the same document must trigger re-indexing
2. **OpenAI Provider**: Tests must use OpenAI for model and embedding (not Ollama)
3. **Clean Tenant**: Each E2E test run creates a fresh tenant for isolation
4. **Focused Tests**: All tests must have timeouts (30s for unit, 120s for E2E)
5. **Data Model Solidity**: Review and ensure document/task data structures are well-designed

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

## Iteration Status

### ✅ Iteration 01 (commit b1611b45) - COMPLETE

**Fixes Applied**:

1. Added `PDFIUM_DYNAMIC_LIB_PATH` to Makefile backend-dev target
2. Added auto-discovery code in PdfiumExtractor
3. Improved error logging for library loading
4. Updated .env.example with PDFIUM_DYNAMIC_LIB_PATH
5. Created scripts/download-pdfium.sh for automated setup

**Result**: PDF extraction now works - 60,967 bytes markdown from lighrag paper

**Reference**: `specs/001-e2e-upload-pdf/ooda/iteration_01/`

### ✅ Iteration 02 (commit e7cc8c4c, tag: ooda-iteration-02) - COMPLETE

**Mission Status**: PRIMARY OBJECTIVE COMPLETE ✅

**E2E Verification**:

- Used MCP Playwright to navigate to documents page
- Clicked on processing document (f6fa9cad-bbff-4892-a855-3bd7d70da044)
- Verified side-by-side viewer displays both PDF (left) and markdown (right)
- 16,887 bytes markdown extracted from 16-page lighrag paper
- Content properly structured (headings, lists, links, emphasis)

**Documentation Created**:

- `observe.md`: E2E test findings, system state, evidence chain (259 lines)
- `orient.md`: Root cause analysis, strategic options, decision matrix (241 lines)
- `decide.md`: Action plan, task breakdown, risk assessment (274 lines)
- `act.md`: Implementation log, commit verification, metrics (228 lines)
- Updated `AGENTS.md`: Service management & E2E testing section (+320 lines)

**Key Findings**:

1. PDF extraction working perfectly (iteration 01 fix successful)
2. Failed documents (rows 2-3) were due to Ollama being offline (separate issue)
3. Frontend PID management needs improvement (not blocking)
4. MCP Playwright excellent for AI-driven E2E testing

**Total Changes**: 5 files, 1,492 insertions

**Reference**: `specs/001-e2e-upload-pdf/ooda/iteration_02/`

### ⏳ Iteration 03 - PLANNED

**Focus**: Fix Makefile frontend PID management

**Problem**: Frontend process may die but PID file remains, causing `make stop` to fail silently.

**Solution**:

1. Add health check loop after `bun run dev &`
2. Poll http://localhost:3000 with retry
3. Only write PID if port responds
4. Add timeout and error reporting

**Priority**: Medium (improves automation reliability)

**Effort**: 2-3 hours

### ✅ Iteration 06 - COMPLETE

**Focus**: PostgreSQL Task Storage Implementation

**Problem**: Tasks were stored in memory and lost on backend restart.

**Fixes Applied**:

1. Replaced `MemoryTaskStorage` with `PostgresTaskStorage` in `state.rs` line 793
2. Fixed schema mapping in `postgres.rs`: `task_data/metadata/progress` → `payload` JSONB
3. Updated `tasks_valid_status` constraint to support all status values

**E2E Verification**:

- Uploaded `AI_Services__Elitizon.pdf` (5 pages)
- Task stored in PostgreSQL: `pdf-21f40259-0051-4616-adf9-d23235e57d52`
- Extracted 5,338 bytes markdown
- Created 20 entities, 9 relationships in AGE graph
- Task completed with status `indexed`

**Database State**:
- AGE Graph: 2,801 nodes, 2,219 edges total
- Vector Storage: 149 vectors

**Reference**: `specs/001-e2e-upload-pdf/ooda/iteration_06/`

### ✅ Iteration 07 - COMPLETE

**Focus**: E2E Pipeline Verification

**Objective**: Verify full pipeline works and investigate reported document visibility issue.

**Findings**:

1. "Documents (0)" was a **transient React loading state**, NOT a bug
2. API returns 23 documents correctly
3. Side-by-side viewer fully functional
4. Task persistence verified (OODA-06 fix working)
5. No code changes required

**E2E Verification Results**:

- Frontend: 23 documents visible in list
- PDF viewer: Page navigation, zoom working
- Markdown renderer: Headings, lists, bold all rendered
- Database: 2,801 nodes, 2,219 edges, 149 vectors

**Reference**: `specs/001-e2e-upload-pdf/ooda/iteration_07/`

### 📋 Iterations 08-10 - BACKLOG

| Iteration | Focus                           | Priority | Effort          |
| --------- | ------------------------------- | -------- | --------------- |
| 08        | Fix Ollama timeout (increase from 60s) | Medium | 2-3h      |
| 09        | Fix PDF-document FK race condition | Low   | 2-3h            |
| 10        | Final regression testing        | Low      | 2-3h            |

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
