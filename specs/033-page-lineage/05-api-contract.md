# API Contract — SPEC-033 Page Lineage

## 1. GET /api/v1/chunks/:id — Add Page Fields

### Before (current)

```json
{
  "chunk_id": "chunk-abc123",
  "document_id": "5d52f10f-1d42-40b5-a41e-d75b3a44f1ae",
  "document_name": "m_renault_espace_rhn_fr_mai_2025.pdf",
  "content": "# motorisation ## full hybrid E-Tech 200 ch ...",
  "index": 0,
  "char_range": { "start": 0, "end": 1024 },
  "start_line": 1,
  "end_line": 18,
  "token_count": 312,
  "entities": [...],
  "relationships": [...],
  "extraction_metadata": { "model": "qwen2.5:latest", ... }
}
```

### After (SPEC-033)

```json
{
  "chunk_id": "chunk-abc123",
  "document_id": "5d52f10f-1d42-40b5-a41e-d75b3a44f1ae",
  "document_name": "m_renault_espace_rhn_fr_mai_2025.pdf",
  "content": "# motorisation ## full hybrid E-Tech 200 ch ...",
  "index": 0,
  "char_range": { "start": 0, "end": 1024 },
  "start_line": 1,
  "end_line": 18,
  "token_count": 312,
  "page_start": 1,
  "page_end": 1,
  "entities": [...],
  "relationships": [...],
  "extraction_metadata": { "model": "qwen2.5:latest", ... }
}
```

**New fields** (both optional — omitted for non-PDF / pre-SPEC-032 documents):
- `page_start?: number` — 1-indexed PDF page where this chunk starts.
- `page_end?: number` — always equals `page_start`.

---

## 2. GET /api/v1/documents/:id/lineage — Add Page Fields to Chunks

### Before (current)

```json
{
  "document_id": "5d52f10f-...",
  "metadata": { ... },
  "lineage": {
    "document_id": "5d52f10f-...",
    "document_name": "m_renault_espace_rhn_fr_mai_2025.pdf",
    "chunks": [
      {
        "chunk_id": "chunk-abc123",
        "chunk_index": 0,
        "start_line": 1,
        "end_line": 18,
        "entity_ids": ["SUV", "RENAULT", ...],
        "relationship_ids": [...]
      }
    ],
    "entities": { ... },
    ...
  }
}
```

### After (SPEC-033)

```json
{
  "document_id": "5d52f10f-...",
  "metadata": { ... },
  "lineage": {
    "document_id": "5d52f10f-...",
    "document_name": "m_renault_espace_rhn_fr_mai_2025.pdf",
    "chunks": [
      {
        "chunk_id": "chunk-abc123",
        "chunk_index": 0,
        "start_line": 1,
        "end_line": 18,
        "page_start": 1,
        "page_end": 1,
        "entity_ids": ["SUV", "RENAULT", ...],
        "relationship_ids": [...]
      }
    ],
    "entities": { ... },
    ...
  }
}
```

**New fields per chunk item** (both optional):
- `page_start?: number` — 1-indexed PDF page.
- `page_end?: number` — always equals `page_start`.

---

## 3. GET /api/v1/query (and /query/stream) — No Change

`SourceReference` already has:

```json
{
  "source_type": "chunk",
  "id": "chunk-abc123",
  "score": 0.95,
  "snippet": "# motorisation ...",
  "document_id": "5d52f10f-...",
  "page_start": 1,
  "page_end": 1
}
```

No server-side changes needed.  The page data flows through already.

---

## 4. Rust DTO Changes Summary

| Struct                                      | File                     | Change                                                   |
| ------------------------------------------- | ------------------------ | -------------------------------------------------------- |
| `ChunkDetailResponse`                       | `lineage_types/chunk.rs` | Add `page_start: Option<u32>`, `page_end: Option<u32>`   |
| Chunk serialisation in full-lineage handler | `lineage/queries.rs`     | Read `page_start`/`page_end` from KV and include in JSON |

### 4.1 `ChunkDetailResponse` diff

```rust
// lineage_types/chunk.rs — add after end_line field:

/// PDF page number (1-indexed) where this chunk starts.
/// Present only for PDFs ingested with SPEC-032 page-aware chunking.
/// @implements SPEC-033 — page attribution surfacing
#[serde(skip_serializing_if = "Option::is_none")]
pub page_start: Option<u32>,

/// PDF page number (1-indexed) where this chunk ends.
/// Always equal to page_start (chunks never span pages).
#[serde(skip_serializing_if = "Option::is_none")]
pub page_end: Option<u32>,
```

### 4.2 `chunk_detail.rs` read logic diff

```rust
// After reading end_line, add:
let page_start = chunk_data
    .get("page_start")
    .and_then(|v: &serde_json::Value| v.as_u64())
    .map(|v| v as u32);

let page_end = chunk_data
    .get("page_end")
    .and_then(|v: &serde_json::Value| v.as_u64())
    .map(|v| v as u32);
```

```rust
// In the Ok(Json(ChunkDetailResponse { ... })) block, add:
page_start,
page_end,
```

---

## 5. TypeScript Type Changes Summary

| Type / Interface           | File                          | Change                                         |
| -------------------------- | ----------------------------- | ---------------------------------------------- |
| `ChunkDetail`              | `src/types/lineage.ts`        | Add `page_start?: number`, `page_end?: number` |
| `ChunkLineage`             | `src/types/lineage.ts`        | Add `page_start?: number`, `page_end?: number` |
| `FullLineageChunk` (local) | `document-hierarchy-tree.tsx` | Add `page_start?: number`, `page_end?: number` |
| `PDFViewerProps`           | `pdf-viewer.tsx`              | Add `currentPage?: number`                     |

---

## 6. OpenAPI Schema Update

The OpenAPI schema in `openapi.rs` uses `#[derive(ToSchema)]` on
`ChunkDetailResponse`.  Adding the new fields with `#[serde(skip_serializing_if)]`
will automatically update the generated OpenAPI spec — no manual change needed.

---

## 7. Backward Compatibility Matrix

| Scenario                          | Behaviour                                    |
| --------------------------------- | -------------------------------------------- |
| Non-PDF document                  | `page_start`/`page_end` absent from response |
| PDF ingested before SPEC-032 W-09 | `page_start`/`page_end` absent from response |
| PDF ingested with SPEC-032        | `page_start`/`page_end` present              |
| Old client consuming new API      | Unknown fields silently ignored              |
| New client consuming old API      | `page_start` undefined → flat layout         |
