# OODA-17 Observe: useDocumentPreferences Hook Extraction

## Mission Brief Re-Read

- Target: DocumentManager < 300 lines
- Current: 767 lines (57.9% reduction achieved)
- Remaining: ~467 lines to reduce

## Code Analysis

### Target: localStorage Preferences (Lines ~113-150, ~451-462)

The preferences logic includes:

1. **State initialization** (4 states with localStorage fallback):
   - `pageSize` (lines 115-123)
   - `statusFilter` (lines 127-135)
   - `sortField` (lines 136-144)
   - `sortDirection` (lines 145-153)

2. **Persistence effect** (lines 451-462):
   - Writes all 4 values to localStorage on change

Total: ~50 lines

### Dependencies

- `localStorage` browser API
- DocStatus, SortField, SortDirection types

### Hook Interface

```typescript
interface UseDocumentPreferencesReturn {
  pageSize: number;
  setPageSize: (size: number) => void;
  statusFilter: DocStatus;
  setStatusFilter: (status: DocStatus) => void;
  sortField: SortField;
  setSortField: (field: SortField) => void;
  sortDirection: SortDirection;
  setSortDirection: (direction: SortDirection) => void;
}
```

## Line Count Estimation

- Lines removed from DocumentManager: ~50
- New hook: ~100 lines
- Net reduction: ~50 lines

## Next: Orient → Decide → Act
