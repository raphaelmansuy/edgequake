# OODA-60: Document Sorting

**Date**: 2026-02-01
**Focus**: Table Column Sorting

## OBSERVE

### Mission Requirements (Re-read from specs/002-unify-ingestion-pipeline.md)
- Sortable document table
- Intuitive sort controls

### Current Sort Implementation

**From document-manager.tsx:**
```typescript
const [sortConfig, setSortConfig] = useState<{
  key: keyof Document | null;
  direction: 'asc' | 'desc';
}>({ key: 'updated_at', direction: 'desc' });

const sortedDocuments = useMemo(() => {
  if (!sortConfig.key) return filteredDocuments;
  
  return [...filteredDocuments].sort((a, b) => {
    const aValue = a[sortConfig.key!];
    const bValue = b[sortConfig.key!];
    
    if (aValue < bValue) return sortConfig.direction === 'asc' ? -1 : 1;
    if (aValue > bValue) return sortConfig.direction === 'asc' ? 1 : -1;
    return 0;
  });
}, [filteredDocuments, sortConfig]);
```

## ORIENT

### Sortable Columns

| Column | Type | Default Direction |
|--------|------|------------------|
| Title | string | A-Z |
| Status | enum | - |
| Created At | date | Newest first |
| Updated At | date | Newest first |
| Size | number | Largest first |

### Sort UX Pattern
```
[First click] → Ascending
[Second click] → Descending
[Third click] → Remove sort (return to default)
```

## DECIDE

**Decision**: Current sort implementation is correct

Client-side sorting provides:
- Instant feedback
- No server load
- Maintains filter state

## ACT

### Column Header Button

```typescript
const SortableHeader = ({ 
  column, 
  label,
  sortConfig,
  onSort 
}: SortableHeaderProps) => (
  <Button
    variant="ghost"
    className="px-0 hover:bg-transparent"
    onClick={() => onSort(column)}
  >
    {label}
    {sortConfig.key === column ? (
      sortConfig.direction === 'asc' 
        ? <ChevronUp className="ml-1 h-4 w-4" />
        : <ChevronDown className="ml-1 h-4 w-4" />
    ) : (
      <ChevronsUpDown className="ml-1 h-4 w-4 opacity-50" />
    )}
  </Button>
);
```

### Sort Handler

```typescript
const handleSort = (column: keyof Document) => {
  setSortConfig(current => {
    if (current.key !== column) {
      return { key: column, direction: 'asc' };
    }
    if (current.direction === 'asc') {
      return { key: column, direction: 'desc' };
    }
    return { key: null, direction: 'desc' };
  });
};
```

**Status**: ✅ VERIFIED - Document sorting complete
