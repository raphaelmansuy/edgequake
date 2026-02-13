# Observation - Iteration 24

## Mission Re-read
Re-read complete mission file (687 lines). Focus: Deliverable #4 "Export Capability: Download complete lineage as JSON/CSV" — WebUI integration.

## Files Examined
- `edgequake_webui/src/components/document/metadata-sidebar.tsx` (120 lines) — Sidebar with collapsible sections, imports
- `edgequake_webui/src/lib/api/edgequake.ts` (lines 1390-1445) — Existing lineage API functions
- `edgequake_webui/src/lib/api/edgequake.ts` (lines 694-696) — PDF download URL pattern using `NEXT_PUBLIC_API_BASE_URL`

## Current State
- Backend export endpoint exists (OODA-22: `/documents/{id}/lineage/export?format=json|csv`)
- No WebUI component calls the export endpoint
- No download buttons for lineage data
- Sidebar has collapsible sections for lineage, metadata, hierarchy, etc.
