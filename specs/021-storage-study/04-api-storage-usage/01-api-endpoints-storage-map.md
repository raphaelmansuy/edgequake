# 01 — API Endpoints to Storage Map

> **Spec**: 021-storage-study  
> **File**: 04-api-storage-usage/01-api-endpoints-storage-map.md  
> **Date**: 2026-06-25  
> **Source**: `edgequake-api/src/handlers/`, `edgequake-api/src/routes.rs`

---

## Endpoint Inventory

### Document Endpoints

| Endpoint                          | Method      | Storage READ                                      | Storage WRITE                                                                                        |
| --------------------------------- | ----------- | ------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `/api/v1/documents`               | GET         | `documents` table                                 | —                                                                                                    |
| `/api/v1/documents`               | POST (text) | `documents` (check dup)                           | `documents`, `edgequake_tasks`                                                                       |
| `/api/v1/documents`               | POST (PDF)  | `pdf_documents` (checksum check)                  | `documents`, `pdf_documents`, `edgequake_tasks`                                                      |
| `/api/v1/documents/{id}`          | GET         | `documents`, `pdf_documents`                      | —                                                                                                    |
| `/api/v1/documents/{id}`          | DELETE      | `documents`, `eq_*_kv`, `eq_*_vectors`, AGE graph | `documents` (cascade), `edgequake_tasks`, `eq_*_kv` delete, `eq_*_vectors` delete, AGE detach delete |
| `/api/v1/documents/{id}/status`   | GET         | `documents`, `edgequake_tasks`                    | —                                                                                                    |
| `/api/v1/documents/{id}/markdown` | GET         | `pdf_documents`                                   | —                                                                                                    |
| `/api/v1/documents/scan`          | POST        | filesystem (path-validated)                       | `documents`, `edgequake_tasks`                                                                       |

### Query Endpoints

| Endpoint               | Method | Storage READ                                        | Storage WRITE             |
| ---------------------- | ------ | --------------------------------------------------- | ------------------------- |
| `/api/v1/query`        | POST   | `eq_*_vectors` (or workspace), `eq_*_kv`, AGE graph | `eq_*_kv` (keyword cache) |
| `/api/v1/query/stream` | POST   | Same as above                                       | Same as above             |

### Graph Endpoints

| Endpoint                       | Method        | Storage READ                                                 | Storage WRITE                                    |
| ------------------------------ | ------------- | ------------------------------------------------------------ | ------------------------------------------------ |
| `/api/v1/graph/nodes`          | GET           | AGE graph (Node MATCH)                                       | —                                                |
| `/api/v1/graph/nodes/{id}`     | GET           | AGE graph (Node by node_id)                                  | —                                                |
| `/api/v1/graph/edges`          | GET           | AGE graph (EDGE MATCH)                                       | —                                                |
| `/api/v1/graph/neighbors/{id}` | GET           | AGE graph (k-hop)                                            | —                                                |
| `/api/v1/graph/search`         | GET           | AGE graph (full-text search), `eq_*_vectors` (entity search) | —                                                |
| `/api/v1/graph/stats`          | GET           | AGE graph (COUNT)                                            | `workspace_metrics_history`                      |
| `/api/v1/graph/entities`       | POST (create) | —                                                            | AGE graph (MERGE Node), `eq_*_vectors`           |
| `/api/v1/graph/entities/{id}`  | PUT (update)  | AGE graph                                                    | AGE graph (SET), `eq_*_vectors`                  |
| `/api/v1/graph/entities/{id}`  | DELETE        | AGE graph                                                    | AGE graph (DETACH DELETE), `eq_*_vectors` delete |

### Workspace Endpoints

| Endpoint                        | Method | Storage READ                                                 | Storage WRITE               |
| ------------------------------- | ------ | ------------------------------------------------------------ | --------------------------- |
| `/api/v1/workspaces`            | GET    | `workspaces`, `tenants`                                      | —                           |
| `/api/v1/workspaces`            | POST   | —                                                            | `workspaces`, `memberships` |
| `/api/v1/workspaces/{id}`       | GET    | `workspaces`, `documents`, `eq_*_kv`, AGE graph              | —                           |
| `/api/v1/workspaces/{id}/stats` | GET    | `documents`, `eq_*_kv` (keys_with_suffix), AGE graph (count) | —                           |

### Conversation Endpoints

| Endpoint                              | Method | Storage READ         | Storage WRITE                          |
| ------------------------------------- | ------ | -------------------- | -------------------------------------- |
| `/api/v1/conversations`               | GET    | `conversations`      | —                                      |
| `/api/v1/conversations`               | POST   | —                    | `conversations`                        |
| `/api/v1/conversations/{id}/messages` | GET    | `messages`           | —                                      |
| `/api/v1/conversations/{id}/messages` | POST   | `messages` (history) | `messages`                             |
| `/api/v1/conversations/{id}`          | DELETE | —                    | `conversations` (cascade → `messages`) |

### System Endpoints

| Endpoint          | Method | Storage READ                                                            | Storage WRITE   |
| ----------------- | ------ | ----------------------------------------------------------------------- | --------------- |
| `/health`         | GET    | `documents` (ping), `eq_*_kv` (ping), `eq_*_vectors` (ping), AGE (ping) | —               |
| `/api/v1/config`  | GET    | `server_config`                                                         | —               |
| `/api/v1/config`  | PUT    | —                                                                       | `server_config` |
| `/api/v1/metrics` | GET    | `workspace_metrics_history`                                             | —               |
| `/api/v1/audit`   | GET    | `audit_logs`                                                            | —               |

---

## Health Check Storage Access Pattern

The `/health` endpoint probes all storage systems:

```
GET /health
    |
    +--> kv_storage.ping()       -> SELECT 1 FROM eq_*_kv_stats (O(1))
    +--> vector_storage.ping()   -> SELECT 1 FROM eq_*_vectors LIMIT 1
    +--> graph_storage.node_count() -> SELECT COUNT(*) in AGE (expensive!)
    +--> pdf_storage.ping()      -> SELECT 1 FROM pdf_documents LIMIT 1
    |
    Response: {"status":"healthy","storage_mode":"postgresql",...}
```

> **WARNING (R-SOLID-03)**: `graph_storage.node_count()` in health check performs
> a full graph COUNT scan. This should be replaced by a lightweight ping.

---

## Delete Cascade Pattern

```
DELETE /api/v1/documents/{id}
    |
    +--> 1. Find all chunk KV keys: kv_storage.keys_with_prefix("{id}-")
    +--> 2. Delete chunk vectors: vector_storage.delete(chunk_vector_ids)
    +--> 3. Delete entity vectors (if sole source): vector_storage.delete(entity_ids)
    +--> 4. Delete KV entries: kv_storage.delete(chunk_keys + metadata_key)
    +--> 5. Graph cascade:
    |        For each entity with source_ids containing this doc:
    |          IF sole source: graph.delete_node(entity_id)  + delete vectors
    |          IF multi-source: graph.upsert_node(entity_id, {source_ids: remaining})
    +--> 6. Delete: documents WHERE id = doc_id (cascades to pdf_documents)
    +--> 7. Cancel pending tasks: edgequake_tasks WHERE document_id = doc_id
```

Source: `edgequake-core/src/orchestrator/deletion.rs`
