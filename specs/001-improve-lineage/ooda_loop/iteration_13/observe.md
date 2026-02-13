# Observation - Iteration 13

## Files Examined
- `edgequake_webui/src/components/document/lineage-tree.tsx` - Existing pipeline view (static steps)
- `edgequake_webui/src/components/document/metadata-sidebar.tsx` - Main sidebar layout
- `edgequake_webui/src/hooks/use-lineage.ts` - useDocumentLineage returns DocumentLineageResponse
- `edgequake_webui/src/types/lineage.ts:195-215` - DocumentLineageResponse with chunks, entities, relationships

## Current State
- LineageTree shows pipeline steps (Upload → Extract → Map → Index) but NOT the actual data hierarchy
- DocumentLineageResponse contains chunks (with indices, token counts, entities) and entities (with source_chunks)
- No component visualizes Document → Chunks → Entities hierarchy
- Mission deliverable #4 requires "Document Lineage Tree: Document → PDF → Chunks → Entities (visual hierarchy)"

## Gap
No visual representation of the actual data hierarchy. Users can't see which entities came from which chunks.
