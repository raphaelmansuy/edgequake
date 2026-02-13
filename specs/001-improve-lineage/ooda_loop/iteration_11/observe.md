# Observation - Iteration 11

## Mission Re-read
Re-read `specs/001-improve-lineage.md`. Focus: WebUI API integration (delivering hooks + API functions for new endpoints).

## Files Examined

- `edgequake_webui/src/lib/api/edgequake.ts` (1747 lines) — found getDocumentLineage calling `/documents/:id/lineage` (new endpoint from OODA-07) but typed with old `DocumentLineageResponse`
- `edgequake_webui/src/hooks/use-lineage.ts` — 4 hooks, no hook for full lineage or metadata
- `edgequake_webui/src/components/lineage/lineage-explorer.tsx` (534 lines) — consumes `useDocumentLineage`, accesses `.entities`, `.chunks`, `.relationships` — expects old response shape

## Issue Found

Changing `getDocumentLineage` return type broke `lineage-explorer.tsx` (23 type errors). The old code expected `DocumentLineageResponse` shape with `.entities`, `.chunks`, `.relationships` at top level, but new `DocumentFullLineageResponse` nests them under `.lineage`.

## Resolution

- Reverted `getDocumentLineage` to use old endpoint `/lineage/documents/:id` with old return type — preserves existing component compatibility
- Added separate `getDocumentFullLineage` function for new `/documents/:id/lineage` endpoint
- Added `getDocumentMetadata` function for `/documents/:id/metadata`
- Added corresponding hooks: `useDocumentFullLineage`, `useDocumentMetadata`
