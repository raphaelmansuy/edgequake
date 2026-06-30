# 5-WHY Analysis + First Principles — SPEC-033 Page Lineage

## 5-WHY Analysis

> **Problem statement**: Users cannot navigate from a retrieved passage or
> hierarchy chunk node directly to the correct PDF page.

---

### WHY 1 — Why can't users navigate to the correct PDF page?

**Answer**: The Data Hierarchy tree renders chunk nodes with no page badge and no
click-to-navigate behaviour. The PDF viewer initialises at page 1 and never
reacts to later user interaction with the tree.

**Code evidence**:

```tsx
// document-hierarchy-tree.tsx — ChunkTreeNode renders index but not page
<TreeNode label={`Chunk ${chunk.chunk_index}`} ...>
```

```tsx
// page.tsx — PDFViewer only gets initialPage; no way to update it later
<PDFViewer file={...} initialPage={initialPdfPage} />
```

---

### WHY 2 — Why does the tree not display page numbers?

**Answer**: The `FullLineageChunk` interface and the `/documents/:id/lineage`
endpoint response do not include `page_start` / `page_end`.  The KV record written
by `chunk_kv_value()` **does** store `page_start`, but the lineage endpoint reads
chunk records through a different code-path that omits these fields.

**Code evidence**:

```rust
// chunk_storage.rs — page_start IS written to KV
if let Some(page) = chunk.page_start {
    value["page_start"] = json!(page);
    value["page_end"] = json!(chunk.page_end.unwrap_or(page));
}
```

```ts
// lineage.ts — FullLineageChunk interface has no page fields
interface FullLineageChunk {
  chunk_id: string;
  chunk_index: number;
  start_line?: number;
  end_line?: number;
  entity_ids?: string[];
}
```

---

### WHY 3 — Why doesn't `ChunkDetailResponse` include page numbers?

**Answer**: `ChunkDetailResponse` (defined in `lineage_types/chunk.rs`) reads
`start_line`, `end_line`, `start_offset`, `end_offset` from the KV record but
does not read `page_start` / `page_end`.  This was an omission when the struct
was designed — the field exists in storage but was not lifted into the DTO.

**Code evidence**:

```rust
// chunk_detail.rs — reads line numbers but not page numbers
let start_line = chunk_data.get("start_line")...;
let end_line   = chunk_data.get("end_line")...;
// page_start is present in chunk_data but never read here
```

---

### WHY 4 — Why doesn't the PDF viewer react to chunk selection changes?

**Answer**: `PDFViewer` is an *uncontrolled* component.  `initialPage` is a one-
shot seed; once the component mounts, `pageNumber` state is internal.  There is no
`currentPage` prop or imperative `goToPage` callback, so parent components cannot
push a new page after the viewer is already mounted.

**Code evidence**:

```tsx
// pdf-viewer.tsx
const [pageNumber, setPageNumber] = useState<number>(initialPage);
// No useEffect watching for initialPage changes from the parent
```

---

### WHY 5 — Why are query citations not grouped by page?

**Answer**: `source-citations.tsx` already receives `page_start` on each chunk
and renders a `p.N` badge, but the rendering loop iterates **flat chunks** and
never groups them by page.  The grouping data is present but the view never
exploits it.

**Code evidence**:

```tsx
// source-citations.tsx — flat iteration, no grouping
{visibleChunks.map((chunk, chunkIdx) => (
  <button key={chunk.chunk_id ?? chunkIdx} ...>
    ...
    {chunk.page_start !== undefined ? (
      <span>p.{chunk.page_start}</span>
    ) : null}
  </button>
))}
```

---

## First Principles Analysis

### Principle 1 — Single Source of Truth for Page Attribution

Page numbers are assigned **once**, at chunk-creation time, by
`PageAwareChunking`.  They are written to KV (`chunk_kv_value`) and vector
metadata (`build_chunk_vector_metadata`).  Every downstream consumer (API, UI)
**must read them from storage** rather than deriving them independently.

> Implication: We must read `page_start`/`page_end` from the KV record in every
> endpoint that returns chunk metadata (`/chunks/:id`, `/documents/:id/lineage`,
> `/documents/:id/full-lineage`).

### Principle 2 — No Chunk Spans Two Pages

`PageAwareChunking` guarantees `page_start == page_end` for every chunk.
This makes page attribution trivially a scalar integer once available.
The data model must treat this as `Optional<u32>` (absent for non-PDF).

> Implication: UI logic can show "Page N" when `page_start` is present and
> omit it entirely for non-PDF documents — no special-casing needed.

### Principle 3 — URL is the Source of Truth for Navigation State

The document detail page already persists chunk selection in the URL
(`?chunk=<id>`).  Page navigation must follow the same pattern: a page parameter
(`?page=N`) drives the PDF viewer, so the view is fully shareable and
bookmark-able.

> Implication: When a user clicks "Chunk 5 on page 7" in the hierarchy tree, the
> handler calls `router.replace(url?chunk=<id>&page=7)`.  The `PDFViewer` reads
> `pageFromUrl` and navigates.

### Principle 4 — PDF Viewer must be Controlled

An uncontrolled viewer that only reads `initialPage` cannot be driven by an
external event (chunk click) once mounted.  The viewer must accept a
`currentPage` prop and expose an `onPageChange` callback — the classic React
controlled-component pattern.

> Implication: `PDFViewer` gets a new `currentPage?: number` prop.  When set,
> the component syncs `pageNumber` state via `useEffect`.  `initialPage` is
> preserved as the default when `currentPage` is absent.

### Principle 5 — Hierarchy before Alphabet

When page attribution is present, chunking follows **page order** — not
alphabetical or arbitrary order.  The UI hierarchy MUST mirror the physical
document structure: Page → Chunk → Entity.  Flattening this to
Document → Chunk → Entity hides provenance.

> Implication: The `DocumentHierarchyTree` has two rendering modes:
> - **Page-grouped** (when `page_start` is present on ≥ 1 chunk): `Page N → [chunks]`
> - **Flat** (non-PDF / no page markers): `[chunks]` as today.

### Principle 6 — Deeplink is the Primitive, Not a Feature

A deeplink to "document D, page P, chunk C" must be a first-class URL that:
- Works when shared across users in the same workspace.
- Survives page refresh.
- Is generated consistently from any citation surface.

The URL schema is already defined:
```
/documents/{documentId}?chunk={chunkId}&page={pageN}
```

> Implication: Every surface (hierarchy tree, query citations, entity provenance)
> must generate the **same** URL format.  No surface may use a different schema.
