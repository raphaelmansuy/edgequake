# Action - Iteration 13

## Changes Made

### New File: edgequake_webui/src/components/document/document-hierarchy-tree.tsx
- `DocumentHierarchyTree` component (~210 lines)
- Accepts `documentId` and optional `documentName` props
- Fetches data via `useDocumentLineage(documentId)` hook
- Builds entity-by-chunk reverse map with `useMemo`
- Tree structure: Document root → Chunk nodes → Entity leaf nodes
- Chunk nodes show: index, line range, token count, entity count
- Entity nodes show: name (truncated), type badge, extraction count badge
- Fully collapsible with `useState` per node
- Loading and error states handled

### Modified: edgequake_webui/src/components/document/metadata-sidebar.tsx
- Added imports for `DocumentHierarchyTree` and `GitBranch` icon
- Added "Data Hierarchy" collapsible section after "Knowledge Graph"

## Verification
- `npx tsc --noEmit` — CLEAN (0 errors)
- Component is purely additive — existing sidebar sections unchanged
