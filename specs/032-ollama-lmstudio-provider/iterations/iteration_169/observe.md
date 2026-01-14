# OODA 169: Observe - Workspace Settings and Rebuild Features

## Date: 2026-01-14

## Observation Summary

Analyzing SPEC-032 Focus areas 4 & 5:

- Focus 4: Workspace settings page with LLM/embedding configuration
- Focus 5: Rebuild document extraction + embedding functionality

## Current State

### Workspace Settings Page

| Feature                  | Status | Location                                                                       |
| ------------------------ | ------ | ------------------------------------------------------------------------------ |
| Workspace detail view    | ✅     | [workspace/page.tsx](<edgequake_webui/src/app/(dashboard)/workspace/page.tsx>) |
| LLM model selector       | ✅     | Lines 18-19, 65, 100-105                                                       |
| Embedding model selector | ✅     | Lines 18, 66, 106-112                                                          |
| Edit mode toggle         | ✅     | Lines 63-64                                                                    |
| Save mutation            | ✅     | Lines 115-135                                                                  |
| Provider icons           | ✅     | Lines 44-55                                                                    |
| Workspace stats          | ✅     | Lines 82-93                                                                    |

### Rebuild Embeddings

| Feature                 | Status | Location                                                                                                |
| ----------------------- | ------ | ------------------------------------------------------------------------------------------------------- |
| RebuildEmbeddingsButton | ✅     | [rebuild-embeddings-button.tsx](edgequake_webui/src/components/workspace/rebuild-embeddings-button.tsx) |
| API endpoint            | ✅     | `POST /api/v1/workspaces/{id}/rebuild-embeddings`                                                       |
| Progress tracking       | ✅     | Uses polling or SSE                                                                                     |
| Confirmation dialog     | ✅     | Lines 192-200                                                                                           |
| Card variant            | ✅     | Lines 180-200                                                                                           |

### Deeplinks (Focus 6)

| Route                 | Status | Description                 |
| --------------------- | ------ | --------------------------- |
| `/w/[slug]/settings`  | ✅     | Workspace settings deeplink |
| `/w/[slug]/query`     | ✅     | Query page deeplink         |
| `/w/[slug]/documents` | ❓     | Need to verify              |
| `/w/[slug]/graph`     | ❓     | Need to verify              |

## Gaps Found

1. **No explicit workspace config page header** - Need to ensure workspace name is prominently displayed
2. **Deeplink to /documents and /graph** - Need to verify these routes exist

## Next Step

Orient: Verify all deeplink routes and document any missing features.
