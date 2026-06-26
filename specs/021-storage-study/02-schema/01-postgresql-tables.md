# 01 — PostgreSQL Relational Tables

> **Spec**: 021-storage-study  
> **File**: 02-schema/01-postgresql-tables.md  
> **Date**: 2026-06-25  
> **Source**: `edgequake/migrations/001_init_database.sql` through `038_add_source_ids_gin_indexes.sql`

---

## Table Inventory (38 migrations)

### Multi-Tenancy Tables

| Table         | PK                   | Key Columns                                      | Notes               |
| ------------- | -------------------- | ------------------------------------------------ | ------------------- |
| `tenants`     | `tenant_id UUID`     | `slug`, `settings JSONB`, `is_active`            | Root tenancy anchor |
| `users`       | `user_id UUID`       | `tenant_id FK`, `email`, `role`, `password_hash` | Per-tenant users    |
| `workspaces`  | `workspace_id UUID`  | `tenant_id FK`, `slug`, `settings JSONB`         | Isolation boundary  |
| `memberships` | `membership_id UUID` | `user_id FK`, `workspace_id FK`, `role`          | RBAC join table     |

### Core Document Tables

| Table           | PK        | Key Columns                                                                                                | **Active?**                                                 | Notes                                                                                                                                 |
| --------------- | --------- | ---------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `documents`     | `id UUID` | `tenant_id FK`, `workspace_id FK`, `content TEXT`, `content_hash`, `status`, `chunk_count`, `entity_count` | YES — API layer                                             | Written by `DocumentTaskProcessor`; status machine; **NOT written by pipeline KV path**                                               |
| `chunks`        | `id UUID` | `document_id FK`, `content TEXT`, `embedding vector(1536)`                                                 | **PARTIAL** — `content` inserted, `embedding` column unused | Relational chunk mirror. The pipeline stores chunk text+embeddings in KV+vector stores. `chunks.embedding` is orphaned.               |
| `entities`      | `id UUID` | `tenant_id FK`, `name TEXT`, `entity_type`, `embedding vector(1536)`, `source_ids UUID[]`                  | **ORPHANED**                                                | Not written by the active pipeline (pipeline uses AGE graph). Created in migration 002, never populated by `orchestrator::ingestion`. |
| `relationships` | `id UUID` | `source_id FK -> entities`, `target_id FK -> entities`                                                     | **ORPHANED**                                                | Same as `entities` — never populated by active pipeline.                                                                              |

### Task Queue

| Table             | PK        | Key Columns                                                                                     | Notes                                                  |
| ----------------- | --------- | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------ |
| `edgequake_tasks` | `id UUID` | `task_type`, `status`, `tenant_id`, `workspace_id`, `payload JSONB`, `circuit_breaker_failures` | Central async job queue; migrations 002, 019, 020, 031 |

### PDF Storage

| Table           | PK            | Key Columns                                                                                                                       | Notes                                   |
| --------------- | ------------- | --------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------- |
| `pdf_documents` | `pdf_id UUID` | `workspace_id FK`, `document_id FK (NULLABLE)`, `pdf_data BYTEA`, `sha256_checksum`, `processing_status`, `markdown_content TEXT` | Raw PDF bytes + vision extraction state |

### Conversation / Chat

| Table           | PK        | Key Columns                                                   | Notes                 |
| --------------- | --------- | ------------------------------------------------------------- | --------------------- |
| `conversations` | `id UUID` | `tenant_id`, `workspace_id`, `title`, `mode`                  | Chat session          |
| `messages`      | `id UUID` | `conversation_id FK`, `role`, `content TEXT`, `context JSONB` | Chat messages         |
| `folders`       | `id UUID` | `tenant_id`, `workspace_id`, `name`                           | Conversation grouping |

### Audit & Security

| Table           | PK         | Key Columns                                                | Notes                                        |
| --------------- | ---------- | ---------------------------------------------------------- | -------------------------------------------- |
| `audit_logs`    | `id UUID`  | `event_type`, `tenant_id`, `user_id`, `result`, `severity` | Compliance log (migration 012)               |
| `server_config` | `key TEXT` | `value JSONB`, `updated_at`                                | Runtime server configuration (migration 030) |

### Graph Observability (Relational)

| Table                       | PK        | Notes                                                                         |
| --------------------------- | --------- | ----------------------------------------------------------------------------- |
| `workspace_metrics_history` | `id UUID` | Per-workspace snapshot of node_count, edge_count, chunk_count (migration 016) |

---

## Status Enums

```sql
-- Document processing state machine
document_status: pending | processing | indexed | failed

-- PDF extraction state
pdf_processing_status: pending | processing | completed | failed
extraction_method:     text | vision | hybrid

-- Task queue state  
task_status: pending | running | completed | failed | cancelled
```

---

## Critical Foreign Key Relationships

```
tenants(tenant_id)
    |--- workspaces(workspace_id) --> pdf_documents(workspace_id)
    |--- users(user_id)
    |--- memberships

workspaces(workspace_id) + tenants(tenant_id)
    |--- documents(id) --> chunks(document_id)
                      --> pdf_documents(document_id)  [NULLABLE, set async]

entities(id) <-- relationships(source_id)
             <-- relationships(target_id)
             [NOTE: entities NOT linked to documents FK; source_ids is UUID[] array]

edgequake_tasks [tenant_id, workspace_id are plain TEXT columns, no FK to workspaces]
```

---

## Orphaned / Vestigial Columns

| Table.Column                      | Status              | Reason                                                        |
| --------------------------------- | ------------------- | ------------------------------------------------------------- |
| `chunks.embedding vector(1536)`   | VESTIGIAL           | Pipeline writes embeddings to `eq_*_vectors`, not this column |
| `entities.embedding vector(1536)` | VESTIGIAL           | Pipeline writes to `eq_*_vectors`                             |
| `entities.*` (all rows)           | ORPHANED at runtime | Pipeline uses AGE graph, not this table                       |
| `relationships.*` (all rows)      | ORPHANED at runtime | Pipeline uses AGE graph, not this table                       |
| `documents.content TEXT`          | PARTIAL             | Large content bloats table; actual retrieval uses `eq_*_kv`   |

---

## RLS Policies

Row-Level Security is enabled on tenant-scoped tables (migration 009).  
Key pattern: `USING (tenant_id = current_setting('app.tenant_id')::uuid)`.

Tables with RLS: `documents`, `entities`, `relationships`, `memberships`, `workspaces`.

---

## Index Inventory (selected)

| Index                            | Table           | Type  | Purpose                       |
| -------------------------------- | --------------- | ----- | ----------------------------- |
| `idx_documents_tenant_workspace` | `documents`     | BTREE | Tenant-scoped document list   |
| `idx_entities_name`              | `entities`      | BTREE | Entity lookup by name         |
| `idx_entities_embedding`         | `entities`      | HNSW  | Vector similarity (VESTIGIAL) |
| `idx_pdf_documents_checksum`     | `pdf_documents` | BTREE | Deduplication check           |
| `idx_audit_logs_tenant`          | `audit_logs`    | BTREE | Audit trail queries           |
