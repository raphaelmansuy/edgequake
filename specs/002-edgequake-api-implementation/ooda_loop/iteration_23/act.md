# Iteration 23: OpenAPI Spec Update

## Observe
- `edgequake/crates/edgequake-api/src/openapi.rs` only registered ~42 paths out of 110+
- Missing: chat, conversations, folders, pipeline, tasks, costs, tenants, workspaces, lineage, PDF
- All handler functions have `#[utoipa::path]` annotations — just need registration in the spec
- All types have `#[derive(ToSchema)]` — just need registration in schemas

## Act
### Added Paths (~70 new endpoints)
- Chat: `chat_completion`, `chat_completion_stream`
- Conversations: list, create, get, update, delete, list_messages, create_message, bulk_delete
- Folders: list, create, update, delete
- Pipeline: get_pipeline_status, cancel_pipeline, get_queue_metrics
- Tasks: list_tasks, get_task, cancel_task, retry_task
- Costs: get_cost_summary, get_model_pricing, estimate_cost
- Tenants: create, list, get, update, delete
- Workspaces: create, list, get, update, delete, get_stats
- Lineage: get_chunk_detail, get_entity_provenance, get_entity_lineage, get_document_lineage
- PDF: upload, get_status, list, delete, get_progress, get_content

### Added Schemas (~70 types)
- Chat: ChatCompletionRequest, ChatCompletionResponse
- Conversations: 23 types (ConversationResponse, MessageResponse, FolderResponse, etc.)
- Pipeline: 4 types (EnhancedPipelineStatusResponse, etc.)
- Tasks: 6 types (TaskResponse, TaskListResponse, etc.)
- Costs: 11 types (CostSummaryResponse, EstimateCostRequest, etc.)
- Workspaces: 18 types (TenantResponse, WorkspaceResponse, etc.)

### Added Tags (10 new)
Chat, Conversations, Folders, Pipeline, Tasks, Costs, Tenants, Workspaces, Lineage, PDF

## Results
- `cargo build -p edgequake-api`: ✅ Build succeeded, 0 errors
- `cargo test -p edgequake-api --lib`: ✅ 446 tests passed, 0 failures
