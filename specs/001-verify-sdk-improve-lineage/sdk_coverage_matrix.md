# SDK Coverage Matrix

> **Last Updated**: 2026-02-13  
> **Purpose**: Track API endpoint coverage across all 10 EdgeQuake SDKs

## Legend

- ✅ **Implemented & Tested** - Endpoint fully implemented with E2E tests
- ⚠️ **Partial** - Implemented but missing tests or incomplete functionality
- ❌ **Missing** - Not yet implemented
- 🚧 **In Progress** - Currently being worked on (OODA iteration reference)

---

## Coverage Summary

| SDK        | Total Endpoints | Implemented | Tested | Coverage % | Status      |
|------------|-----------------|-------------|--------|------------|-------------|
| Python     | 131             | ~105        | ~85    | ~80%       | ⚠️ Good     |
| TypeScript | 131             | ~118        | ~95    | ~90%       | ⚠️ Excellent|
| Rust       | 131             | ~111        | ~98    | ~85%       | ⚠️ Excellent|
| C#         | 131             | ~78         | ~45    | ~60%       | ⚠️ Fair     |
| Go         | 131             | ~78         | ~50    | ~60%       | ⚠️ Fair     |
| Java       | 131             | ~65         | ~30    | ~50%       | ❌ Needs Work|
| Kotlin     | 131             | ~65         | ~32    | ~50%       | ❌ Needs Work|
| PHP        | 131             | ~72         | ~40    | ~55%       | ⚠️ Fair     |
| Ruby       | 131             | ~85         | ~50    | ~65%       | ⚠️ Good     |
| Swift      | 131             | ~65         | ~28    | ~50%       | ❌ Needs Work|

---

## Detailed Endpoint Coverage

### 1. Health & Observability (4 endpoints)

| Endpoint          | Python | TypeScript | Rust | C# | Go | Java | Kotlin | PHP | Ruby | Swift |
|-------------------|--------|------------|------|----|----|----|--------|-----|------|-------|
| GET /health       | ✅     | ✅         | ✅   | ✅ | ✅ | ✅  | ✅     | ✅  | ✅   | ✅    |
| GET /ready        | ✅     | ✅         | ✅   | ✅ | ✅ | ✅  | ✅     | ✅  | ✅   | ✅    |
| GET /live         | ✅     | ✅         | ✅   | ⚠️ | ⚠️ | ❌  | ❌     | ⚠️  | ⚠️   | ❌    |
| GET /metrics      | ✅     | ✅         | ✅   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |

### 2. Authentication (4 endpoints)

| Endpoint                   | Python | TypeScript | Rust | C# | Go | Java | Kotlin | PHP | Ruby | Swift |
|----------------------------|--------|------------|------|----|----|----|--------|-----|------|-------|
| POST /api/v1/auth/login    | ✅     | ✅         | ✅   | ✅ | ✅ | ⚠️  | ⚠️     | ⚠️  | ✅   | ❌    |
| POST /api/v1/auth/refresh  | ✅     | ✅         | ✅   | ✅ | ✅ | ❌  | ❌     | ❌  | ⚠️   | ❌    |
| POST /api/v1/auth/logout   | ✅     | ✅         | ✅   | ⚠️ | ⚠️ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/auth/me        | ✅     | ✅         | ✅   | ✅ | ✅ | ❌  | ❌     | ❌  | ⚠️   | ❌    |

### 3. Users (3 endpoints)

| Endpoint                       | Python | TypeScript | Rust | C# | Go | Java | Kotlin | PHP | Ruby | Swift |
|--------------------------------|--------|------------|------|----|----|----|--------|-----|------|-------|
| POST /api/v1/users             | ✅     | ✅         | ✅   | ⚠️ | ⚠️ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/users              | ✅     | ✅         | ✅   | ⚠️ | ⚠️ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/users/{user_id}    | ✅     | ✅         | ✅   | ⚠️ | ⚠️ | ❌  | ❌     | ❌  | ❌   | ❌    |
| DELETE /api/v1/users/{user_id} | ✅     | ✅         | ✅   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |

### 4. API Keys (3 endpoints)

| Endpoint                           | Python | TypeScript | Rust | C# | Go | Java | Kotlin | PHP | Ruby | Swift |
|------------------------------------|--------|------------|------|----|----|----|--------|-----|------|-------|
| POST /api/v1/api-keys              | ✅     | ✅         | ✅   | ⚠️ | ⚠️ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/api-keys               | ✅     | ✅         | ✅   | ⚠️ | ⚠️ | ❌  | ❌     | ❌  | ❌   | ❌    |
| DELETE /api/v1/api-keys/{key_id}   | ✅     | ✅         | ✅   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |

### 5. Tenants (5 endpoints)

| Endpoint                             | Python | TypeScript | Rust | C# | Go | Java | Kotlin | PHP | Ruby | Swift |
|--------------------------------------|--------|------------|------|----|----|----|--------|-----|------|-------|
| POST /api/v1/tenants                 | ✅     | ✅         | ✅   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/tenants                  | ✅     | ✅         | ✅   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/tenants/{tenant_id}      | ✅     | ✅         | ✅   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| PUT /api/v1/tenants/{tenant_id}      | ✅     | ✅         | ✅   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| DELETE /api/v1/tenants/{tenant_id}   | ✅     | ✅         | ✅   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |

### 6. Workspaces (12 endpoints)

| Endpoint                                                    | Python | TypeScript | Rust | C# | Go | Java | Kotlin | PHP | Ruby | Swift |
|-------------------------------------------------------------|--------|------------|------|----|----|----|--------|-----|------|-------|
| POST /api/v1/tenants/{tenant_id}/workspaces                 | ✅     | ✅         | ✅   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/tenants/{tenant_id}/workspaces                  | ✅     | ✅         | ✅   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/tenants/{tenant_id}/workspaces/by-slug/{slug}   | ✅     | ✅         | ⚠️   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/workspaces/{workspace_id}                       | ✅     | ✅         | ✅   | ⚠️ | ⚠️ | ❌  | ❌     | ❌  | ❌   | ❌    |
| PUT /api/v1/workspaces/{workspace_id}                       | ✅     | ✅         | ✅   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| DELETE /api/v1/workspaces/{workspace_id}                    | ✅     | ✅         | ✅   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/workspaces/{workspace_id}/stats                 | ✅     | ✅         | ✅   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/workspaces/{workspace_id}/metrics-history       | ⚠️     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| POST /api/v1/workspaces/{workspace_id}/metrics-snapshot     | ⚠️     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| POST /api/v1/workspaces/{workspace_id}/rebuild-embeddings   | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| POST /api/v1/workspaces/{workspace_id}/rebuild-knowledge-graph | ✅  | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| POST /api/v1/workspaces/{workspace_id}/reprocess-documents  | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |

### 7. Documents (20 endpoints)

| Endpoint                                              | Python | TypeScript | Rust | C# | Go | Java | Kotlin | PHP | Ruby | Swift |
|-------------------------------------------------------|--------|------------|------|----|----|----|--------|-----|------|-------|
| POST /api/v1/documents                                | ✅     | ✅         | ✅   | ✅ | ✅ | ✅  | ✅     | ✅  | ✅   | ✅    |
| GET /api/v1/documents                                 | ✅     | ✅         | ✅   | ✅ | ✅ | ✅  | ✅     | ✅  | ✅   | ✅    |
| DELETE /api/v1/documents                              | ✅     | ✅         | ✅   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/documents/track/{track_id}                | ✅     | ✅         | ✅   | ⚠️ | ⚠️ | ❌  | ❌     | ❌  | ⚠️   | ❌    |
| POST /api/v1/documents/upload                         | ✅     | ✅         | ✅   | ✅ | ✅ | ⚠️  | ⚠️     | ⚠️  | ✅   | ⚠️    |
| POST /api/v1/documents/upload/batch                   | ✅     | ✅         | ⚠️   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| POST /api/v1/documents/scan                           | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| POST /api/v1/documents/reprocess                      | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| POST /api/v1/documents/recover-stuck                  | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/documents/{document_id}                   | ✅     | ✅         | ✅   | ✅ | ✅ | ✅  | ✅     | ✅  | ✅   | ✅    |
| DELETE /api/v1/documents/{document_id}                | ✅     | ✅         | ✅   | ✅ | ✅ | ✅  | ✅     | ✅  | ✅   | ✅    |
| GET /api/v1/documents/{document_id}/deletion-impact   | ⚠️     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| POST /api/v1/documents/{document_id}/retry-chunks     | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/documents/{document_id}/failed-chunks     | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/documents/{document_id}/lineage           | ✅     | ✅         | ⚠️   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/documents/{document_id}/metadata          | ✅     | ✅         | ⚠️   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/documents/{document_id}/lineage/export    | ⚠️     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |

### 8. PDF Documents (10 endpoints)

| Endpoint                                        | Python | TypeScript | Rust | C# | Go | Java | Kotlin | PHP | Ruby | Swift |
|-------------------------------------------------|--------|------------|------|----|----|----|--------|-----|------|-------|
| POST /api/v1/documents/pdf                      | ✅     | ✅         | ✅   | ⚠️ | ⚠️ | ❌  | ❌     | ❌  | ⚠️   | ❌    |
| GET /api/v1/documents/pdf                       | ✅     | ✅         | ✅   | ⚠️ | ⚠️ | ❌  | ❌     | ❌  | ⚠️   | ❌    |
| GET /api/v1/documents/pdf/progress/{track_id}   | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| POST /api/v1/documents/pdf/{pdf_id}/retry       | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| DELETE /api/v1/documents/pdf/{pdf_id}/cancel    | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/documents/pdf/{pdf_id}/download     | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/documents/pdf/{pdf_id}/content      | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/documents/pdf/{pdf_id}              | ✅     | ✅         | ✅   | ⚠️ | ⚠️ | ❌  | ❌     | ❌  | ⚠️   | ❌    |
| DELETE /api/v1/documents/pdf/{pdf_id}           | ✅     | ✅         | ✅   | ⚠️ | ⚠️ | ❌  | ❌     | ❌  | ⚠️   | ❌    |

### 9. Query & Chat (4 endpoints)

| Endpoint                               | Python | TypeScript | Rust | C# | Go | Java | Kotlin | PHP | Ruby | Swift |
|----------------------------------------|--------|------------|------|----|----|----|--------|-----|------|-------|
| POST /api/v1/query                     | ✅     | ✅         | ✅   | ✅ | ✅ | ✅  | ✅     | ✅  | ✅   | ✅    |
| POST /api/v1/query/stream              | ✅     | ✅         | ✅   | ⚠️ | ⚠️ | ❌  | ❌     | ❌  | ⚠️   | ❌    |
| POST /api/v1/chat/completions          | ✅     | ✅         | ✅   | ✅ | ✅ | ✅  | ✅     | ✅  | ✅   | ✅    |
| POST /api/v1/chat/completions/stream   | ✅     | ✅         | ✅   | ⚠️ | ⚠️ | ❌  | ❌     | ❌  | ⚠️   | ❌    |

### 10. Conversations (12 endpoints)

| Endpoint                                         | Python | TypeScript | Rust | C# | Go | Java | Kotlin | PHP | Ruby | Swift |
|--------------------------------------------------|--------|------------|------|----|----|----|--------|-----|------|-------|
| GET /api/v1/conversations                        | ✅     | ✅         | ✅   | ⚠️ | ⚠️ | ❌  | ❌     | ❌  | ⚠️   | ❌    |
| POST /api/v1/conversations                       | ✅     | ✅         | ✅   | ⚠️ | ⚠️ | ❌  | ❌     | ❌  | ⚠️   | ❌    |
| POST /api/v1/conversations/import                | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| POST /api/v1/conversations/bulk/delete           | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| POST /api/v1/conversations/bulk/archive          | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| POST /api/v1/conversations/bulk/move             | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/conversations/{id}                   | ✅     | ✅         | ✅   | ⚠️ | ⚠️ | ❌  | ❌     | ❌  | ⚠️   | ❌    |
| PATCH /api/v1/conversations/{id}                 | ✅     | ✅         | ✅   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| DELETE /api/v1/conversations/{id}                | ✅     | ✅         | ✅   | ⚠️ | ⚠️ | ❌  | ❌     | ❌  | ⚠️   | ❌    |
| GET /api/v1/conversations/{id}/messages          | ✅     | ✅         | ✅   | ⚠️ | ⚠️ | ❌  | ❌     | ❌  | ⚠️   | ❌    |
| POST /api/v1/conversations/{id}/messages         | ✅     | ✅         | ✅   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| POST /api/v1/conversations/{id}/share            | ✅     | ✅         | ⚠️   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| DELETE /api/v1/conversations/{id}/share          | ✅     | ✅         | ⚠️   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |

### 11. Messages (2 endpoints)

| Endpoint                              | Python | TypeScript | Rust | C# | Go | Java | Kotlin | PHP | Ruby | Swift |
|---------------------------------------|--------|------------|------|----|----|----|--------|-----|------|-------|
| PATCH /api/v1/messages/{message_id}   | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| DELETE /api/v1/messages/{message_id}  | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |

### 12. Folders (4 endpoints)

| Endpoint                              | Python | TypeScript | Rust | C# | Go | Java | Kotlin | PHP | Ruby | Swift |
|---------------------------------------|--------|------------|------|----|----|----|--------|-----|------|-------|
| GET /api/v1/folders                   | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| POST /api/v1/folders                  | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| PATCH /api/v1/folders/{folder_id}     | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| DELETE /api/v1/folders/{folder_id}    | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |

### 13. Shared Conversations (1 endpoint)

| Endpoint                           | Python | TypeScript | Rust | C# | Go | Java | Kotlin | PHP | Ruby | Swift |
|------------------------------------|--------|------------|------|----|----|----|--------|-----|------|-------|
| GET /api/v1/shared/{share_id}      | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |

### 14. Graph (9 endpoints)

| Endpoint                              | Python | TypeScript | Rust | C# | Go | Java | Kotlin | PHP | Ruby | Swift |
|---------------------------------------|--------|------------|------|----|----|----|--------|-----|------|-------|
| GET /api/v1/graph                     | ✅     | ✅         | ✅   | ✅ | ✅ | ✅  | ✅     | ✅  | ✅   | ✅    |
| GET /api/v1/graph/stream              | ✅     | ✅         | ✅   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/graph/nodes/{node_id}     | ✅     | ✅         | ✅   | ⚠️ | ⚠️ | ❌  | ❌     | ❌  | ⚠️   | ❌    |
| GET /api/v1/graph/nodes/search        | ✅     | ✅         | ✅   | ✅ | ✅ | ⚠️  | ⚠️     | ⚠️  | ✅   | ⚠️    |
| GET /api/v1/graph/labels/search       | ✅     | ✅         | ⚠️   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/graph/labels/popular      | ✅     | ✅         | ⚠️   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| POST /api/v1/graph/degrees/batch      | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |

### 15. Entities (9 endpoints)

| Endpoint                                            | Python | TypeScript | Rust | C# | Go | Java | Kotlin | PHP | Ruby | Swift |
|-----------------------------------------------------|--------|------------|------|----|----|----|--------|-----|------|-------|
| GET /api/v1/graph/entities                          | ✅     | ✅         | ✅   | ⚠️ | ⚠️ | ❌  | ❌     | ❌  | ⚠️   | ❌    |
| POST /api/v1/graph/entities                         | ✅     | ✅         | ✅   | ⚠️ | ⚠️ | ❌  | ❌     | ❌  | ⚠️   | ❌    |
| GET /api/v1/graph/entities/exists                   | ✅     | ✅         | ✅   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| POST /api/v1/graph/entities/merge                   | ✅     | ✅         | ✅   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/graph/entities/{entity_name}            | ✅     | ✅         | ✅   | ⚠️ | ⚠️ | ❌  | ❌     | ❌  | ⚠️   | ❌    |
| PUT /api/v1/graph/entities/{entity_name}            | ✅     | ✅         | ✅   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| DELETE /api/v1/graph/entities/{entity_name}         | ✅     | ✅         | ✅   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/graph/entities/{entity_name}/neighborhood | ✅     | ✅         | ⚠️   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |

### 16. Relationships (5 endpoints)

| Endpoint                                                  | Python | TypeScript | Rust | C# | Go | Java | Kotlin | PHP | Ruby | Swift |
|-----------------------------------------------------------|--------|------------|------|----|----|----|--------|-----|------|-------|
| GET /api/v1/graph/relationships                           | ✅     | ✅         | ✅   | ⚠️ | ⚠️ | ❌  | ❌     | ❌  | ⚠️   | ❌    |
| POST /api/v1/graph/relationships                          | ✅     | ✅         | ✅   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/graph/relationships/{relationship_id}         | ✅     | ✅         | ✅   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| PUT /api/v1/graph/relationships/{relationship_id}         | ✅     | ✅         | ⚠️   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| DELETE /api/v1/graph/relationships/{relationship_id}      | ✅     | ✅         | ⚠️   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |

### 17. Tasks (4 endpoints)

| Endpoint                              | Python | TypeScript | Rust | C# | Go | Java | Kotlin | PHP | Ruby | Swift |
|---------------------------------------|--------|------------|------|----|----|----|--------|-----|------|-------|
| GET /api/v1/tasks/{track_id}          | ✅     | ✅         | ✅   | ⚠️ | ⚠️ | ❌  | ❌     | ❌  | ⚠️   | ❌    |
| GET /api/v1/tasks                     | ✅     | ✅         | ✅   | ⚠️ | ⚠️ | ❌  | ❌     | ❌  | ⚠️   | ❌    |
| POST /api/v1/tasks/{track_id}/cancel  | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| POST /api/v1/tasks/{track_id}/retry   | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |

### 18. Pipeline (3 endpoints)

| Endpoint                              | Python | TypeScript | Rust | C# | Go | Java | Kotlin | PHP | Ruby | Swift |
|---------------------------------------|--------|------------|------|----|----|----|--------|-----|------|-------|
| GET /api/v1/pipeline/status           | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| POST /api/v1/pipeline/cancel          | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/pipeline/queue-metrics    | ⚠️     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |

### 19. Costs (6 endpoints)

| Endpoint                               | Python | TypeScript | Rust | C# | Go | Java | Kotlin | PHP | Ruby | Swift |
|----------------------------------------|--------|------------|------|----|----|----|--------|-----|------|-------|
| GET /api/v1/pipeline/costs/pricing     | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| POST /api/v1/pipeline/costs/estimate   | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/costs/summary              | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/costs/history              | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/costs/budget               | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| PATCH /api/v1/costs/budget             | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |

### 20. Lineage (6 endpoints)

| Endpoint                                          | Python | TypeScript | Rust | C# | Go | Java | Kotlin | PHP | Ruby | Swift |
|---------------------------------------------------|--------|------------|------|----|----|----|--------|-----|------|-------|
| GET /api/v1/lineage/entities/{entity_name}        | ✅     | ✅         | ⚠️   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/lineage/documents/{document_id}       | ✅     | ✅         | ⚠️   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/chunks/{chunk_id}                     | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/chunks/{chunk_id}/lineage             | ⚠️     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/entities/{entity_id}/provenance       | ⚠️     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |

### 21. Settings (3 endpoints)

| Endpoint                                   | Python | TypeScript | Rust | C# | Go | Java | Kotlin | PHP | Ruby | Swift |
|--------------------------------------------|--------|------------|------|----|----|----|--------|-----|------|-------|
| GET /api/v1/settings/provider/status       | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/settings/providers             | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |

### 22. Models (6 endpoints)

| Endpoint                            | Python | TypeScript | Rust | C# | Go | Java | Kotlin | PHP | Ruby | Swift |
|-------------------------------------|--------|------------|------|----|----|----|--------|-----|------|-------|
| GET /api/v1/models                  | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/models/llm              | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/models/embedding        | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/models/health           | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/models/{provider}       | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/v1/models/{provider}/{model} | ✅   | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |

### 23. WebSocket (2 endpoints)

| Endpoint                              | Python | TypeScript | Rust | C# | Go | Java | Kotlin | PHP | Ruby | Swift |
|---------------------------------------|--------|------------|------|----|----|----|--------|-----|------|-------|
| WS /ws/pipeline/progress              | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| WS /ws/progress/{track_id}            | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |

### 24. Ollama Emulation (5 endpoints)

| Endpoint                  | Python | TypeScript | Rust | C# | Go | Java | Kotlin | PHP | Ruby | Swift |
|---------------------------|--------|------------|------|----|----|----|--------|-----|------|-------|
| GET /api/version          | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/tags             | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| GET /api/ps               | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| POST /api/generate        | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |
| POST /api/chat            | ✅     | ✅         | ❌   | ❌ | ❌ | ❌  | ❌     | ❌  | ❌   | ❌    |

---

## Priority Gap Areas

### High Priority (Mission Critical)

1. **Lineage & Metadata (All SDKs)**
   - Python: Missing chunk lineage endpoint tests
   - TypeScript: Full coverage ✅
   - Rust: Missing lineage export, chunk lineage
   - Others: Missing all lineage endpoints

2. **WebSocket Support (Secondary SDKs)**
   - Only Python & TypeScript have WS support
   - Rust, C#, Go, etc.: Need implementation

3. **Streaming (Secondary SDKs)**
   - C#, Go: Partial SSE support
   - Java, Kotlin, PHP, Swift: No streaming

### Medium Priority (Important Features)

1. **Conversations & Folders**
   - Python, TypeScript: Good coverage
   - Rust: Missing folders entirely
   - Others: Partial or missing

2. **Cost Tracking**
   - Python, TypeScript: Full support
   - All others: Missing

3. **Workspace Management**
   - Python, TypeScript: Full support
   - Rust: Partial (missing rebuild endpoints)
   - Others: Minimal or missing

### Low Priority (Nice to Have)

1. **Ollama Emulation**
   - Only Python & TypeScript support
   - Most use cases don't need this

2. **Bulk Operations**
   - Conversations bulk delete/archive/move
   - Only Python & TypeScript support

---

## Next Steps

1. **Phase 1**: Complete Python SDK to 95%+ coverage (baseline reference)
2. **Phase 2**: Bring TypeScript to 95%+ (frontend primary SDK)
3. **Phase 3**: Complete Rust SDK (backend integration SDK)
4. **Phase 4**: Bring secondary SDKs to 80%+ minimum
5. **Phase 5**: Standardize testing across all SDKs

---

**Last Updated**: 2026-02-13  
**Maintained By**: OODA Loop Iterations  
**Review Frequency**: After each iteration
