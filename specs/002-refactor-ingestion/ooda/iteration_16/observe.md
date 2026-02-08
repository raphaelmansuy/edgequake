# OODA-16 Observe: useBulkSelection Hook Extraction

## Mission Brief Re-Read
- Target: DocumentManager < 300 lines
- Current: 841 lines (53.8% reduction achieved)
- Remaining: ~541 lines to reduce

## Code Analysis

### Target: Bulk Selection Handlers (Lines ~344-420)
The bulk selection logic includes:

1. **Selection state**: `selectedIds` Set
2. **Selection handlers**:
   - `handleSelectAll(checked)` - Select/deselect all
   - `handleSelectOne(docId, checked)` - Toggle single selection
   - `handleClearSelection()` - Clear all selections
3. **Bulk operation handlers**:
   - `handleBulkDelete()` - Delete selected documents
   - `handleBulkReprocess()` - Reprocess selected documents

Total: ~75 lines

### Dependencies
- `selectedIds` state (manages internally)
- `documents` array (for handleSelectAll mapping)
- `data?.items` (for track_ids in reprocess)
- `deleteDocument`, `reprocessDocument` API calls
- `queryClient` for cache invalidation
- `t` for translations
- `toast` for notifications

### Hook Interface
```typescript
interface UseBulkSelectionOptions {
  documents: Document[];
}

interface UseBulkSelectionReturn {
  selectedIds: Set<string>;
  handleSelectAll: (checked: boolean) => void;
  handleSelectOne: (docId: string, checked: boolean) => void;
  handleClearSelection: () => void;
  handleBulkDelete: () => Promise<void>;
  handleBulkReprocess: () => Promise<void>;
}
```

## Line Count Estimation
- Lines removed from DocumentManager: ~75
- New hook: ~150 lines
- Net reduction: ~75 lines

## Next: Orient
