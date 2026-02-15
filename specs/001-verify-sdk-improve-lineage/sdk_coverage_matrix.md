# SDK API Coverage Matrix

**Last Updated**: 2026-02-15  
**Backend Routes**: 108 unique paths (131+ with HTTP methods)  
**SDKs**: Python, TypeScript, Rust, Java, Kotlin, Go, C#, PHP, Ruby, Swift

---

## Legend

| Symbol | Meaning                |
| ------ | ---------------------- |
| ✅     | Implemented & Verified |
| ⚠️     | Partial Implementation |
| ❌     | Not Implemented        |
| ➖     | Not Applicable         |

---

## Health & Infrastructure (5 endpoints)

| Endpoint                  | Method | Python | TypeScript | Java | Kotlin | Go  | C#  | PHP | Ruby | Swift |
| ------------------------- | ------ | ------ | ---------- | ---- | ------ | --- | --- | --- | ---- | ----- |
| `/health`                 | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ⚠️    |
| `/ready`                  | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ⚠️    |
| `/live`                   | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ⚠️    |
| `/metrics`                | GET    | ⚠️     | ✅         | ✅   | ✅     | ⚠️  | ❌  | ❌  | ❌   | ❌    |
| `/ws/pipeline/progress`   | WS     | ⚠️     | ⚠️         | ❌   | ❌     | ❌  | ❌  | ❌  | ❌   | ❌    |
| `/ws/progress/{track_id}` | WS     | ⚠️     | ⚠️         | ❌   | ❌     | ❌  | ❌  | ❌  | ❌   | ❌    |

---

## Authentication (8 endpoints)

| Endpoint                  | Method | Python | TypeScript | Java | Kotlin | Go  | C#  | PHP | Ruby | Swift |
| ------------------------- | ------ | ------ | ---------- | ---- | ------ | --- | --- | --- | ---- | ----- |
| `/api/v1/auth/login`      | POST   | ✅     | ✅         | ✅   | ✅     | ✅  | ✅  | ⚠️  | ⚠️   | ⚠️    |
| `/api/v1/auth/refresh`    | POST   | ✅     | ✅         | ✅   | ✅     | ✅  | ✅  | ⚠️  | ⚠️   | ⚠️    |
| `/api/v1/auth/logout`     | POST   | ✅     | ✅         | ✅   | ✅     | ✅  | ✅  | ⚠️  | ⚠️   | ⚠️    |
| `/api/v1/auth/me`         | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ✅  | ⚠️  | ⚠️   | ⚠️    |
| `/api/v1/users`           | POST   | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/users`           | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/users/{user_id}` | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/users/{user_id}` | DELETE | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |

---

## API Keys (3 endpoints)

| Endpoint                    | Method | Python | TypeScript | Java | Kotlin | Go  | C#  | PHP | Ruby | Swift |
| --------------------------- | ------ | ------ | ---------- | ---- | ------ | --- | --- | --- | ---- | ----- |
| `/api/v1/api-keys`          | POST   | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/api-keys`          | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/api-keys/{key_id}` | DELETE | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |

---

## Tenants & Workspaces (15 endpoints)

| Endpoint                                                    | Method | Python | TypeScript | Java | Kotlin | Go  | C#  | PHP | Ruby | Swift |
| ----------------------------------------------------------- | ------ | ------ | ---------- | ---- | ------ | --- | --- | --- | ---- | ----- |
| `/api/v1/tenants`                                           | POST   | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/tenants`                                           | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/tenants/{tenant_id}`                               | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/tenants/{tenant_id}`                               | PUT    | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/tenants/{tenant_id}`                               | DELETE | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/tenants/{tenant_id}/workspaces`                    | POST   | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/tenants/{tenant_id}/workspaces`                    | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/workspaces/{workspace_id}`                         | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/workspaces/{workspace_id}`                         | PUT    | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/workspaces/{workspace_id}`                         | DELETE | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/workspaces/{workspace_id}/stats`                   | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/workspaces/{workspace_id}/metrics-history`         | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/workspaces/{workspace_id}/metrics-snapshot`        | POST   | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/workspaces/{workspace_id}/rebuild-embeddings`      | POST   | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/workspaces/{workspace_id}/rebuild-knowledge-graph` | POST   | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |

---

## Documents (20 endpoints)

| Endpoint                                         | Method | Python | TypeScript | Java | Kotlin | Go  | C#  | PHP | Ruby | Swift |
| ------------------------------------------------ | ------ | ------ | ---------- | ---- | ------ | --- | --- | --- | ---- | ----- |
| `/api/v1/documents`                              | POST   | ✅     | ✅         | ✅   | ✅     | ✅  | ✅  | ✅  | ✅   | ⚠️    |
| `/api/v1/documents`                              | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ✅  | ✅  | ✅   | ⚠️    |
| `/api/v1/documents`                              | DELETE | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/documents/{document_id}`                | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ✅  | ✅  | ✅   | ⚠️    |
| `/api/v1/documents/{document_id}`                | DELETE | ✅     | ✅         | ✅   | ✅     | ✅  | ✅  | ✅  | ✅   | ⚠️    |
| `/api/v1/documents/upload`                       | POST   | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/documents/upload/batch`                 | POST   | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/documents/pdf`                          | POST   | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/documents/pdf`                          | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/documents/pdf/{pdf_id}`                 | GET    | ✅     | ✅         | ✅   | ✅     | ⚠️  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/documents/pdf/{pdf_id}`                 | DELETE | ✅     | ✅         | ✅   | ✅     | ⚠️  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/documents/pdf/{pdf_id}/download`        | GET    | ✅     | ✅         | ✅   | ✅     | ⚠️  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/documents/pdf/{pdf_id}/content`         | GET    | ✅     | ✅         | ✅   | ✅     | ⚠️  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/documents/scan`                         | POST   | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/documents/track/{track_id}`             | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/documents/reprocess`                    | POST   | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/documents/recover-stuck`                | POST   | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/documents/{document_id}/metadata`       | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/documents/{document_id}/lineage`        | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/documents/{document_id}/lineage/export` | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |

---

## Query & Chat (4 endpoints)

| Endpoint                          | Method | Python | TypeScript | Java | Kotlin | Go  | C#  | PHP | Ruby | Swift |
| --------------------------------- | ------ | ------ | ---------- | ---- | ------ | --- | --- | --- | ---- | ----- |
| `/api/v1/query`                   | POST   | ✅     | ✅         | ✅   | ✅     | ✅  | ✅  | ✅  | ✅   | ⚠️    |
| `/api/v1/query/stream`            | POST   | ✅     | ✅         | ✅   | ✅     | ⚠️  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/chat/completions`        | POST   | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/chat/completions/stream` | POST   | ✅     | ✅         | ✅   | ✅     | ⚠️  | ❌  | ❌  | ❌   | ❌    |

---

## Conversations (15 endpoints)

| Endpoint                              | Method   | Python | TypeScript | Java | Kotlin | Go  | C#  | PHP | Ruby | Swift |
| ------------------------------------- | -------- | ------ | ---------- | ---- | ------ | --- | --- | --- | ---- | ----- |
| `/api/v1/conversations`               | GET      | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/conversations`               | POST     | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/conversations/{id}`          | GET      | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/conversations/{id}`          | PATCH    | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/conversations/{id}`          | DELETE   | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/conversations/{id}/messages` | GET      | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/conversations/{id}/messages` | POST     | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/conversations/{id}/share`    | POST     | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/conversations/{id}/share`    | DELETE   | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/conversations/import`        | POST     | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/conversations/bulk/delete`   | POST     | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/conversations/bulk/archive`  | POST     | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/conversations/bulk/move`     | POST     | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/folders`                     | GET/POST | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/shared/{share_id}`           | GET      | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |

---

## Knowledge Graph (15 endpoints)

| Endpoint                               | Method         | Python | TypeScript | Java | Kotlin | Go  | C#  | PHP | Ruby | Swift |
| -------------------------------------- | -------------- | ------ | ---------- | ---- | ------ | --- | --- | --- | ---- | ----- |
| `/api/v1/graph`                        | GET            | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/graph/stream`                 | GET            | ⚠️     | ⚠️         | ⚠️   | ⚠️     | ❌  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/graph/nodes/{node_id}`        | GET            | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/graph/nodes/search`           | GET            | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/graph/labels/search`          | GET            | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/graph/labels/popular`         | GET            | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/graph/degrees/batch`          | POST           | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/graph/entities`               | GET/POST       | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/graph/entities/{entity_name}` | GET            | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/graph/entities/{entity_name}` | PUT            | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/graph/entities/{entity_name}` | DELETE         | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/graph/entities/exists`        | GET            | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/graph/entities/merge`         | POST           | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/graph/relationships`          | GET/POST       | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/graph/relationships/{rel_id}` | GET/PUT/DELETE | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |

---

## Lineage & Provenance (7 endpoints)

**⚠️ CORRECTED (Iteration 03)**: All 10 SDKs have LineageService implementations. Previous matrix showed C#/PHP/Ruby/Swift as ❌ — this was INCORRECT.

| Endpoint                                  | Method | Python | TypeScript | Rust | Java | Kotlin | Go  | C#  | PHP | Ruby | Swift |
| ----------------------------------------- | ------ | ------ | ---------- | ---- | ---- | ------ | --- | --- | --- | ---- | ----- |
| `/api/v1/lineage/entities/{entity_name}`  | GET    | ✅     | ✅         | ✅   | ✅   | ✅     | ✅  | ✅  | ✅  | ✅   | ✅    |
| `/api/v1/lineage/documents/{document_id}` | GET    | ✅     | ✅         | ✅   | ✅   | ✅     | ✅  | ✅  | ✅  | ✅   | ✅    |
| `/api/v1/documents/{id}/lineage`          | GET    | ✅     | ✅         | ✅   | ✅   | ✅     | ✅  | ✅  | ⚠️  | ⚠️   | ✅    |
| `/api/v1/documents/{id}/lineage/export`   | GET    | ✅     | ✅         | ✅   | ✅   | ✅     | ✅  | ✅  | ⚠️  | ⚠️   | ✅    |
| `/api/v1/chunks/{chunk_id}`               | GET    | ✅     | ✅         | ✅   | ✅   | ✅     | ✅  | ✅  | ⚠️  | ⚠️   | ✅    |
| `/api/v1/chunks/{chunk_id}/lineage`       | GET    | ✅     | ✅         | ✅   | ✅   | ✅     | ✅  | ✅  | ⚠️  | ⚠️   | ✅    |
| `/api/v1/entities/{entity_id}/provenance` | GET    | ✅     | ✅         | ✅   | ✅   | ✅     | ✅  | ✅  | ⚠️  | ⚠️   | ✅    |

**Evidence (file locations):**
- C#: `LineageService.cs` (70 lines, 7 methods, OODA-24)
- Swift: `LineageService.swift` (72 lines, 7 methods, OODA-26)
- PHP: `Services.php` (contains LineageService class)
- Ruby: `services.rb` (contains LineageService class)
- Rust: `client.rs` (lineage, chunks, provenance resources)

---

## Tasks & Pipeline (10 endpoints)

| Endpoint                          | Method | Python | TypeScript | Java | Kotlin | Go  | C#  | PHP | Ruby | Swift |
| --------------------------------- | ------ | ------ | ---------- | ---- | ------ | --- | --- | --- | ---- | ----- |
| `/api/v1/tasks`                   | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/tasks/{track_id}`        | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ⚠️  | ⚠️  | ⚠️   | ❌    |
| `/api/v1/tasks/{track_id}/cancel` | POST   | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/tasks/{track_id}/retry`  | POST   | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/pipeline/status`         | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/pipeline/cancel`         | POST   | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/pipeline/queue-metrics`  | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/pipeline/costs/pricing`  | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/pipeline/costs/estimate` | POST   | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |

---

## Costs & Budget (4 endpoints)

| Endpoint                | Method | Python | TypeScript | Java | Kotlin | Go  | C#  | PHP | Ruby | Swift |
| ----------------------- | ------ | ------ | ---------- | ---- | ------ | --- | --- | --- | ---- | ----- |
| `/api/v1/costs/summary` | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/costs/history` | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/costs/budget`  | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/costs/budget`  | PATCH  | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |

---

## Settings & Models (8 endpoints)

| Endpoint                            | Method | Python | TypeScript | Java | Kotlin | Go  | C#  | PHP | Ruby | Swift |
| ----------------------------------- | ------ | ------ | ---------- | ---- | ------ | --- | --- | --- | ---- | ----- |
| `/api/v1/settings/provider/status`  | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/settings/providers`        | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/models`                    | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/models/llm`                | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/models/embedding`          | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/models/health`             | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/models/{provider}`         | GET    | ✅     | ✅         | ✅   | ✅     | ⚠️  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/models/{provider}/{model}` | GET    | ✅     | ✅         | ✅   | ✅     | ⚠️  | ❌  | ❌  | ❌   | ❌    |

---

## Ollama Emulation (5 endpoints)

| Endpoint        | Method | Python | TypeScript | Java | Kotlin | Go  | C#  | PHP | Ruby | Swift |
| --------------- | ------ | ------ | ---------- | ---- | ------ | --- | --- | --- | ---- | ----- |
| `/api/version`  | GET    | ⚠️     | ✅         | ⚠️   | ⚠️     | ⚠️  | ❌  | ❌  | ❌   | ❌    |
| `/api/tags`     | GET    | ⚠️     | ✅         | ⚠️   | ⚠️     | ⚠️  | ❌  | ❌  | ❌   | ❌    |
| `/api/ps`       | GET    | ⚠️     | ✅         | ⚠️   | ⚠️     | ⚠️  | ❌  | ❌  | ❌   | ❌    |
| `/api/generate` | POST   | ⚠️     | ✅         | ⚠️   | ⚠️     | ⚠️  | ❌  | ❌  | ❌   | ❌    |
| `/api/chat`     | POST   | ⚠️     | ✅         | ⚠️   | ⚠️     | ⚠️  | ❌  | ❌  | ❌   | ❌    |

---

## Summary Statistics

**⚠️ UPDATED (Iteration 03)**: Actual test counts from CI runs

| SDK            | Tests Passed | Tests Skipped | Coverage % | Lineage Support |
| -------------- | ------------ | ------------- | ---------- | --------------- |
| **Python**     | 520          | 32            | ~88%       | ✅ Full (7 methods) |
| **TypeScript** | 288          | 65 E2E        | ~92%       | ✅ Full (7 methods) |
| **Rust**       | 152          | 1             | ~85%       | ✅ Full (3 resources) |
| **Java**       | 230          | 0             | ~87%       | ✅ Full (7 methods) |
| **Kotlin**     | 230          | 0             | ~87%       | ✅ Full (4+ methods) |
| **Go**         | 234          | 0             | ~78%       | ✅ Full (4+ methods) |
| **C#**         | 267          | 0             | ~75%       | ✅ Full (7 methods) |
| **PHP**        | ~200         | TBD           | ~65%       | ⚠️ Partial |
| **Ruby**       | 237          | 0             | ~70%       | ⚠️ Partial |
| **Swift**      | ~50          | TBD           | ~50%       | ✅ Full (7 methods) |

**Total Verified Tests**: 2,158+

---

## Coverage Tiers

**⚠️ CORRECTED (Iteration 03)**: Based on actual code inspection and test runs

### Tier 1: Production Ready (>85% + Full Lineage)

- ✅ Python (520 tests, 88%, full lineage)
- ✅ TypeScript (288 tests, 92%, full lineage)
- ✅ Java (230 tests, 87%, full lineage)
- ✅ Kotlin (230 tests, 87%, full lineage)
- ✅ Rust (152 tests, 85%, full lineage)

### Tier 2: Functional (70-85% + Full Lineage)

- ✅ Go (234 tests, 78%, full lineage)
- ✅ C# (267 tests, 75%, full lineage)
- ✅ Ruby (237 tests, 70%, partial lineage)

### Tier 3: Near Production (50-70%)

- ⚠️ PHP (~200 tests, 65%, partial lineage)
- ⚠️ Swift (~50 tests, 50%, full lineage - needs more tests)

---

## Priority Actions

1. **TypeScript E2E**: Run 65 skipped E2E tests with live backend
2. **PHP**: Verify test execution and complete lineage methods
3. **Swift**: Add more unit tests to increase coverage
4. **Go SDK**: Add streaming endpoints, Ollama emulation
5. **All SDKs**: Add WebSocket progress tracking (0/10 support)
