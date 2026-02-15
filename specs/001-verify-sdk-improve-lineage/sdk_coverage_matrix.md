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

| Endpoint                                  | Method | Python | TypeScript | Java | Kotlin | Go  | C#  | PHP | Ruby | Swift |
| ----------------------------------------- | ------ | ------ | ---------- | ---- | ------ | --- | --- | --- | ---- | ----- |
| `/api/v1/lineage/entities/{entity_name}`  | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/lineage/documents/{document_id}` | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/chunks/{chunk_id}`               | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/chunks/{chunk_id}/lineage`       | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |
| `/api/v1/entities/{entity_id}/provenance` | GET    | ✅     | ✅         | ✅   | ✅     | ✅  | ❌  | ❌  | ❌   | ❌    |

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

| SDK            | Total ✅ | Total ⚠️ | Total ❌ | Coverage % |
| -------------- | -------- | -------- | -------- | ---------- |
| **Python**     | 95+      | 10       | 3        | ~88%       |
| **TypeScript** | 100+     | 5        | 3        | ~92%       |
| **Java**       | 95+      | 8        | 5        | ~87%       |
| **Kotlin**     | 95+      | 8        | 5        | ~87%       |
| **Go**         | 85+      | 15       | 8        | ~78%       |
| **C#**         | 25       | 15       | 68       | ~23%       |
| **PHP**        | 15       | 20       | 73       | ~14%       |
| **Ruby**       | 15       | 20       | 73       | ~14%       |
| **Swift**      | 5        | 10       | 93       | ~5%        |

---

## Coverage Tiers

### Tier 1: Production Ready (>85%)

- ✅ Python (88%)
- ✅ TypeScript (92%)
- ✅ Java (87%)
- ✅ Kotlin (87%)

### Tier 2: Functional (60-85%)

- ⚠️ Go (78%)

### Tier 3: Basic (20-60%)

- ⚠️ C# (23%)

### Tier 4: Minimal (<20%)

- ❌ PHP (14%)
- ❌ Ruby (14%)
- ❌ Swift (5%)

---

## Priority Actions

1. **Go SDK**: Add streaming endpoints, Ollama emulation
2. **C# SDK**: Expand beyond basic CRUD to lineage, costs
3. **PHP SDK**: Add lineage, pipeline, costs APIs
4. **Ruby SDK**: Add lineage, pipeline, costs APIs
5. **Swift SDK**: Major expansion needed - basic framework only
