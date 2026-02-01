# OODA-81: Observe

**Date**: 2026-02-01
**Mission Re-read**: ✅ Read ./specs/002-unify-ingestion-pipeline.md

## Observation Focus

New requirements added to mission:

1. Document uniqueness hash at workspace level (not global)
2. PDF + Markdown dual view in document detail
3. SRP/DRY compliance in ingestion pipeline
4. E2E tests with Playwright MCP (no screenshots)

---

## Finding 1: Content Hash Uniqueness is GLOBAL (Bug)

### Current State

**Database Schema** (`docker/init.sql:528-529`):

```sql
CREATE UNIQUE INDEX IF NOT EXISTS idx_documents_content_hash_unique
    ON documents(content_hash) WHERE content_hash IS NOT NULL AND status = 'indexed';
```

**Problem**: Index is on `content_hash` alone - does NOT include `workspace_id`.

**Handler Check** (`documents.rs:2318-2342`):

```rust
let hash_key = format!("doc:hash:{}", content_hash);  // ❌ NO workspace_id!
if let Some(existing_doc_id) = state.kv_storage.get_by_id(&hash_key).await? {
    // Returns duplicate...
}
```

**Impact**: Same document uploaded to different workspaces is rejected globally.

### Required Fix

1. Database: `CREATE UNIQUE INDEX ... ON documents(workspace_id, content_hash)`
2. Handler: `let hash_key = format!("doc:hash:{}:{}", workspace_id, content_hash);`

---

## Finding 2: PDF Viewer Exists but No Side-by-Side View

### Current Components

| File                         | Purpose                | Status     |
| ---------------------------- | ---------------------- | ---------- |
| `pdf-viewer.tsx`             | react-pdf based viewer | ✅ Working |
| `document-detail-dialog.tsx` | Document details modal | ✅ Working |

### Missing

- No side-by-side PDF + Markdown view
- Document detail dialog has tabs (Overview, Content, Entities) but no PDF tab
- For PDF-origin documents, should show original PDF alongside extracted markdown

### Required Changes

1. Add "Source" tab to document-detail-dialog
2. Implement split-pane layout for PDF + Markdown
3. Query API for both PDF blob and markdown content

---

## Finding 3: SRP/DRY Issues in Ingestion Pipeline

### Files to Audit

| File            | Lines | Concern                                 |
| --------------- | ----- | --------------------------------------- |
| `documents.rs`  | 4134  | Too large - multiple responsibilities   |
| `pdf_upload.rs` | ~500  | Duplicates some logic from documents.rs |
| `processor.rs`  | ~800  | Pipeline processing                     |

### Specific DRY Violations

1. **Content hash calculation** - duplicated in:
   - `documents.rs:520-523` (text upload)
   - `documents.rs:2313-2314` (file upload)
   - Potentially in PDF handler

2. **Duplicate check logic** - not shared between handlers

3. **Document metadata building** - repeated JSON structures

---

## Finding 4: E2E Test Infrastructure

### Current State

- Playwright config exists: `playwright.config.ts`
- E2E tests exist: `e2e/document-detail.spec.ts`
- MCP Playwright available in environment

### Required Tests

1. Upload PDF → verify no duplicate error → view PDF+Markdown
2. Upload duplicate in SAME workspace → verify 409 Conflict
3. Upload same file in DIFFERENT workspace → verify success
4. Status progression during ingestion
5. Error recovery scenarios

---

## Summary of Gaps

| Requirement                | Current State          | Gap                   |
| -------------------------- | ---------------------- | --------------------- |
| Workspace-level uniqueness | Global uniqueness      | 🔴 Critical Bug       |
| PDF+Markdown dual view     | PDF viewer only        | 🟡 Feature needed     |
| SRP compliance             | Large monolithic files | 🟡 Refactor needed    |
| DRY compliance             | Duplicated hash logic  | 🟡 Refactor needed    |
| E2E tests                  | Basic tests exist      | 🟡 Need new scenarios |

---

## Next Action

Proceed to **Orient** phase to analyze solutions for workspace-scoped uniqueness.
