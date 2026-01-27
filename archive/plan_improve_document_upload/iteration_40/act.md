# OODA Iteration 40 - Act

## Changes Made

1. Added `handleDocumentDoubleClick` callback for navigation to graph
2. Only navigates for completed documents
3. Added onDoubleClick to TableRow

## Files Modified

- `edgequake_webui/src/components/documents/document-manager.tsx`
  - Added handleDocumentDoubleClick (~line 739)
  - Added onDoubleClick prop to TableRow (~line 1247)

## Verification

- TypeScript compilation: ✅ No errors

## Result

Users can now double-click completed documents to navigate directly to graph.
