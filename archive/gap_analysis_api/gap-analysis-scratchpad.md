# Gap Analysis - Working Notes

## Last Updated: 2024-12-24

## Current Phase: analysis

## Current File: gap_analysis_api/gap-analysis-scratchpad.md

### Progress

- Source UI components analyzed: 28/28
- Target UI components analyzed: 25/28
- API endpoints mapped: 38/38
- Features mapped: 45/45

### Component Registry

| ID   | Component/Feature     | Source Status | Target Status | Gap Type     |
| ---- | --------------------- | ------------- | ------------- | ------------ |
| C001 | GraphViewer           | ✅ complete   | ✅ complete   | -            |
| C002 | GraphControls         | ✅ complete   | ✅ complete   | -            |
| C003 | GraphLabels           | ✅ complete   | ✅ complete   | -            |
| C004 | GraphSearch           | ✅ complete   | ✅ complete   | -            |
| C005 | ZoomControl           | ✅ complete   | ✅ complete   | -            |
| C006 | LayoutsControl        | ✅ complete   | ✅ complete   | -            |
| C007 | Legend                | ✅ complete   | ✅ complete   | -            |
| C008 | PropertiesView        | ✅ complete   | ✅ complete   | -            |
| C009 | PropertyEditDialog    | ✅ complete   | ⚠️ partial    | API-BREAKING |
| C010 | MergeDialog           | ✅ complete   | ⚠️ partial    | API-BREAKING |
| C011 | DocumentManager       | ✅ complete   | ✅ complete   | -            |
| C012 | UploadDocumentsDialog | ✅ complete   | ✅ complete   | -            |
| C013 | DeleteDocumentsDialog | ✅ complete   | ✅ complete   | -            |
| C014 | PipelineStatusDialog  | ✅ complete   | ⚠️ partial    | API-BREAKING |
| C015 | ClearDocumentsDialog  | ✅ complete   | ❌ missing    | MISSING      |
| C016 | ChatMessage           | ✅ complete   | ✅ complete   | -            |
| C017 | QuerySettings         | ✅ complete   | ✅ complete   | -            |
| C018 | TenantSelector        | ✅ complete   | ✅ complete   | -            |
| C019 | AppSettings           | ✅ complete   | ✅ complete   | -            |
| C020 | LanguageToggle        | ✅ complete   | ✅ complete   | -            |
| C021 | ThemeToggle           | ✅ complete   | ✅ complete   | -            |
| C022 | ScanNewDocuments      | ✅ complete   | ❌ missing    | MISSING      |
| C023 | ReprocessFailedDocs   | ✅ complete   | ❌ missing    | MISSING      |
| C024 | ResetDocumentStatus   | ✅ complete   | ❌ missing    | MISSING      |
| C025 | EntityEditDialog      | ✅ complete   | ⚠️ partial    | API-BREAKING |
| C026 | RelationEditDialog    | ✅ complete   | ⚠️ partial    | API-BREAKING |
| C027 | TrackStatusView       | ⚠️ partial    | ✅ complete   | -            |
| C028 | BatchUploadProgress   | ✅ complete   | ✅ complete   | -            |

### API Mapping Registry

| ID   | Old Endpoint                      | New Endpoint                         | Breaking Changes                 |
| ---- | --------------------------------- | ------------------------------------ | -------------------------------- |
| A001 | GET /health                       | GET /health                          | None                             |
| A002 | POST /query                       | POST /api/v1/query                   | URL prefix added                 |
| A003 | POST /query/stream                | POST /api/v1/query/stream            | URL prefix added                 |
| A004 | GET /documents                    | GET /api/v1/documents                | Pagination schema changed        |
| A005 | POST /documents/text              | POST /api/v1/documents               | Body format changed              |
| A006 | POST /documents/upload            | POST /api/v1/documents/upload        | Response schema changed          |
| A007 | GET /documents/pipeline_status    | GET /api/v1/tasks                    | Endpoint renamed, schema changed |
| A008 | POST /documents/scan              | POST /api/v1/documents/scan          | Same                             |
| A009 | POST /documents/reprocess_failed  | POST /api/v1/documents/reprocess     | Endpoint renamed                 |
| A010 | DELETE /documents                 | DELETE /api/v1/documents             | Same                             |
| A011 | DELETE /documents/delete_document | DELETE /api/v1/documents/{id}        | RESTful path change              |
| A012 | GET /graphs                       | GET /api/v1/graph                    | Endpoint renamed                 |
| A013 | GET /graph/label/list             | GET /api/v1/graph/labels             | Endpoint renamed                 |
| A014 | GET /graph/label/popular          | GET /api/v1/graph/labels/popular     | Same                             |
| A015 | GET /graph/label/search           | GET /api/v1/graph/labels/search      | Same                             |
| A016 | POST /graph/entity/edit           | PUT /api/v1/graph/entities/{name}    | Method + path changed            |
| A017 | POST /graph/relation/edit         | PUT /api/v1/graph/relationships/{id} | Method + path changed            |
| A018 | GET /graph/entity/exists          | GET /api/v1/graph/entities/exists    | Same                             |
| A019 | POST /login                       | POST /api/v1/auth/login              | URL prefix added                 |
| A020 | GET /auth-status                  | GET /api/v1/auth/me                  | Endpoint renamed                 |
| A021 | GET /api/v1/tenants               | GET /api/v1/tenants                  | Same                             |
| A022 | GET /api/v1/knowledge-bases       | GET /api/v1/tenants/{id}/workspaces  | Renamed to workspaces            |
| A023 | POST /documents/cancel_pipeline   | POST /api/v1/pipeline/cancel         | Endpoint moved                   |
| A024 | GET /documents/track_status/{id}  | GET /api/v1/documents/track/{id}     | Path changed                     |
| A025 | POST /documents/reset_status      | N/A                                  | Not implemented in new API       |
| A026 | GET /documents/status_counts      | GET /api/v1/documents?page_size=1    | Merged into list response        |
| A027 | GET /documents/scan-progress      | N/A                                  | Not implemented (use tasks)      |
| A028 | POST /documents/clear_cache       | N/A                                  | Not implemented in new API       |
| A029 | GET /api/chat                     | GET /api/chat                        | Ollama emulation - same          |
| A030 | POST /api/chat                    | POST /api/chat                       | Ollama emulation - same          |
| A031 | POST /api/generate                | POST /api/generate                   | Ollama emulation - same          |
| A032 | GET /api/tags                     | GET /api/tags                        | Ollama emulation - same          |
| A033 | GET /api/ps                       | GET /api/ps                          | Ollama emulation - same          |
| A034 | GET /api/version                  | GET /api/version                     | Ollama emulation - same          |
| A035 | POST /api/v1/entities/merge       | POST /api/v1/graph/entities/merge    | Path changed                     |
| A036 | GET /api/v1/tasks                 | GET /api/v1/tasks                    | New endpoint                     |
| A037 | POST /api/v1/tasks/{id}/cancel    | POST /api/v1/tasks/{id}/cancel       | New endpoint                     |
| A038 | POST /api/v1/tasks/{id}/retry     | POST /api/v1/tasks/{id}/retry        | New endpoint                     |

### Completed

- Component inventory: ✅ Complete
- API mapping: ✅ Complete
- Gap identification: ✅ Complete

### Findings

#### Parity Achieved

- GraphViewer: `lightrag_webui/src/features/GraphViewer.tsx` ↔ `edgequake_webui/src/components/graph/graph-viewer.tsx`
- DocumentManager: `lightrag_webui/src/features/DocumentManager.tsx` ↔ `edgequake_webui/src/components/documents/document-manager.tsx`
- QueryInterface: `lightrag_webui/src/features/RetrievalTesting.tsx` ↔ `edgequake_webui/src/components/query/query-interface.tsx`
- TenantSelector: `lightrag_webui/src/components/TenantSelector.tsx` ↔ `edgequake_webui/src/components/layout/tenant-selector.tsx`

#### Gaps Identified

- [GAP-UI-001]: Scan New Documents - P1 - Button and dialog to scan input directory for new documents
- [GAP-UI-002]: Reprocess Failed Documents - P1 - Button and dialog to retry failed document processing
- [GAP-UI-003]: Reset Document Status - P2 - Ability to reset document status to pending/failed
- [GAP-UI-004]: Clear Cache - P3 - Button to clear LLM cache
- [GAP-UI-005]: Entity Edit - Rename Flow - P1 - Entity rename with merge conflict handling
- [GAP-UI-006]: Relation Edit Dialog - P2 - Full relation property editing
- [GAP-UI-007]: Pipeline Progress Messages - P2 - Real-time pipeline progress messages
- [GAP-UI-008]: Scan Progress Indicator - P2 - Progress bar for directory scanning
- [GAP-UI-009]: Clear Documents Dialog - P2 - Confirmation dialog for clearing all documents
- [GAP-UI-010]: Document Status Filter - P1 - Already implemented but needs status_counts integration

#### API Breaking Changes

- [API-001]: Documents pagination - Response structure changed from `{statuses: {...}}` to `{documents: [...], pagination: {...}}`
- [API-002]: Pipeline status - Moved from `/documents/pipeline_status` to task-based `/api/v1/tasks`
- [API-003]: Entity edit - Changed from POST to PUT with RESTful path
- [API-004]: Relation edit - Changed from POST to PUT with RESTful path
- [API-005]: Knowledge bases → Workspaces - API path and naming changed
- [API-006]: Track status path - Changed from `/documents/track_status/{id}` to `/documents/track/{id}`
- [API-007]: Document delete - Changed from body-based delete to RESTful path-based delete

#### Target Exceeds Source

- Task management API: Full task lifecycle with cancel/retry per task
- Metrics endpoint: Prometheus-compatible metrics
- Enhanced error handling: Detailed error types with suggestions
- Workspace stats: Per-workspace statistics endpoint
- Batch file upload: Multi-file upload in single request

#### Ambiguous/Needs Clarification

- Scan progress: Should we implement WebSocket for real-time progress?
- Cache clear: Is this needed in new architecture with different caching?
- Reset status: Is this needed with task-based retry functionality?

### Pending Actions

- [x] Create component parity matrix
- [x] Create API migration guide
- [x] Create UI gap analysis report
- [x] Create migration roadmap
- [ ] Implement GAP-UI-001: Scan documents button
- [ ] Implement GAP-UI-002: Reprocess failed button
- [ ] Implement GAP-UI-009: Clear documents dialog
- [ ] Update entity edit to use new API
- [ ] Update relation edit to use new API
- [ ] Add pipeline progress integration
