# Task Log: Document Link Fix for Source Citations

**Date:** 2025-06-20 14:30  
**Mode:** beastmode  
**Commit:** 6e3e881

---

## Summary

Fixed document links in source citations that were producing 404 errors due to malformed URLs containing `-chunk-N` suffix.

## Problem

When clicking "Open document in new window" in source citations, the URL was:

```
/documents/8ddd9d1b-f6cf-4be2-90fe-7ba70ca5780a-chunk-0?highlight=...
```

This resulted in 404 errors because the document ID should be:

```
/documents/8ddd9d1b-f6cf-4be2-90fe-7ba70ca5780a?highlight=...
```

## Root Cause

1. **Backend**: `sota_engine.rs` creates chunks without calling `with_document_id()` - the `document_id` field was `None`
2. **Frontend**: `convertServerMessage()` fell back to `s.id` (chunk ID with suffix) when `s.document_id` was null

## Solution

### Backend (sota_engine.rs)

Added `extract_document_id()` helper function:

```rust
fn extract_document_id(chunk_id: &str) -> Option<String> {
    if let Some(suffix_idx) = chunk_id.rfind("-chunk-") {
        if suffix_idx > 0 {
            return Some(chunk_id[..suffix_idx].to_string());
        }
    }
    None
}
```

Applied in 3 locations:

- `query_local()`
- `query_hybrid()`
- `query_naive()`

### Frontend (query-interface.tsx)

Added `extractDocId()` helper in `convertServerMessage()`:

```typescript
const extractDocId = (chunkId: string): string => {
  const suffixIndex = chunkId.lastIndexOf("-chunk-");
  return suffixIndex > 0 ? chunkId.substring(0, suffixIndex) : chunkId;
};
```

Changed from:

```typescript
document_id: s.document_id ?? s.id,
```

To:

```typescript
document_id: s.document_id ?? extractDocId(s.id),
```

## Testing

Verified with 2 different queries:

| Test | Query                 | Document | URL Format    | Loaded     | Highlighting |
| ---- | --------------------- | -------- | ------------- | ---------- | ------------ |
| 1    | "Project Genesis..."  | 8ddd9d1b | ✅ Clean UUID | ✅ Success | ✅ Visible   |
| 2    | "QuantumTech Labs..." | b10ab852 | ✅ Clean UUID | ✅ Success | ✅ Visible   |

## Files Changed

- `edgequake/crates/edgequake-query/src/sota_engine.rs` - Added extract_document_id() and 3 update sites
- `edgequake_webui/src/components/query/query-interface.tsx` - Added extractDocId() helper

## Actions

- Added `extract_document_id()` helper function to Rust backend
- Updated 3 chunk creation sites in `sota_engine.rs`
- Added `extractDocId()` helper function to TypeScript frontend
- Updated `convertServerMessage()` to use extraction as fallback
- Tested with browser automation (2 successful tests)
- Committed fix with detailed commit message

## Decisions

- Implemented extraction in both frontend and backend for defense in depth
- Used `rfind` in Rust and `lastIndexOf` in TypeScript to find last occurrence
- Frontend extraction serves as fallback when `document_id` is null

## Next Steps

- Consider testing "Open Graph Explorer" link if required
- Monitor for any edge cases with non-standard chunk ID formats

## Lessons/Insights

- Chunk IDs follow format `uuid-chunk-N` where N is the chunk index
- Backend should always populate `document_id` on chunks for proper linking
- Frontend fallback extraction provides defense against null `document_id`

---

**Status:** ✅ COMPLETE
