# OODA-14 Act: useDocumentMutations Hook Implementation

## Changes Made

### File: `hooks/use-document-mutations.ts` (NEW - 266 lines)
- Extracted all 4 document mutations:
  - `deleteMutation`: Delete single document
  - `deleteAllMutation`: Delete all documents  
  - `reprocessMutation`: Retry failed document
  - `cancelMutation`: Cancel processing
- Added TypeScript interfaces for options and return types
- Added JSDoc documentation with feature/use case references
- Fixed reprocessMutation return type to match API (`{ track_id, message, count }`)
- Added `isAnyMutationPending` convenience flag

### File: `document-manager.tsx` (1064 → 988 lines)
- Added import for useDocumentMutations
- Added hook call with callback:
  ```typescript
  const { deleteMutation, deleteAllMutation, reprocessMutation, cancelMutation } = 
    useDocumentMutations({ onReprocessSuccess: () => setPipelineDialogOpen(true) });
  ```
- Removed ~76 lines of inline mutation definitions
- Removed unused imports: `cancelTask`, `deleteAllDocuments`, `useMutation`

## Metrics

| Metric | Before | After | Delta |
|--------|--------|-------|-------|
| DocumentManager | 1064 | 988 | **-76** |
| New hook | 0 | 266 | +266 |
| Cumulative saved | 758 | 834 | +76 |
| % of original | 41.6% | 45.8% | +4.2% |

## Files Changed
1. `hooks/use-document-mutations.ts` - Created
2. `document-manager.tsx` - Modified

## Verification
- ✅ TypeScript compilation clean (hook)
- ✅ Only pre-existing warnings remain in main component
- ✅ Line count verified

## Commit
`OODA-14: Extract useDocumentMutations hook`
