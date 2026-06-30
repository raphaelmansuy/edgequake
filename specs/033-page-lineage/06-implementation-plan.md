# Implementation Plan — SPEC-033 Page Lineage

## DRY / SOLID Checklist

| Principle | Applied Where                                                                                                                        |
| --------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| **SRP**   | Each component does one thing: `PageGroupNode` groups; `PDFViewer` displays; page-deeplink logic is centralised in one URL helper    |
| **OCP**   | `PDFViewer` gains `currentPage` without removing `initialPage`; `ChunkDetailResponse` gains fields without breaking existing callers |
| **LSP**   | `PageGroupNode` is a new node type, not a subtype of `ChunkTreeNode` — no LSP risk                                                   |
| **ISP**   | `onChunkSelect` callback signature extended with optional `page` parameter rather than adding a new callback                         |
| **DIP**   | `page.tsx` owns URL coordination; children receive props, not direct router access                                                   |
| **DRY**   | Deeplink URL construction extracted into a single `buildDocumentPageUrl(docId, chunkId?, page?)` helper used by all surfaces         |

---

## Phase 1 — Backend: Surface Page Fields in API (P1)

> Estimated scope: ~30 lines of Rust. Zero new tables.

### Task B-1: `ChunkDetailResponse` — add `page_start`/`page_end`

**File**: `edgequake/crates/edgequake-api/src/handlers/lineage_types/chunk.rs`

```rust
// ADD after end_line field:
#[serde(skip_serializing_if = "Option::is_none")]
pub page_start: Option<u32>,
#[serde(skip_serializing_if = "Option::is_none")]
pub page_end: Option<u32>,
```

**File**: `edgequake/crates/edgequake-api/src/handlers/lineage/chunk_detail.rs`

```rust
// ADD after end_line read:
let page_start = chunk_data.get("page_start")
    .and_then(|v| v.as_u64()).map(|v| v as u32);
let page_end = chunk_data.get("page_end")
    .and_then(|v| v.as_u64()).map(|v| v as u32);

// ADD to ChunkDetailResponse construction:
page_start,
page_end,
```

**Test**: Extend the existing `get_chunk_detail` contract test to assert
`page_start == Some(3)` when the mock chunk KV record includes `"page_start": 3`.

---

### Task B-2: Full Lineage Response — include page fields in chunk items

**File**: `edgequake/crates/edgequake-api/src/handlers/lineage/queries.rs`
(the section that builds `lineage.chunks` JSON from KV records).

Locate where each chunk KV record is serialised and add:

```rust
if let Some(page) = chunk_kv.get("page_start").and_then(|v| v.as_u64()) {
    chunk_item["page_start"] = json!(page as u32);
    let page_end = chunk_kv.get("page_end")
        .and_then(|v| v.as_u64())
        .unwrap_or(page) as u32;
    chunk_item["page_end"] = json!(page_end);
}
```

**Test**: Extend `test_get_full_document_lineage` to assert chunk items include
`page_start` when the mock data has it.

---

### Phase 1 Acceptance Criteria

- [ ] `GET /api/v1/chunks/chunk-abc` returns `{ ..., "page_start": 3, "page_end": 3 }` for a PDF chunk
- [ ] `GET /api/v1/documents/:id/lineage` returns chunks with `page_start` in `lineage.chunks[]`
- [ ] Non-PDF chunks return neither field (field is absent, not null)
- [ ] `cargo test -p edgequake-api --lib` passes
- [ ] `cargo clippy -p edgequake-api -- -D warnings` passes

---

## Phase 2 — Frontend: TypeScript Types + URL Helper (P2)

> Prerequisite: Phase 1.

### Task F-1: Extend TypeScript types

**File**: `edgequake_webui/src/types/lineage.ts`

```ts
// ChunkDetail — add:
page_start?: number;
page_end?: number;

// ChunkLineage — add:
page_start?: number;
page_end?: number;
```

**File**: `edgequake_webui/src/components/document/document-hierarchy-tree.tsx`

```ts
// FullLineageChunk (local interface) — add:
page_start?: number;
page_end?: number;
```

---

### Task F-2: `buildDocumentPageUrl` helper (DRY)

**New file**: `edgequake_webui/src/lib/utils/document-url.ts`

```ts
/**
 * Build a canonical document viewer URL with optional chunk + page params.
 *
 * @implements SPEC-033 — Single definition of deeplink URL schema.
 * All citation surfaces (hierarchy tree, query citations) MUST use this helper.
 *
 * Schema: /documents/{docId}?chunk={chunkId}&page={page}
 */
export function buildDocumentPageUrl(
  docId: string,
  chunkId?: string,
  page?: number
): string {
  const params = new URLSearchParams();
  if (chunkId) params.set('chunk', chunkId);
  if (page !== undefined && page >= 1) params.set('page', String(page));
  const qs = params.toString();
  return `/documents/${docId}${qs ? `?${qs}` : ''}`;
}
```

**Tests**: `src/lib/utils/__tests__/document-url.test.ts`

```ts
describe('buildDocumentPageUrl', () => {
  it('returns path-only when no params', () =>
    expect(buildDocumentPageUrl('d1')).toBe('/documents/d1'));
  it('includes chunk param', () =>
    expect(buildDocumentPageUrl('d1', 'c1')).toBe('/documents/d1?chunk=c1'));
  it('includes page param', () =>
    expect(buildDocumentPageUrl('d1', undefined, 3)).toBe('/documents/d1?page=3'));
  it('includes both params', () =>
    expect(buildDocumentPageUrl('d1', 'c1', 3)).toBe('/documents/d1?chunk=c1&page=3'));
  it('omits page 0', () =>
    expect(buildDocumentPageUrl('d1', 'c1', 0)).toBe('/documents/d1?chunk=c1'));
});
```

---

### Phase 2 Acceptance Criteria

- [ ] `bun test` passes with new `document-url.test.ts`
- [ ] No `tsc --strict` errors on changed files

---

## Phase 3 — PDF Viewer: Controlled `currentPage` Prop (P3)

> Prerequisite: Phase 2.

### Task F-3: Controlled `currentPage` in `PDFViewer`

**File**: `edgequake_webui/src/components/documents/pdf-viewer.tsx`

1. Add `currentPage?: number` to `PDFViewerProps`.
2. Add sync effect after `useState` calls:

```ts
// WHY: currentPage drives the viewer after initial mount (SPEC-033).
// Exclude pageNumber from deps to prevent feedback loop.
useEffect(() => {
  if (currentPage === undefined) return;
  if (numPages > 0) {
    setPageNumber(Math.max(1, Math.min(numPages, currentPage)));
  } else {
    // PDF not yet loaded: queue the page change; applied after load
    setPageNumber(currentPage);
  }
}, [currentPage, numPages]); // numPages needed to clamp correctly
```

**Edge cases**:
- `currentPage` changes before PDF loads: `numPages = 0`, so the clamp
  uses `currentPage` directly. After load, `numPages` is set, triggering
  the effect again and clamping to a valid page.
- `currentPage` set to same value: `setPageNumber` is a no-op if value
  is identical (React batches identical state updates).

---

### Task F-4: Wire `currentPage` in `page.tsx`

**File**: `edgequake_webui/src/app/(dashboard)/documents/[id]/page.tsx`

Change the `PDFViewer` props from `initialPage` only to also pass
`currentPage`:

```tsx
<PDFViewer
  file={getPdfDownloadUrl(pdfIdForViewer!)}
  initialPage={initialPdfPage}
  currentPage={pageFromUrl}  // controlled — reacts to URL changes
/>
```

The existing `pageFromUrl` is already derived from `searchParams.get('page')`.
This single line addition makes the PDF viewer respond to URL navigation
without any other change.

---

### Task F-5: Update `handleChunkSelect` to include page

**File**: `edgequake_webui/src/app/(dashboard)/documents/[id]/page.tsx`

Current `onChunkSelect` signature: `(chunkId, startLine?, endLine?)`.
New signature: `(chunkId, startLine?, endLine?, page?)`.

```ts
const handleChunkSelect = useCallback(
  (chunkId: string, start?: number, end?: number, page?: number) => {
    // ... existing toggle logic ...

    const params = new URLSearchParams(searchParams.toString());
    if (nextChunkId) {
      params.set('chunk', nextChunkId);
    } else {
      params.delete('chunk');
    }
    // SPEC-033: include page when available
    if (page !== undefined && page >= 1) {
      params.set('page', String(page));
    }
    router.replace(`/documents/${documentId}?${params.toString()}`, { scroll: false });
  },
  [selectedChunkId, searchParams, router, documentId],
);
```

---

### Phase 3 Acceptance Criteria

- [ ] Clicking chunk node in hierarchy tree jumps PDF to chunk's page
- [ ] URL updates to `?chunk=id&page=N` on chunk click
- [ ] PDF viewer toolbar prev/next still works
- [ ] Sharing URL restores PDF on correct page
- [ ] `bun test` passes

---

## Phase 4 — Data Hierarchy: Page Grouping (P4)

> Prerequisite: Phase 3.

### Task F-6: `groupChunksByPage` pure function

**File**: `edgequake_webui/src/components/document/document-hierarchy-tree.tsx`

Add before the component definition:

```ts
/**
 * Group chunks by page number.
 * Returns null when no chunk has page_start (non-PDF fallback).
 *
 * @implements SPEC-033 FR-003 — page-grouped Data Hierarchy
 */
function groupChunksByPage(
  chunks: FullLineageChunk[]
): Map<number, FullLineageChunk[]> | null {
  const hasPages = chunks.some(c => c.page_start !== undefined && c.page_start > 0);
  if (!hasPages) return null;

  const map = new Map<number, FullLineageChunk[]>();
  for (const chunk of chunks) {
    const page = (chunk.page_start && chunk.page_start > 0) ? chunk.page_start : 0;
    const list = map.get(page) ?? [];
    list.push(chunk);
    map.set(page, list);
  }
  return map;
}
```

---

### Task F-7: `PageGroupNode` component

**Same file** — add new component:

```tsx
interface PageGroupNodeProps {
  page: number;
  chunks: FullLineageChunk[];
  entitiesByChunk: Map<string, EntityLineage[]>;
  documentId: string;
  selectedChunkId?: string;
  onSelect?: (chunkId: string, start?: number, end?: number, page?: number) => void;
}

function PageGroupNode({
  page,
  chunks,
  entitiesByChunk,
  documentId,
  selectedChunkId,
  onSelect,
}: PageGroupNodeProps) {
  const [isOpen, setIsOpen] = useState(page <= 3); // first 3 pages open by default
  const entityCount = chunks.reduce(
    (acc, c) => acc + (entitiesByChunk.get(c.chunk_id)?.length ?? 0), 0
  );
  const pageUrl = buildDocumentPageUrl(documentId, undefined, page);

  return (
    <div className="ml-3">
      {/* Page header row */}
      <div
        className="flex items-center gap-1.5 py-0.5 px-1 rounded hover:bg-muted/40
                   cursor-pointer select-none group"
        role="button"
        aria-expanded={isOpen}
        aria-label={`Page ${page}: ${chunks.length} chunks`}
        onClick={() => setIsOpen(o => !o)}
        onKeyDown={e => { if (e.key === 'Enter' || e.key === ' ') setIsOpen(o => !o); }}
        tabIndex={0}
      >
        {isOpen
          ? <ChevronDown className="h-3 w-3 text-muted-foreground" />
          : <ChevronRight className="h-3 w-3 text-muted-foreground" />}
        <Layers className="h-3 w-3 text-primary/60" />
        <span className="text-xs font-medium">Page {page}</span>
        <span className="text-xs text-muted-foreground ml-auto">
          {chunks.length} chunk{chunks.length !== 1 ? 's' : ''} · {entityCount} ent
        </span>
        {/* Page deeplink badge — stops propagation so header click only expands */}
        <Link
          href={pageUrl}
          className="text-[10px] font-medium text-primary hover:underline flex items-center gap-0.5
                     opacity-0 group-hover:opacity-100 transition-opacity focus-visible:opacity-100
                     focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-primary/50
                     rounded-sm px-1"
          aria-label={`Go to page ${page}`}
          title={`Open PDF at page ${page}`}
          onClick={e => e.stopPropagation()}
        >
          p.{page}
          <ExternalLink className="h-2.5 w-2.5" />
        </Link>
      </div>

      {/* Chunk nodes */}
      {isOpen && chunks.map(chunk => (
        <ChunkTreeNode
          key={chunk.chunk_id}
          chunk={chunk}
          entities={entitiesByChunk.get(chunk.chunk_id) ?? []}
          depth={2}
          isSelected={selectedChunkId === chunk.chunk_id}
          onSelect={onSelect}
        />
      ))}
    </div>
  );
}
```

---

### Task F-8: Update `DocumentHierarchyTree` render to use page grouping

In the main render, replace the flat `chunks.map(...)` with:

```tsx
{(() => {
  const grouped = groupChunksByPage(chunks);
  if (grouped) {
    // Page-grouped mode (FR-003)
    return [...grouped.entries()]
      .sort(([a], [b]) => a - b)
      .map(([page, pageChunks]) =>
        page === 0 ? (
          // "No page" bucket — render flat without page header
          pageChunks.map(chunk => (
            <ChunkTreeNode
              key={chunk.chunk_id}
              chunk={chunk}
              entities={entitiesByChunk.get(chunk.chunk_id) ?? []}
              depth={1}
              isSelected={selectedChunkId === chunk.chunk_id}
              onSelect={onChunkSelect}
            />
          ))
        ) : (
          <PageGroupNode
            key={`page-${page}`}
            page={page}
            chunks={pageChunks}
            entitiesByChunk={entitiesByChunk}
            documentId={documentId}
            selectedChunkId={selectedChunkId}
            onSelect={onChunkSelect}
          />
        )
      );
  }
  // Flat mode — non-PDF (FR-010)
  return chunks.map(chunk => (
    <ChunkTreeNode
      key={chunk.chunk_id}
      chunk={chunk}
      entities={entitiesByChunk.get(chunk.chunk_id) ?? []}
      depth={1}
      isSelected={selectedChunkId === chunk.chunk_id}
      onSelect={onChunkSelect}
    />
  ));
})()}
```

---

### Task F-9: `ChunkTreeNode` — add page badge

Extend `ChunkTreeNode` to show `p.N` badge and pass page to `onSelect`:

```tsx
// In ChunkTreeNode, inside the node row:
{chunk.page_start !== undefined && chunk.page_start > 0 && (
  <Link
    href={buildDocumentPageUrl(documentId, chunk.chunk_id, chunk.page_start)}
    className="text-[10px] font-medium text-primary hover:underline
               flex items-center gap-0.5 ml-auto flex-shrink-0
               focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-primary/50"
    aria-label={`Page ${chunk.page_start}`}
    title={`Open PDF at page ${chunk.page_start}`}
    onClick={e => e.stopPropagation()}
  >
    p.{chunk.page_start}
  </Link>
)}
```

In the `onSelect` call:

```tsx
onSelect?.(chunk.chunk_id, chunk.start_line, chunk.end_line, chunk.page_start);
```

---

### Phase 4 Acceptance Criteria

- [ ] PDF document shows "Page N" headers in Data Hierarchy
- [ ] Non-PDF shows flat list (no Page headers)
- [ ] Clicking page header deeplinks to PDF at page N
- [ ] Clicking chunk node jumps PDF to chunk's page
- [ ] p.N badge visible on hover on each chunk node
- [ ] `bun test` passes

---

## Phase 5 — Query Citations: Page-Grouped Passages (P5)

> Prerequisite: Phase 2 (URL helper).

### Task F-10: `groupPassagesByPage` in `source-citations.tsx`

**File**: `edgequake_webui/src/components/query/source-citations.tsx`

```ts
/**
 * Group passages by page_start.
 * Returns null when no passage has page data (non-PDF fallback).
 *
 * @implements SPEC-033 FR-008
 */
function groupPassagesByPage(
  chunks: NonNullable<QueryContext['chunks']>
): Map<number | null, NonNullable<QueryContext['chunks']>> | null {
  const hasPages = chunks.some(c => c.page_start !== undefined);
  if (!hasPages) return null;

  const map = new Map<number | null, NonNullable<QueryContext['chunks']>>();
  for (const chunk of chunks) {
    const key = chunk.page_start ?? null;
    const list = map.get(key) ?? [];
    list.push(chunk);
    map.set(key, list);
  }
  return map;
}
```

---

### Task F-11: `PagePassageGroup` sub-component

```tsx
function PagePassageGroup({
  page,
  passages,
  docId,
  normalizeScore,
  onDocumentClick,
}: {
  page: number | null;
  passages: NonNullable<QueryContext['chunks']>;
  docId: string;
  normalizeScore: (s: number) => number;
  onDocumentClick?: (docId: string, content?: string, idx?: number,
                     start?: number, end?: number, chunkId?: string) => void;
}) {
  return (
    <>
      {page !== null && (
        <div className="flex items-center gap-1 mt-2 mb-1">
          <BookOpen className="h-3 w-3 text-muted-foreground/70" />
          <span className="text-[10px] font-semibold uppercase tracking-wide
                           text-muted-foreground bg-muted/30 rounded px-1.5 py-0.5">
            Page {page}
          </span>
        </div>
      )}
      {passages.map((chunk, idx) => (
        <PassageRow
          key={chunk.chunk_id ?? idx}
          chunk={chunk}
          docId={docId}
          normalizeScore={normalizeScore}
          onDocumentClick={onDocumentClick}
        />
      ))}
    </>
  );
}
```

---

### Task F-12: `PassageRow` sub-component with `p.N` deeplink badge

Extract the existing passage `<button>` into a `PassageRow` component
and add a `p.N` badge using `buildDocumentPageUrl`:

```tsx
function PassageRow({
  chunk,
  docId,
  normalizeScore,
  onDocumentClick,
}: { ... }) {
  const score = normalizeScore(chunk.score);
  const pageUrl = (chunk.document_id && chunk.page_start)
    ? buildDocumentPageUrl(chunk.document_id, chunk.chunk_id, chunk.page_start)
    : null;

  return (
    <div className="flex items-start gap-1.5">
      <button
        className="flex-1 text-left p-2 rounded-md bg-muted/30 hover:bg-yellow-50
                   dark:hover:bg-yellow-900/20 border border-transparent
                   hover:border-yellow-200 dark:hover:border-yellow-800 transition-colors"
        onClick={() => onDocumentClick?.(docId, chunk.content, chunk.chunk_index,
                                         chunk.start_line, chunk.end_line, chunk.chunk_id)}
      >
        <p className="text-xs text-muted-foreground line-clamp-2">{chunk.content}</p>
        <div className="flex items-center gap-1 mt-1">
          <span className={`text-xs font-semibold ${scoreColor(score)}`}>
            {Math.round(score * 100)}%
          </span>
        </div>
      </button>
      {/* Page deeplink badge — separate from passage select button */}
      {pageUrl && (
        <Link
          href={pageUrl}
          className="flex-shrink-0 text-[10px] font-medium text-primary
                     hover:underline flex items-center gap-0.5 mt-2 self-start
                     focus-visible:outline-none focus-visible:ring-1
                     focus-visible:ring-primary/50 rounded-sm px-1"
          title={`Open PDF at page ${chunk.page_start}`}
          aria-label={`Open document at page ${chunk.page_start}`}
        >
          p.{chunk.page_start}
          <ExternalLink className="h-2.5 w-2.5" />
        </Link>
      )}
    </div>
  );
}
```

---

### Task F-13: Wire grouping into `DocumentsTab`

In `DocumentsTab`, replace the `visibleChunks.map(...)` with:

```tsx
{(() => {
  const grouped = groupPassagesByPage(visibleChunks);
  if (grouped) {
    return [...grouped.entries()]
      .sort(([a], [b]) => {
        if (a === null) return 1;
        if (b === null) return -1;
        return a - b;
      })
      .map(([page, passages]) => (
        <PagePassageGroup
          key={page ?? 'nopage'}
          page={page}
          passages={passages}
          docId={docId}
          normalizeScore={normalizeScore}
          onDocumentClick={onDocumentClick}
        />
      ));
  }
  // Flat fallback (non-PDF or no page data)
  return visibleChunks.map((chunk, idx) => (
    <PassageRow
      key={chunk.chunk_id ?? idx}
      chunk={chunk}
      docId={docId}
      normalizeScore={normalizeScore}
      onDocumentClick={onDocumentClick}
    />
  ));
})()}
```

---

### Phase 5 Acceptance Criteria

- [ ] Query results for PDF document show passages grouped under "Page N" headers
- [ ] Non-PDF query results remain flat (no Page headers)
- [ ] `p.N ↗` badge is a `<Link>` that opens document at correct page
- [ ] `bun test` passes
- [ ] No TypeScript strict errors

---

## Phase Summary

| Phase | Scope                  | Key Files Changed                                         | Effort |
| ----- | ---------------------- | --------------------------------------------------------- | ------ |
| P1    | API page fields        | `chunk_detail.rs`, `lineage_types/chunk.rs`, `queries.rs` | XS     |
| P2    | TS types + URL helper  | `lineage.ts`, `document-url.ts`                           | XS     |
| P3    | Controlled PDF viewer  | `pdf-viewer.tsx`, `page.tsx`                              | S      |
| P4    | Page-grouped hierarchy | `document-hierarchy-tree.tsx`                             | M      |
| P5    | Page-grouped citations | `source-citations.tsx`                                    | M      |

---

## Migration / Rollout Notes

1. Deploy P1 (API) before P2–P5 (frontend) — backend is additive and
   safe to deploy independently.
2. Existing documents without `page_start` in KV are not affected:
   the API omits the field, and the UI falls back to flat layout.
3. No database migration required.
4. No feature flag needed — the page grouping only activates when data
   is present, so it is invisibly off for non-PDF or pre-SPEC-032 docs.
