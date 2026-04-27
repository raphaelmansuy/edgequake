# Rust SDK gap analysis (`sdks/rust`)

**Canon:** [API-CANON.md](./API-CANON.md)  
**Client:** `sdks/rust/src/client.rs`, resources under `sdks/rust/src/resources/`.

## Summary

| Category | Pre-`feat/sdk` issue | Resolution |
|----------|---------------------|------------|
| Documents | `upload_text` called non-existent `/documents/upload/text` | Use `POST /api/v1/documents` with typed `UploadDocumentRequest` |
| Documents | `status(id)` called non-existent `/documents/{id}/status` | Removed; use `get`, `track`, or PDF progress endpoints |
| Workspaces | `rebuild()` called non-existent `/workspaces/{id}/rebuild` | Removed; use `rebuild_embeddings`, `rebuild_knowledge_graph`, `reprocess_documents` |
| Workspaces | Missing `metrics_snapshot` | Added `trigger_metrics_snapshot` |
| Tenants | Missing list/create workspaces + get-by-slug | Added `list_workspaces`, `create_workspace`, `get_workspace_by_slug` |
| Models | `set_provider` called non-existent `PUT /settings/provider` | Removed |
| Conversations | `pin`/`unpin` called non-existent `/conversations/{id}/pin` | Implemented via `PATCH /conversations/{id}` with `is_pinned` |
| Health | `ready`/`live`/`metrics` assumed JSON | API returns **plain text** (`OK` / Prometheus); use `get_text` |
| Graph | Missing `stream`, `degrees_batch` on graph resource | Added `stream()` + `degrees_batch` delegating to correct path |
| Graph entities | `degrees()` sent raw JSON array instead of `{ "node_ids": [...] }` | Fixed request body + response shape (`Vec<NodeDegree>`) |
| Provenance | Parsed `Vec<ProvenanceRecord>`; API returns `EntityProvenanceResponse` object | New types + `for_entity` return type |
| Upload response | `UploadDocumentResponse` used `id`; API uses `document_id` | `serde(alias)` compatibility |
| PDF | Missing retry/cancel/delete/download | Added methods on `PdfResource` |
| Admin / config / injections | Not exposed | Added `AdminResource`, `ConfigResource`, workspace injection helpers |
| Query / chat | No streaming helpers | Added `stream_execute` / `stream_completions` (`bytes::Bytes` stream, feature `stream`) |
| Conversations | Flat list/detail/message JSON vs paginated + nested `ConversationWithMessagesResponse`; bulk body `ids`; missing `PATCH/DELETE` messages; no `shared` resource | `list` → `PaginatedConversations`; `list_messages` → `PaginatedMessages`; `get` → nested `ConversationDetail`; bulk uses `conversation_ids` + `affected`; `update_message` / `delete_message`; `client.shared().get()` |
| Conversation query params | `list()` took no filters | `ConversationListQuery` + `conversation_list_query_string`; `list_with_query`; `list_messages_with_query` + `ListMessagesQuery`; `bulk_move(..., Option<&str>)` |

## File ↔ route mapping (maintained)

| Resource module | Primary paths |
|-----------------|---------------|
| `auth.rs` | `/api/v1/auth/*` |
| `users.rs` | `/api/v1/users/*` |
| `api_keys.rs` | `/api/v1/api-keys/*` |
| `tenants.rs` | `/api/v1/tenants`, `/api/v1/tenants/{tid}/workspaces*`, `/by-slug/{slug}` |
| `workspaces.rs` | `/api/v1/workspaces/{id}/*` |
| `admin.rs` | `/api/v1/admin/*` |
| `config.rs` | `/api/v1/config/effective` |
| `documents.rs` | `/api/v1/documents/*` (non-PDF) |
| `pdf.rs` | `/api/v1/documents/pdf/*` |
| `query.rs` | `/api/v1/query`, `/api/v1/query/stream` |
| `chat.rs` | `/api/v1/chat/completions`, `/stream` |
| `graph.rs` | `/api/v1/graph`, `/stream`, `/nodes/*`, `/labels/*`, `/degrees/batch` |
| `entities.rs` | `/api/v1/graph/entities/*` |
| `relationships.rs` | `/api/v1/graph/relationships/*` |
| `conversations.rs` | `/api/v1/conversations/*`, bulk, share, `/api/v1/messages/*` |
| `shared.rs` | `/api/v1/shared/{share_id}` |
| `folders.rs` | `/api/v1/folders/*` |
| `tasks.rs` | `/api/v1/tasks/*` |
| `pipeline.rs` | `/api/v1/pipeline/*` |
| `costs.rs` | `/api/v1/costs/*`, `/pipeline/costs/*` |
| `lineage.rs` | `/api/v1/lineage/*`, document lineage export |
| `chunks.rs` | `/api/v1/chunks/*` |
| `provenance.rs` | `/api/v1/entities/{id}/provenance` |
| `settings.rs` | `/api/v1/settings/*` |
| `models.rs` | `/api/v1/models/*` |
| `health.rs` | `/health`, `/ready`, `/live`, `/metrics` |

## Tests

Wiremock integration tests updated to match real paths and response shapes: `sdks/rust/tests/integration_tests.rs`, `error_path_tests.rs`.
