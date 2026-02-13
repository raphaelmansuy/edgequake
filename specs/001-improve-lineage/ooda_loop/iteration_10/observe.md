# Observation - Iteration 10

## Mission Re-read
Re-read `specs/001-improve-lineage.md`. Focus: Deliverable #4 — WebUI types and metadata display.

## Files Examined

- `edgequake_webui/src/types/index.ts` (1172 lines) — `Document` interface missing `document_type`, `sha256_checksum`, `page_count`, `file_size_bytes`
- `edgequake_webui/src/types/lineage.ts` (338 lines) — `ChunkDetail` missing `start_line`/`end_line`; no types for new API endpoints
- `edgequake_webui/src/components/document/source-info-grid.tsx` (88 lines) — Shows basic metadata but not document_type, checksum, page_count
- `edgequake_webui/src/components/document/metadata-sidebar.tsx` (89 lines) — Wrapper with child components
- `edgequake_webui/src/components/document/key-stats.tsx` (100 lines) — Already good

## Gaps

1. TypeScript `Document` interface missing OODA-04 fields
2. `ChunkDetail` missing OODA-07 `start_line`/`end_line` fields
3. No TypeScript types for `DocumentFullLineageResponse` or `ChunkLineageApiResponse`
4. `SourceInfoGrid` not displaying document_type, checksum, page_count
