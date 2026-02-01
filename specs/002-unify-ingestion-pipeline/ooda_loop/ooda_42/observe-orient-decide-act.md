# OODA-42: PDF Immediate Visibility After Upload

**Date**: 2026-02-01
**Focus**: Optimistic Updates for PDF Upload

## OBSERVE

### Mission Requirements (Re-read from specs/002-unify-ingestion-pipeline.md)

- PDFs MUST appear immediately in documents panel after upload
- Same behavior as Markdown files
- Use optimistic updates or real-time polling
- Unified upload behavior for PDF and Markdown

### Current State Analysis

```
PDF Upload Flow:
1. User drops PDF file
2. uploadPdfDocument() creates PDF record
3. Background task creates document record async
4. Document appears only after task completes (delayed)

Markdown Upload Flow:
1. User drops text file
2. uploadDocument() creates document record immediately
3. Document appears in list right away
```

**Root Cause:**
Backend creates PDF record with `document_id: None`, then creates document asynchronously during vision processing.

```rust
// pdf_upload.rs line 508
document_id: None,
```

### Query Cache Structure

```typescript
queryKey: [
  "documents",
  selectedTenantId,
  selectedWorkspaceId,
  currentPage,
  pageSize,
  statusFilter,
];
```

## ORIENT

### First Principle: Instant Feedback

- User expects to see their upload immediately
- Processing can happen in background
- Status badge shows "Processing" state

### Solution Options

1. **Backend change**: Create document record immediately with 'pending' status
2. **Frontend optimistic update**: Add temporary document to cache
3. **Both**: Most robust solution

### Chosen Approach

Frontend optimistic update using `setQueriesData`:

- Add temporary Document to all document query caches
- Use `pdf_id` as temporary ID
- Mark status as 'processing'
- Cache will be replaced when real document appears

## DECIDE

**Decision**: Implement optimistic updates for PDF upload

### Implementation

1. After PDF upload response, create optimistic Document object
2. Use `queryClient.setQueriesData` with predicate to update all document caches
3. Check for duplicates before adding
4. Add `selectedWorkspaceId` to callback dependencies

## ACT

### Changes Made

**File**: `edgequake_webui/src/components/documents/document-manager.tsx`

1. **Added optimistic update logic** (lines 341-371):

```typescript
// OODA-42: Optimistic update for PDF upload
// WHY: PDFs must appear immediately in documents panel (same as markdown)
if (pdfResponse.pdf_id && !pdfResponse.duplicate_of) {
  const optimisticDoc: Document = {
    id: pdfResponse.pdf_id, // Use pdf_id as temporary ID
    title: file.name,
    file_name: file.name,
    file_size: file.size,
    source_type: "pdf",
    status: "processing",
    mime_type: "application/pdf",
    created_at: new Date().toISOString(),
    pdf_id: pdfResponse.pdf_id,
    track_id: pdfResponse.track_id,
  };

  // Add to all document query caches for instant visibility
  queryClient.setQueriesData<{ documents: Document[]; total: number }>(
    { queryKey: ["documents"] },
    (old) => {
      if (!old) return old;
      // Check if document already exists (by pdf_id)
      const exists = old.documents.some(
        (d) => d.pdf_id === pdfResponse.pdf_id || d.id === pdfResponse.pdf_id,
      );
      if (exists) return old;
      return {
        documents: [optimisticDoc, ...old.documents],
        total: old.total + 1,
      };
    },
  );
}
```

2. **Updated callback dependencies** (line 518):

```typescript
[queryClient, t, router, selectedWorkspaceId];
```

### Behavior After Change

```
PDF Upload Flow (NEW):
1. User drops PDF file
2. uploadPdfDocument() creates PDF record
3. Frontend adds optimistic document to cache IMMEDIATELY
4. User sees document with "Processing" badge
5. Background task completes
6. Cache invalidation replaces optimistic with real document
```

### Evidence

- PDF appears in list immediately after upload
- Status shows "Processing" badge
- When processing completes, status updates to "Completed"
- Same UX as Markdown upload

**Status**: ✅ COMPLETE - Optimistic updates implemented
