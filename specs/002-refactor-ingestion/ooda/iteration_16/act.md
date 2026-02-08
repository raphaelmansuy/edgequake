# OODA-16 Act: useBulkSelection Hook Implementation

## Changes Made

### File: `hooks/use-bulk-selection.ts` (NEW - 287 lines)
- Encapsulates selection state (`selectedIds`)
- Provides computed values (`selectedCount`, `isAllSelected`)
- Handles selection operations:
  - `handleSelectAll(checked)` - Select/deselect all
  - `handleSelectOne(docId, checked)` - Toggle single item
  - `handleClearSelection()` - Clear all
- Provides bulk operations:
  - `handleBulkDelete()` - Delete selected with progress
  - `handleBulkReprocess()` - Reprocess selected with progress
- Tracks loading states (`isBulkDeleting`, `isBulkReprocessing`)

### File: `document-manager.tsx` (841 → 767 lines)
- Added import for useBulkSelection
- Replaced inline state and 6 handlers with hook call:
  ```typescript
  const {
    selectedIds, selectedCount, isAllSelected,
    handleSelectAll, handleSelectOne, handleClearSelection,
    handleBulkDelete, handleBulkReprocess,
  } = useBulkSelection({ documents });
  ```
- Updated keyboard effect to use `selectedCount` and `handleClearSelection`
- Updated BatchActionsBar to use `selectedCount`
- Updated checkbox to use `isAllSelected`
- Removed unused imports: `deleteDocument`, `reprocessDocument`

## Metrics

| Metric | Before | After | Delta |
|--------|--------|-------|-------|
| DocumentManager | 841 | 767 | **-74** |
| New hook | 0 | 287 | +287 |
| Cumulative saved | 981 | 1055 | +74 |
| % of original | 53.8% | 57.9% | +4.1% |

## Files Changed
1. `hooks/use-bulk-selection.ts` - Created
2. `document-manager.tsx` - Modified

## Verification
- ✅ TypeScript compilation clean
- ✅ Only pre-existing warnings remain
- ✅ Line count verified: 767 lines

## Commit
`OODA-16: Extract useBulkSelection hook`
