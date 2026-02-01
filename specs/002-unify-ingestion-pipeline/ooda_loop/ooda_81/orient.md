# OODA-81: Orient

**Date**: 2026-02-01
**Mission Re-read**: ✅

## Analysis: First Principles Approach

### Principle 1: Multi-tenancy Isolation

Documents belong to workspaces. Uniqueness constraints MUST be scoped to workspace.

**Current violation**: Global content_hash index breaks workspace isolation.

### Principle 2: Single Responsibility

Each component should have ONE reason to change:

- Hash calculation → ContentHasher module
- Duplicate detection → DuplicateChecker service
- Document storage → DocumentRepository
- API handlers → Thin orchestration layer

### Principle 3: DRY (Don't Repeat Yourself)

Shared logic should be extracted:

- Content hashing
- Metadata building
- Status tracking
- Error formatting

---

## Solution Analysis

### Issue 1: Workspace-Scoped Uniqueness

**Option A: Database-only fix**

- Change unique index to include workspace_id
- Pros: Simple, single migration
- Cons: Doesn't fix KV storage key

**Option B: Fix both DB and handler**

- Update index: `ON documents(workspace_id, content_hash)`
- Update handler: `doc:hash:{workspace_id}:{content_hash}`
- Pros: Complete fix
- Cons: Need migration + code change

**Decision**: Option B (complete fix)

```
┌─────────────────────────────────────────────────────────┐
│                 WORKSPACE-SCOPED UNIQUENESS             │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  Workspace A                     Workspace B            │
│  ┌─────────────────┐           ┌─────────────────┐     │
│  │ doc.pdf (hash1) │           │ doc.pdf (hash1) │     │
│  │ ✓ Allowed       │           │ ✓ Also Allowed  │     │
│  └─────────────────┘           └─────────────────┘     │
│                                                         │
│  Workspace A                                            │
│  ┌─────────────────┐                                   │
│  │ doc.pdf (hash1) │ ← Already exists                  │
│  │ ✗ Duplicate     │ ← Same workspace = REJECT         │
│  └─────────────────┘                                   │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### Issue 2: PDF + Markdown Dual View

**Architecture**:

```
┌───────────────────────────────────────────────────────────────┐
│                     DocumentDetailDialog                       │
├───────────────────────────────────────────────────────────────┤
│  Tabs: [Overview] [Content] [Source] [Entities]               │
│                                                                │
│  When source_type === 'pdf':                                  │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │  [Source] Tab                                            │  │
│  │  ┌───────────────────┬───────────────────────────────┐  │  │
│  │  │                   │                               │  │  │
│  │  │   PDF Viewer      │   Markdown Viewer             │  │  │
│  │  │   (react-pdf)     │   (existing renderer)         │  │  │
│  │  │                   │                               │  │  │
│  │  │   ◄ 1/10 ►        │   # Extracted Text            │  │  │
│  │  │   [🔍+] [🔍-]     │   Lorem ipsum...              │  │  │
│  │  │                   │                               │  │  │
│  │  └───────────────────┴───────────────────────────────┘  │  │
│  │        ◄──── Draggable Divider ────►                    │  │
│  └─────────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────────┘
```

**Components needed**:

1. `pdf-markdown-split-view.tsx` - New component with resizable panes
2. Update `document-detail-dialog.tsx` - Add Source tab
3. API endpoint for markdown content of PDF-origin docs

### Issue 3: SRP/DRY Refactoring

**Proposed module extraction**:

```
edgequake-api/src/
├── handlers/
│   ├── documents.rs      → Slim handler layer
│   └── pdf_upload.rs     → PDF-specific upload
├── services/
│   ├── mod.rs
│   ├── content_hasher.rs     ← NEW: SHA-256 hash + workspace key
│   ├── duplicate_checker.rs  ← NEW: Check duplicate in workspace
│   └── metadata_builder.rs   ← NEW: Build document metadata JSON
└── ...
```

---

## Risk Assessment

| Fix                     | Risk Level | Mitigation                   |
| ----------------------- | ---------- | ---------------------------- |
| DB index change         | Low        | Migration with IF NOT EXISTS |
| Handler hash key change | Medium     | Test with existing data      |
| Split view component    | Low        | Additive change              |
| SRP refactoring         | Medium     | Incremental extraction       |

---

## Priority Order

1. **P0**: Fix workspace-scoped uniqueness (breaking bug)
2. **P1**: Add PDF+Markdown split view (feature)
3. **P2**: SRP/DRY refactoring (code quality)
4. **P3**: E2E tests (validation)

---

## Next Action

Proceed to **Decide** phase to define specific implementation steps.
