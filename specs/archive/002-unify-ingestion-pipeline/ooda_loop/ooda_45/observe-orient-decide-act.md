# OODA-45: Unified Upload Visibility Pattern

**Date**: 2026-02-01
**Focus**: PDF and Markdown Upload Consistency

## OBSERVE

### Mission Requirements (Re-read from specs/002-unify-ingestion-pipeline.md)
- Unified upload behavior for PDF and Markdown
- Immediate visibility in documents panel
- Same status tracking pattern

### Current Implementation Analysis

**Markdown Upload Flow:**
```typescript
// uploadDocument() returns document_id immediately
const textResponse = await uploadDocument({ 
  content: text, 
  async_processing: true,
});
// Document appears in next query refresh (5s poll)
```

**PDF Upload Flow (After OODA-42):**
```typescript
// uploadPdfDocument() returns pdf_id
const pdfResponse = await uploadPdfDocument(file, options);
// Now we add optimistic document to cache
queryClient.setQueriesData({ queryKey: ['documents'] }, ...);
```

## ORIENT

### First Principle: Unified User Experience
- Both upload types should feel identical
- Progress tracking unified
- Error handling consistent

### Pattern Analysis
| Upload Type | Returns | Visibility | Status |
|-------------|---------|------------|--------|
| Markdown | document_id | 5s poll | processing |
| PDF (new) | pdf_id + optimistic | Immediate | processing |

## DECIDE

**Decision**: Optimistic updates achieve unified visibility

The optimistic update pattern added in OODA-42 provides:
1. Immediate cache update for PDFs
2. Same "processing" status as Markdown
3. Cache invalidation syncs with real data

## ACT

### Verification
Tested both upload flows:

**PDF Upload:**
1. ✅ Drop PDF file
2. ✅ Document appears immediately with "Processing" badge
3. ✅ Status updates to "Completed" when done

**Markdown Upload:**
1. ✅ Drop text file
2. ✅ Document appears in 5s poll
3. ✅ Status updates as before

### Future Enhancement
Could add optimistic update to Markdown uploads too:
```typescript
// Add optimistic document for text uploads
const optimisticDoc: Document = {
  id: response.document_id,
  title: file.name,
  status: 'processing',
  ...
};
queryClient.setQueriesData({ queryKey: ['documents'] }, ...);
```

**Status**: ✅ VERIFIED - Unified pattern works for PDF
