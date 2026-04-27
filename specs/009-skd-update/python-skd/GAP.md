# Python SDK gap analysis (`sdks/python`)

**Canon:** [API-CANON.md](../API-CANON.md)

## Summary

| Area | Issue | Resolution |
|------|-------|------------|
| Provenance | `GET /entities/{id}/provenance` returns **`EntityProvenanceResponse`** JSON object | Added `EntityProvenanceResponse` model; `provenance.get()` returns it |
| Admin | No client surface for `/api/v1/admin/*` | Added `client.admin` with quota + server defaults |
| Config | No `GET /config/effective` | Added `client.config.effective()` |
| Workspaces | Knowledge injection endpoints missing | Added `put_injection`, `put_injection_file`, `list_injections`, `get_injection`, `update_injection`, `delete_injection` |
| Async parity | `AsyncWorkspacesResource` missing several methods vs sync | Extended async class to mirror sync (metrics, rebuilds, injections, etc.) |
| Async costs | Missing `update_budget` | Added |
| Conversations bulk | Request used `ids` instead of API `conversation_ids`; responses used `deleted_count` vs `affected` | Fixed; import/share DTOs aligned; `ConversationDetail` accepts nested `conversation` wrapper; async parity for bulk/messages |
| Conversations list/messages | Legacy `page`/`page_size` and flat list returns | `ConversationListParams` / `ListMessagesParams`; `PaginatedConversations` / `PaginatedMessages`; legacy kwargs map to `filter[folder_id]` + `limit` |
| Message PATCH | `MessageUpdate` subset | Expanded to `tokens_used`, `duration_ms`, `thinking_time_ms`, `context`, `is_error` (+ `metadata`) |

## Already aligned (reference)

- Text upload: `POST /api/v1/documents` (`documents.upload`) — correct.
- PDF sub-resource: matches `/documents/pdf/*` family.
- Tenants: `tenants` module includes workspace CRUD + by-slug.
- Conversations: pin handled via `ConversationUpdate.is_pinned` (no fake `/pin` routes).

## Cross-reference

- Node/TypeScript: [../node-js/GAP.md](../node-js/GAP.md)  
- Rust: [../RUST-SDK-GAP.md](../RUST-SDK-GAP.md)
