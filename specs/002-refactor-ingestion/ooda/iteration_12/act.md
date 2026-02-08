# OODA-12: Act

## Summary
Extracted `DocumentTableStates` component for loading skeleton and empty state.

## Changes Made

### New File: `document-table-states.tsx` (96 lines)
- LoadingSkeleton: Animated skeleton rows matching table structure
- EmptyState: Centered empty state with upload CTA
- Returns null when neither state applies (table renders)

### Modified: `document-manager.tsx`
- **Before**: 1399 lines
- **After**: 1373 lines
- **Reduction**: 26 lines

### Import Cleanup
Removed 2 unused imports:
- `Skeleton` from UI
- `Upload` from lucide-react

## Cumulative Progress

| Iteration | Component | Lines Saved |
|-----------|-----------|-------------|
| OODA-04 | useStuckDetection hook | -33 |
| OODA-05 | useDocumentWebSocket hook | -50 |
| OODA-06 | UploadProgressList | -126 |
| OODA-07 | BatchActionsBar | -15 |
| OODA-08 | DocumentDropzone | -24 |
| OODA-09 | DocumentActionsMenu | -61 |
| OODA-10 | QuickActionButtons | -75 |
| OODA-11 | ProcessingStatusSummary | -45 |
| **OODA-12** | **DocumentTableStates** | **-26** |
| **Total** | | **-455** |

**DocumentManager**: 1822 → 1373 lines (target: <300, remaining: ~1073 lines)

## Verification
- TypeScript: ✅ No new errors
- Conditional rendering works correctly
