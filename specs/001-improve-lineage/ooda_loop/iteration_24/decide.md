# Decision - Iteration 24

## Changes to Make
1. `edgequake.ts` — Add `exportDocumentLineage(docId, format)` using link click pattern
2. `lineage-export.tsx` — New component with JSON/CSV download buttons
3. `metadata-sidebar.tsx` — Add "Export Lineage" collapsible section

## Expected Outcome
- Users can download lineage as JSON or CSV from the metadata sidebar
- Two clear buttons, loading states, error handling
- TypeScript compiles cleanly
