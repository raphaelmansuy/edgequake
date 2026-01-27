# OODA Iteration 32 - Act

## Changes Made

1. Added `highlightMatches()` helper function using regex split
2. Highlights matched text with yellow background (dark mode aware)
3. Applied to document title display in table

## Files Modified

- `edgequake_webui/src/components/documents/document-manager.tsx`
  - Added `highlightMatches()` helper (~line 146)
  - Updated title span to use highlightMatches (~line 1237)

## Verification

- TypeScript compilation: ✅ No errors

## Result

Search terms now highlight in yellow within document titles for clear visual feedback.
