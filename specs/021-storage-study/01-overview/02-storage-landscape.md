# 02 — Storage Landscape

> **Spec**: 021-storage-study  
> **File**: 01-overview/02-storage-landscape.md  
> **Date**: 2026-06-25

---

## Storage System Inventory

EdgeQuake uses **five distinct storage systems**, all mediated by Rust trait objects:

```
+----------------------------------------------------------+
|                   EdgeQuake Production                    |
|                                                          |
|  PostgreSQL 15+                                          |
|  +---------+  +---------+  +-----------+  +-----------+ |
|  |Relational|  |  KV    |  |  Vector   |  |  AGE      | |
|  | Tables  |  | Tables  |  |  Tables   |  |  Graph    | |
|  |(SQL/DML)|  |(JSONB)  |  |(pgvector) |  |(Cypher)   | |
|  +---------+  +---------+  +-----------+  +-----------+ |
|                                                          |
|  + Apache AGE graph catalog (ag_catalog schema)          |
+----------------------------------------------------------+

Testing / CI
+----------------------------------------------------------+
|              In-Memory Adapters (no PostgreSQL)           |
|  MemoryKVStorage  MemoryVectorStorage  MemoryGraphStorage |
+----------------------------------------------------------+
```

---

## Storage Trait Hierarchy

```
edgequake_storage::traits
  |
  +-- KVStorage                      (key-value, JSONB)
  |     impl: PostgresKVStorage      -> public.eq_{prefix}_kv
  |     impl: MemoryKVStorage        -> HashMap<String, Value>
  |
  +-- VectorStorage                  (f32 embeddings, cosine distance)
  |     impl: PgVectorStorage        -> public.eq_{prefix}_vectors
  |     impl: MemoryVectorStorage    -> Vec<(String, Vec<f32>, Value)>
  |
  +-- GraphStorage                   (property graph: Node + EDGE)
  |     impl: PostgresAGEGraphStorage -> AGE graph via Cypher
  |     impl: MemoryGraphStorage     -> HashMap nodes + edges
  |
  +-- WorkspaceVectorRegistry        (per-workspace vector isolation)
  |     impl: PgWorkspaceVectorRegistry -> per-workspace PgVectorStorage
  |     impl: MemoryWorkspaceVectorRegistry
  |
  +-- PdfDocumentStorage             (raw PDF + processing state)
  |     impl: PostgresPdfStorage     -> public.pdf_documents
  |     impl: MemoryPdfStorage       -> HashMap<Uuid, PdfDocument>
  |
  +-- ConversationStorage            (chat history)
        impl: PostgresConversationStorage -> conversations + messages tables
        impl: MemoryConversationStorage   -> HashMap
```

---

## Storage Role Summary

| Storage System                 | Trait                                       | Backend Table(s)                                | Primary Use                                        |
| ------------------------------ | ------------------------------------------- | ----------------------------------------------- | -------------------------------------------------- |
| KV Store                       | `KVStorage`                                 | `eq_{prefix}_kv`                                | Doc metadata, chunk text, LLM cache, keyword cache |
| Vector Store (global)          | `VectorStorage`                             | `eq_{prefix}_vectors`                           | Chunk+entity embeddings (default/legacy namespace) |
| Vector Store (workspace)       | `WorkspaceVectorRegistry` → `VectorStorage` | `eq_{ns}_ws_{uuid8}_vectors`                    | Per-workspace embeddings with correct dimension    |
| Graph Store                    | `GraphStorage`                              | AGE graph (`Node`, `EDGE`)                      | Entities, relationships, graph traversal           |
| Relational: documents          | (direct SQL via sqlx)                       | `documents`                                     | Document lifecycle, status, metadata (API layer)   |
| Relational: tasks              | (direct SQL via sqlx)                       | `edgequake_tasks`                               | Async job queue                                    |
| Relational: pdf_documents      | `PdfDocumentStorage`                        | `pdf_documents`                                 | Raw PDF bytes + vision extraction state            |
| Relational: tenants/workspaces | `WorkspaceService`                          | `tenants`, `workspaces`, `users`, `memberships` | Multi-tenancy                                      |
| Relational: conversations      | `ConversationStorage`                       | `conversations`, `messages`, `folders`          | Chat history                                       |
| Relational: audit              | `AuditLogger`                               | `audit_logs`                                    | Compliance events                                  |

---

## Backend Selection Logic

```
Environment variable DATABASE_URL set?
  YES -> PostgreSQL adapters
  NO  -> Memory adapters (test mode only)

EDGEQUAKE_LLM_PROVIDER / OPENAI_API_KEY controls LLM provider
(does not affect storage selection)
```

Source: `edgequake-api/src/state/mod.rs`, `AppState::new_postgres()` vs `AppState::new_memory()`.

---

## Namespace / Table Prefix Conventions

The KV and vector stores use a **namespace prefix** derived from the workspace
and storage configuration:

```
PostgresConfig::table_prefix() -> String
  = format!("eq_{namespace}")   // e.g. "eq_default"

KV table:     public.eq_{prefix}_kv        -> eq_eq_default_kv
Vector table: public.eq_{prefix}_vectors   -> eq_eq_default_vectors

Workspace vector table:
  WorkspaceVectorConfig::table_name()
  = format!("eq_{ns}_ws_{uuid8}_vectors")
  -> eq_default_ws_4e32a055_vectors
```

See [02-schema/05-dynamic-tables.md](../02-schema/05-dynamic-tables.md) for full details.
