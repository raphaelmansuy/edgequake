# OODA Iteration 31 - Act

## Changes Made

1. Added Copy icon import from lucide-react
2. Added "Copy ID" option to document dropdown menu
3. Uses navigator.clipboard API with toast confirmation

## Files Modified

- `edgequake_webui/src/components/documents/document-manager.tsx`
  - Added Copy to lucide imports (~line 72)
  - Added Copy ID DropdownMenuItem (~line 1317)

## Verification

- TypeScript compilation: ✅ No errors

## Result

Users can now quickly copy document ID from the dropdown menu for debugging/API use.
