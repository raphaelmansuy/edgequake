# Observation - Iteration 12

## Files Examined

- `edgequake_webui/src/components/document/metadata-sidebar.tsx` - Main sidebar component, accepts Document prop
- `edgequake_webui/src/components/document/lineage-tree.tsx` - Shows pipeline steps, uses document.lineage
- `edgequake_webui/src/components/document/source-info-grid.tsx` - Shows source metadata (enhanced in OODA-10)
- `edgequake_webui/src/hooks/use-lineage.ts` - Contains useDocumentMetadata hook (added OODA-11)
- `edgequake_webui/src/types/index.ts:80-215` - Document and DocumentLineage interfaces
- `edgequake/crates/edgequake-api/src/handlers/lineage.rs:745-770` - GET /documents/:id/metadata returns raw KV JSON

## Current State

- MetadataSidebar only uses data from Document prop (fetched via getDocument)
- The `/documents/:id/metadata` endpoint returns richer KV-stored metadata but is not consumed by UI
- `useDocumentMetadata` hook exists but is unused
- SourceInfoGrid shows document_type, page_count, sha256_checksum (OODA-10), but only from Document interface
- The KV metadata may contain fields not in the Document type (e.g., pdf extraction details, chunking stats)
- No component shows the enhanced metadata from the new API endpoint

## Gap Identified

MetadataSidebar doesn't fetch or display enhanced metadata from `/documents/:id/metadata`. The hook exists but needs a consumer component.
