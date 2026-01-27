# Act - Iteration 08: Pipeline ETA Display

## Actions Taken

### 1. Added ETA Calculation Function
Added `useMemo` hook with ETA calculation logic:
- Calculates processing rate from elapsed time
- Estimates remaining time based on current rate
- Handles edge cases (initial 30s warmup, near completion)
- Provides human-readable output (minutes, hours+minutes)

### 2. Added ETA Display in Progress Section
Added visual ETA indicator with Clock icon:
- Positioned below progress bar
- Uses i18n for all text
- Centered layout with muted styling

### 3. Fixed Syntax Error
Fixed double `}}` syntax error that occurred during edit.

## Files Changed
- `edgequake_webui/src/components/documents/pipeline-status-dialog.tsx`
  - Added `Clock` icon import
  - Added `useMemo` import
  - Added ETA calculation hook (~25 lines)
  - Added ETA display component (~5 lines)

## Verification
- ✅ TypeScript compilation passes (`pnpm exec tsc --noEmit`)

## Impact
- **UX Improvement**: Users now see estimated time remaining
- **Transparency**: Better understanding of processing duration
- **Confidence**: Users know when to expect completion

## Next Steps
- Continue to Iteration 09
- Focus on additional UX improvements or testing
