# Requirements — SPEC-033 Page Lineage

## Functional Requirements

### FR-001 — Page field in Chunk KV Response
The `/api/v1/chunks/:id` endpoint (GET chunk detail) MUST return
`page_start?: number` and `page_end?: number` when the chunk was produced
from a PDF with page markers.

**Acceptance**: `GET /api/v1/chunks/chunk-abc` returns
`{ ..., "page_start": 3, "page_end": 3 }`.

---

### FR-002 — Page field in Full Lineage Response
The `/api/v1/documents/:id/lineage` endpoint MUST include `page_start` and
`page_end` for each chunk in the `lineage.chunks[]` array.

**Acceptance**: Chunk items inside the lineage response carry
`"page_start": 3` when the source was a PDF with page markers.

---

### FR-003 — Page-Grouped Data Hierarchy Tree
When at least one chunk in a document has a `page_start` value, the
`DocumentHierarchyTree` MUST render a two-level hierarchy:

```
Document (23 chunks · 543 entities)
├── Page 1 (2 chunks · 14 entities)
│   ├── Chunk 0  [L1-2 · 14 ent]  →  deeplink
│   └── Chunk 1  [L3-8 · 6 ent]   →  deeplink
├── Page 2 (1 chunk · 8 entities)
│   └── Chunk 2  [L9-15 · 8 ent]  →  deeplink
...
```

When no chunk has a `page_start` value (non-PDF or pre-SPEC-032 document),
the tree MUST fall back to the current flat layout.

---

### FR-004 — Chunk Node Badge Shows Page
Each chunk node in the Data Hierarchy tree MUST display a
`p.N` badge when `page_start` is available.

---

### FR-005 — Chunk Click Navigates PDF to Correct Page
When the user clicks a chunk node in the Data Hierarchy tree and that
chunk has `page_start = N`, the PDF viewer MUST navigate to page N.

The URL MUST be updated to `?chunk=<id>&page=N` via `router.replace`
(no browser-history entry).

---

### FR-006 — Controlled PDF Viewer
`PDFViewer` MUST accept a `currentPage?: number` prop.  When `currentPage`
changes, the viewer MUST navigate to that page.  `initialPage` is preserved
as a one-shot seed when `currentPage` is not provided.

---

### FR-007 — Page Header Deeplink in Data Hierarchy
Each "Page N" group header in the Data Hierarchy tree MUST be a
clickable deeplink that:
1. Navigates the PDF viewer to page N.
2. Updates the URL to `?page=N`.
3. Does NOT select any specific chunk.

---

### FR-008 — Citation Grouping by Page in Query Results
In the query `DocumentsTab`, when chunks from the same document share a
`page_start` value, they MUST be grouped under a `Page N` sub-header within
the document card.

Example:

```
╔════════════════════════════════════════════╗
║ 1  m_renault_espace  25%  20×  ↗           ║
╠════════════════════════════════════════════╣
║ ▸ Page 3                                   ║
║   §1  # motorisation ## full hybrid...  100%║
║   §3  # plaisir de conduite...          81%║
║ ▸ Page 7                                   ║
║   §2  # voyagez dans l'Espace...         92%║
╚════════════════════════════════════════════╝
```

Chunks with no `page_start` (non-PDF) MUST continue to render flat (no
page header), matching the current behaviour.

---

### FR-009 — Page Badge Deeplink in Query Citations
Each passage in the citation list that has a `page_start = N` MUST render
a `p.N` badge that is a clickable `<Link>` navigating to:

```
/documents/{document_id}?chunk={chunk_id}&page={page_start}
```

This deeplink MUST open the document viewer with:
- PDF viewer on page N.
- Chunk `chunk_id` highlighted in the markdown panel.

---

### FR-010 — Non-PDF / Missing Page Graceful Fallback
All UI components MUST gracefully handle `page_start === undefined`.
No page grouping or badge is rendered; layout is identical to today.

---

### FR-011 — Cross-surface URL Schema Consistency
All surfaces generating page deeplinks MUST use the canonical URL schema:

```
/documents/{documentId}?chunk={chunkId}&page={pageN}
```

No surface may use `#page=N`, a different param name, or a different path.

---

## Non-Functional Requirements

### NFR-001 — No Additional API Calls
Page data MUST be included in existing API responses (FR-001, FR-002).
The UI MUST NOT make extra HTTP requests to determine page numbers.

### NFR-002 — Backward Compatibility
All API and UI changes MUST be additive.  Documents processed before
SPEC-033 (no `page_start` in KV) MUST continue to render correctly with
the flat (non-page-grouped) layout.

### NFR-003 — PDF Viewer Navigation < 100 ms
Navigating the PDF viewer to a new page via `currentPage` MUST not
re-fetch the PDF document.  Only the rendered page changes.

### NFR-004 — Accessibility
Page group headers (`Page N`) MUST have `role="button"` and
`aria-label="Go to page N"` for keyboard accessibility.
Chunk node page badges MUST have `aria-label="Page N"`.

### NFR-005 — i18n
Any new string literals MUST use the i18n translation hook
(`useTranslation`) with a `documents.hierarchy.*` namespace key and an
English default.

### NFR-006 — TypeScript Strict
All new TypeScript must pass `tsc --strict` with no `any` types.

### NFR-007 — Rust Clippy Clean
All new Rust code must pass `cargo clippy --all-targets -- -D warnings`.
