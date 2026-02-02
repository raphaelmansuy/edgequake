# OODA-61: Table Pagination

**Date**: 2026-02-01
**Focus**: Document List Pagination

## OBSERVE

### Mission Requirements (Re-read from specs/002-unify-ingestion-pipeline.md)
- Efficient document list rendering
- Pagination for large lists

### Current Pagination State

**From document-manager.tsx:**
```typescript
const [pagination, setPagination] = useState({
  pageIndex: 0,
  pageSize: 20,
});

const paginatedDocuments = useMemo(() => {
  const start = pagination.pageIndex * pagination.pageSize;
  const end = start + pagination.pageSize;
  return sortedDocuments.slice(start, end);
}, [sortedDocuments, pagination]);

const totalPages = Math.ceil(sortedDocuments.length / pagination.pageSize);
```

## ORIENT

### Pagination Features

| Feature | Status | Implementation |
|---------|--------|----------------|
| Page size selector | ✅ | 10, 20, 50, 100 |
| Page navigation | ✅ | First, Prev, Next, Last |
| Page jumping | ❌ | Future enhancement |
| Total count | ✅ | "X of Y documents" |

### Performance Implications
- Client-side pagination: Works for <1000 docs
- Server-side pagination: Needed for larger datasets
- Current approach: Client-side sufficient

## DECIDE

**Decision**: Client-side pagination correctly implemented

Benefits:
- Instant page changes
- Works with filtering/sorting
- No API calls per page

## ACT

### Pagination Controls

```typescript
<div className="flex items-center justify-between">
  <div className="text-sm text-muted-foreground">
    Showing {pagination.pageIndex * pagination.pageSize + 1} to{' '}
    {Math.min(
      (pagination.pageIndex + 1) * pagination.pageSize,
      sortedDocuments.length
    )}{' '}
    of {sortedDocuments.length} documents
  </div>
  
  <div className="flex items-center gap-2">
    <Button
      variant="outline"
      size="sm"
      onClick={() => setPagination(p => ({ ...p, pageIndex: 0 }))}
      disabled={pagination.pageIndex === 0}
    >
      <ChevronsLeft className="h-4 w-4" />
    </Button>
    <Button
      variant="outline"
      size="sm"
      onClick={() => setPagination(p => ({ ...p, pageIndex: p.pageIndex - 1 }))}
      disabled={pagination.pageIndex === 0}
    >
      <ChevronLeft className="h-4 w-4" />
    </Button>
    
    <span className="text-sm">
      Page {pagination.pageIndex + 1} of {totalPages}
    </span>
    
    <Button
      variant="outline"
      size="sm"
      onClick={() => setPagination(p => ({ ...p, pageIndex: p.pageIndex + 1 }))}
      disabled={pagination.pageIndex >= totalPages - 1}
    >
      <ChevronRight className="h-4 w-4" />
    </Button>
    <Button
      variant="outline"
      size="sm"
      onClick={() => setPagination(p => ({ ...p, pageIndex: totalPages - 1 }))}
      disabled={pagination.pageIndex >= totalPages - 1}
    >
      <ChevronsRight className="h-4 w-4" />
    </Button>
  </div>
</div>
```

**Status**: ✅ VERIFIED - Pagination complete
