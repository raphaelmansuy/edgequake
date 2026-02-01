# OODA-44: Document Preview Panel View Details Link

**Date**: 2026-02-01
**Focus**: View Details Link in Document Preview Panel

## OBSERVE

### Mission Requirements (Re-read from specs/002-unify-ingestion-pipeline.md)
- Ensure in document panel we have a "View Details" link to navigate to detail page
- No code duplication between list and detail views

### Current Implementation
The preview panel's `onViewFull` callback was previously:
- Opening PDF viewer dialog for PDFs
- Navigating to detail page for non-PDFs

### Code Before
```typescript
onViewFull={(doc) => {
  if (doc.source_type === 'pdf' || doc.pdf_id) {
    handleViewPdf(doc);  // Opens dialog
  } else {
    router.push(`/documents/${doc.id}`);
  }
}}
```

## ORIENT

### First Principle: Consistent Navigation
- All document types should navigate to the same detail page
- Dialog was redundant since detail page now has PDF viewer
- Reduces code paths and maintenance burden

## DECIDE

**Decision**: Simplify `onViewFull` to always navigate to detail page

## ACT

### Changes Made

**File**: `edgequake_webui/src/components/documents/document-manager.tsx`

**Updated onViewFull in preview panel** (lines 1638-1641):
```typescript
onViewFull={(doc) => {
  // OODA-41: Always navigate to document detail page
  // WHY: Per SPEC-002, use dedicated page instead of dialog
  router.push(`/documents/${doc.id}`);
}}
```

### Code After
All document types use the same navigation:
1. Preview panel "View Full" → `/documents/[id]`
2. Double-click row → `/documents/[id]`
3. View Details button → `/documents/[id]`

### Evidence
- Single navigation pattern for all documents
- No code duplication
- PDF viewer dialog still available via dropdown menu if needed

**Status**: ✅ COMPLETE - View Details consistently navigates to detail page
