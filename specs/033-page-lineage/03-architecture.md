# Architecture — SPEC-033 Page Lineage

## 1. End-to-End Data Flow

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│                         INGESTION (already complete)                             │
│                                                                                  │
│  PDF binary                                                                      │
│       │                                                                          │
│       ▼                                                                          │
│  edgequake-pdf  ──►  Markdown with <!-- edgequake-page:N --> markers            │
│       │                                                                          │
│       ▼                                                                          │
│  PageAwareChunking                                                               │
│    ├─ splits at page markers                                                     │
│    └─ stamps ChunkResult { page_start: N, page_end: N }                         │
│             │                                                                    │
│             ▼                                                                    │
│  chunk_kv_value()                                                                │
│    ├─ KV store: { ..., "page_start": N, "page_end": N }   ◄── SSOT             │
│    └─ vector metadata: { ..., "page_start": N }                                 │
└──────────────────────────────────────────────────────────────────────────────────┘
                 │
                 │  KV / Postgres
                 ▼
┌──────────────────────────────────────────────────────────────────────────────────┐
│                         API LAYER (changes in SPEC-033)                          │
│                                                                                  │
│  GET /api/v1/chunks/:id                                                          │
│    ├─ reads KV record                                                            │
│    ├─ reads page_start, page_end  ◄── NEW                                       │
│    └─ returns ChunkDetailResponse { ..., page_start, page_end }                 │
│                                                                                  │
│  GET /api/v1/documents/:id/lineage                                               │
│    ├─ reads KV lineage record (chunks array)                                     │
│    ├─ includes page_start, page_end in each chunk item  ◄── NEW                 │
│    └─ returns DocumentFullLineageResponse { lineage: { chunks: [...] } }        │
│                                                                                  │
│  GET /api/v1/query  (no change)                                                  │
│    └─ SourceReference already has page_start, page_end                          │
└──────────────────────────────────────────────────────────────────────────────────┘
                 │
                 │  JSON over HTTP
                 ▼
┌──────────────────────────────────────────────────────────────────────────────────┐
│                         FRONTEND (changes in SPEC-033)                           │
│                                                                                  │
│  DocumentHierarchyTree                                                           │
│    ├─ reads page_start from chunk items                                          │
│    ├─ groups chunks by page (when page_start present)                           │
│    ├─ renders Page N headers with deeplink  ◄── NEW                             │
│    ├─ renders p.N badge on each chunk node   ◄── NEW                            │
│    └─ on chunk click → router.replace(?chunk=id&page=N)  ◄── CHANGED           │
│                                                                                  │
│  PDFViewer                                                                       │
│    ├─ accepts currentPage?: number  ◄── NEW (controlled prop)                   │
│    └─ useEffect syncs pageNumber when currentPage changes                       │
│                                                                                  │
│  Document Detail Page (page.tsx)                                                 │
│    ├─ reads ?page=N from URL (already done)                                     │
│    ├─ passes pageFromUrl as currentPage to PDFViewer  ◄── CHANGED               │
│    └─ handleChunkSelect → pushes chunk.page_start into URL  ◄── CHANGED         │
│                                                                                  │
│  source-citations.tsx (query)                                                    │
│    ├─ groups passages by page_start  ◄── NEW                                    │
│    └─ p.N badge is a <Link> to ?chunk=id&page=N  ◄── ENHANCED                  │
└──────────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Component Dependency Graph

```
page.tsx (Document Detail)
│
├── PDFViewer                      ← currentPage: number | undefined  (NEW)
│     └── react-pdf <Page>         ← pageNumber: state (synced from currentPage)
│
├── SideBySideViewer
│     ├── [left]  PDFViewer
│     └── [right] ContentRenderer
│
└── MetadataSidebar
      └── DocumentHierarchyTree    ← onChunkSelect(chunkId, start?, end?, page?)  (CHANGED)
            └── PageGroupNode (NEW)
                  └── ChunkTreeNode
                        └── EntityTreeNode
```

---

## 3. Sequence Diagram — Chunk Click to PDF Navigation

```
User                   DocumentHierarchyTree    page.tsx           PDFViewer
 │                            │                    │                    │
 │── click "Chunk 5 p.3" ──► │                    │                    │
 │                            │                    │                    │
 │                            │─ onChunkSelect ──► │                    │
 │                            │  (id, start, end,  │                    │
 │                            │   page=3)          │                    │
 │                            │                    │                    │
 │                            │                    │─ router.replace ──►│
 │                            │                    │  ?chunk=id&page=3  │
 │                            │                    │                    │
 │                            │                    │  setCurrentPage(3)─►│
 │                            │                    │                    │─ setPageNumber(3)
 │                            │                    │                    │─ re-render Page 3
 │                            │                    │                    │
 │◄─────────── PDF on page 3 shown ───────────────────────────────────── │
```

---

## 4. Sequence Diagram — Citation Click from Query Results

```
User                   source-citations.tsx      router               page.tsx
 │                            │                    │                    │
 │── click "p.3" badge ──────►│                    │                    │
 │                            │── <Link href=      │                    │
 │                            │   /docs/D?chunk=C  │                    │
 │                            │   &page=3> ───────►│                    │
 │                            │                    │── navigate ───────►│
 │                            │                    │                    │─ pageFromUrl=3
 │                            │                    │                    │─ chunkIdFromUrl=C
 │                            │                    │                    │─ PDFViewer(currentPage=3)
 │                            │                    │                    │─ ContentRenderer(highlight C)
 │◄──────────────── Document at page 3 with chunk highlighted ──────────│
```

---

## 5. Storage Layer — No Changes

The page attribution data is already correctly stored in:

- **KV storage**: `page_start` / `page_end` keys in chunk JSON record
- **Vector storage**: `page_start` in vector metadata

The only storage-layer code change is in the **API read path**: reading and
forwarding these existing keys in endpoints that currently omit them.

---

## 6. Module Responsibility Matrix (SOLID)

| Module                  | Responsibility                               | SOLID Principle                 |
| ----------------------- | -------------------------------------------- | ------------------------------- |
| `PageAwareChunking`     | Stamp `page_start`/`page_end` on each chunk  | SRP — chunking only             |
| `chunk_kv_value()`      | Persist page fields to KV                    | SRP — serialisation only        |
| `ChunkDetailResponse`   | Surface page fields in REST DTO              | OCP — extend, not modify        |
| `DocumentHierarchyTree` | Group chunks by page (pure transform)        | SRP — display only              |
| `PDFViewer`             | Navigate to `currentPage` when it changes    | OCP — new prop, backward compat |
| `page.tsx`              | Coordinate URL state ↔ child component props | Mediator (DIP)                  |
| `source-citations.tsx`  | Group passages by page                       | SRP — citation display          |
| URL schema (`?page=N`)  | Canonical navigation state                   | DRY — single definition         |

---

## 7. Edge Cases

| Case                                                      | Behaviour                                                                                    |
| --------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| Non-PDF document (no page markers)                        | No page grouping, no page badges, flat layout                                                |
| PDF ingested before SPEC-032 W-09 (no `page_start` in KV) | Same as non-PDF — flat layout                                                                |
| `page_start === 0`                                        | Treated as "no page" — rendered in ungrouped bucket                                          |
| Multiple chunks on the same page                          | All rendered under same "Page N" header                                                      |
| Single-page PDF (all chunks `page_start = 1`)             | Shows one "Page 1" group header                                                              |
| User navigates PDF manually (toolbar prev/next)           | Internal `pageNumber` state updates; URL NOT rewritten (uncontrolled navigation stays local) |
| `currentPage` prop exceeds `numPages`                     | Clamped to `Math.min(numPages, currentPage)`                                                 |
| `currentPage` changes before PDF loads                    | Queued; applied after `onLoadSuccess` sets `numPages`                                        |
