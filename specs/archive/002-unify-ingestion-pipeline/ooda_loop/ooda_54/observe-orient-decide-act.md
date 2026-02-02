# OODA-54: Loading State Management

**Date**: 2026-02-01
**Focus**: Loading UX Patterns

## OBSERVE

### Mission Requirements (Re-read from specs/002-unify-ingestion-pipeline.md)
- Clear loading indicators
- Progressive loading for large content

### Current Loading States

**Document List Loading:**
```typescript
const { data, isLoading, isFetching } = useQuery({
  queryKey: ['documents', workspaceId],
  queryFn: () => getDocuments(workspaceId),
});

if (isLoading) {
  return <DocumentListSkeleton />;
}
```

**PDF Loading:**
```typescript
<Document
  file={file}
  loading={<PDFLoadingSkeleton />}
  onLoadSuccess={({ numPages }) => setNumPages(numPages)}
>
```

## ORIENT

### Loading State Categories

| Component | Initial Load | Re-fetch | Error |
|-----------|--------------|----------|-------|
| Document List | Skeleton | Subtle indicator | Error card |
| PDF Viewer | Spinner | Overlay | Error message |
| Markdown | Skeleton | Fade | Error message |
| Upload | Progress bar | N/A | Toast |

### Loading Optimization
- Skeleton preserves layout space
- Progressive rendering for PDF pages
- Stale-while-revalidate for lists

## DECIDE

**Decision**: Loading states are well-implemented

Current patterns follow best practices:
1. Skeleton loading for initial render
2. Subtle re-fetch indicator
3. Progress tracking for uploads

## ACT

### PDFLoadingSkeleton Implementation

```typescript
const PDFLoadingSkeleton = () => (
  <div className="space-y-4 animate-pulse">
    <div className="h-8 bg-muted rounded w-32" />
    <div className="aspect-[8.5/11] bg-muted rounded" />
    <div className="flex gap-2">
      <div className="h-8 w-8 bg-muted rounded" />
      <div className="h-8 w-8 bg-muted rounded" />
      <div className="h-8 w-8 bg-muted rounded" />
    </div>
  </div>
);
```

### Stale While Revalidate

```typescript
const queryOptions = {
  staleTime: 30_000, // 30 seconds
  gcTime: 5 * 60_000, // 5 minutes
  refetchOnWindowFocus: true,
};
```

**Status**: ✅ VERIFIED - Loading states complete
