# Data Model — SPEC-033 Page Lineage

## 1. Rust Side

### 1.1 `ChunkDetailResponse` — add page fields

**File**: `edgequake/crates/edgequake-api/src/handlers/lineage_types/chunk.rs`

```rust
/// Chunk detail response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ChunkDetailResponse {
    // ... existing fields ...

    /// PDF page number (1-indexed) where this chunk starts.
    /// Present only when the source was a PDF with page markers (SPEC-033).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_start: Option<u32>,

    /// PDF page number (1-indexed) where this chunk ends.
    /// Always equal to `page_start` — chunks never cross page boundaries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_end: Option<u32>,
}
```

**Handler change** (`chunk_detail.rs`): read from KV record:

```rust
let page_start = chunk_data
    .get("page_start")
    .and_then(|v| v.as_u64())
    .map(|v| v as u32);

let page_end = chunk_data
    .get("page_end")
    .and_then(|v| v.as_u64())
    .map(|v| v as u32);
```

---

### 1.2 `FullLineageChunk` in lineage KV — already stored; surface in response

The KV record written by `chunk_kv_value()` already includes `page_start`
and `page_end` when the source is a PDF.  The `/documents/:id/lineage`
handler must read and forward these fields.

**File**: `edgequake/crates/edgequake-api/src/handlers/lineage/queries.rs`
(specifically the full-lineage endpoint that reads KV chunk records).

The chunk serialisation step that builds the `lineage.chunks` array must
be extended:

```rust
// In the section that serialises each chunk KV record:
let page_start = chunk_data
    .get("page_start")
    .and_then(|v| v.as_u64())
    .map(|v| v as u32);
let page_end = chunk_data
    .get("page_end")
    .and_then(|v| v.as_u64())
    .map(|v| v as u32);

// Include in the serialised chunk JSON:
if let Some(p) = page_start {
    chunk_json["page_start"] = json!(p);
    chunk_json["page_end"]   = json!(page_end.unwrap_or(p));
}
```

---

### 1.3 `SourceReference` — already has page fields (no change needed)

`page_start` and `page_end` are already present on `SourceReference` in
`query_types.rs` and are already populated by `source_reference_builder.rs`
from the `RetrievedChunk`.  No Rust change required for query endpoints.

---

## 2. TypeScript Side

### 2.1 `ChunkDetail` — add page fields

**File**: `edgequake_webui/src/types/lineage.ts`

```ts
export interface ChunkDetail {
  // ... existing fields ...

  /** PDF page number (1-indexed) where this chunk starts. SPEC-033. */
  page_start?: number;
  /** PDF page number (1-indexed) where this chunk ends. Always equals page_start. */
  page_end?: number;
}
```

---

### 2.2 `ChunkLineage` — add page fields

**File**: `edgequake_webui/src/types/lineage.ts`

```ts
export interface ChunkLineage {
  // ... existing fields ...

  /** PDF page number (1-indexed). SPEC-033. */
  page_start?: number;
  /** PDF page number (1-indexed). Always equals page_start. */
  page_end?: number;
}
```

---

### 2.3 `FullLineageChunk` (local interface in `document-hierarchy-tree.tsx`)

```ts
interface FullLineageChunk {
  chunk_id: string;
  chunk_index: number;
  start_line?: number;
  end_line?: number;
  start_offset?: number;
  end_offset?: number;
  entity_ids?: string[];
  extraction_metadata?: Record<string, unknown>;
  relationship_ids?: string[];
  /** PDF page number (1-indexed). Present only for PDFs. SPEC-033. */
  page_start?: number;
  /** Always equals page_start. */
  page_end?: number;
}
```

---

### 2.4 `PDFViewerProps` — add `currentPage` controlled prop

**File**: `edgequake_webui/src/components/documents/pdf-viewer.tsx`

```ts
interface PDFViewerProps {
  // ... existing fields ...

  /**
   * Controlled current page (1-indexed).
   * When provided, the viewer navigates to this page whenever the value changes.
   * Takes precedence over `initialPage` for runtime navigation.
   * SPEC-033: Required for chunk-click-to-page navigation.
   */
  currentPage?: number;
}
```

Internal sync effect:

```ts
// Sync pageNumber with controlled currentPage prop.
// WHY: initialPage is a one-shot seed; currentPage drives the viewer after mount.
useEffect(() => {
  if (currentPage !== undefined && currentPage !== pageNumber) {
    setPageNumber(Math.max(1, Math.min(numPages || currentPage, currentPage)));
  }
}, [currentPage]); // intentionally exclude pageNumber to avoid feedback loop
```

---

## 3. Derived State — Page Grouping

The page grouping is a **pure UI transformation** — no new storage, no new
API calls.  It is computed in `DocumentHierarchyTree` from the loaded
`FullLineageChunk[]`:

```ts
/** Group chunks by page number when page data is available. */
function groupChunksByPage(
  chunks: FullLineageChunk[]
): Map<number, FullLineageChunk[]> | null {
  const hasPages = chunks.some(c => c.page_start !== undefined);
  if (!hasPages) return null; // fall back to flat layout

  const map = new Map<number, FullLineageChunk[]>();
  for (const chunk of chunks) {
    const page = chunk.page_start ?? 0; // 0 = "no page" bucket
    const list = map.get(page) ?? [];
    list.push(chunk);
    map.set(page, list);
  }
  return map;
}
```

---

## 4. URL Schema (canonical)

| Parameter | Type                | Source                        |
| --------- | ------------------- | ----------------------------- |
| `chunk`   | string (chunk UUID) | chunk click or citation click |
| `page`    | integer ≥ 1         | `chunk.page_start`            |

Full URL example:
```
/documents/5d52f10f-1d42-40b5-a41e-d75b3a44f1ae?chunk=chunk-abc123&page=3
```

The document detail page already reads `searchParams.get('page')` and
passes it as `initialPdfPage` to `PDFViewer`.  After this spec, it also
passes it as `currentPage` so re-navigation works without a full reload.
