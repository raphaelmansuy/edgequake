# Migration Notes — Source of Truth Map

> **IMMUTABILITY RULE**: All `.sql` files in this directory are checksum-locked  
> (`checksums.lock`). Never edit an existing `.sql` migration file.  
> New schema changes must be new numbered migration files.

---

## Source of Truth Map (SPEC-021 2026-06-25)

| Domain                    | Authoritative Store          | CQRS Read Model       | Notes                                  |
| ------------------------- | ---------------------------- | --------------------- | -------------------------------------- |
| Document lifecycle        | `documents` table            | —                     | status, chunk_count, entity_count      |
| Chunk text                | `eq_*_kv` store              | —                     | key: `{id}-chunk-{n}`                  |
| Entity (traversal)        | AGE graph `Node`             | `entities` table      | Cypher MATCH; relational for analytics |
| Relationships (traversal) | AGE graph `EDGE`             | `relationships` table | Cypher MATCH; relational for analytics |
| Chunk embeddings          | `eq_*_vectors`               | —                     | `metadata.type = "chunk"`              |
| Entity embeddings         | `eq_*_vectors`               | —                     | `metadata.type = "entity"`             |
| PDF raw bytes             | `pdf_documents.pdf_data`     | —                     | BYTEA + processing state               |
| Task queue                | `edgequake_tasks`            | —                     | Async background jobs                  |
| Conversations             | `conversations` + `messages` | —                     | Chat history                           |
| Workspace registry        | `workspaces` table           | —                     | Tenant isolation                       |

---

## Vestigial Columns (never written by active pipeline)

| Column                   | Table      | Status                  | Remedy                   |
| ------------------------ | ---------- | ----------------------- | ------------------------ |
| `embedding vector(1536)` | `chunks`   | VESTIGIAL — always NULL | Dropped in migration 039 |
| `embedding vector(1536)` | `entities` | VESTIGIAL — always NULL | Dropped in migration 039 |

---

## CQRS Read Models

The `entities` and `relationships` tables in migration 002 were originally intended
as the primary store. The pipeline now uses Apache AGE for traversal.

From **migration 039** onward:
- `entities` is populated by dual-write in `KnowledgeGraphMerger` (when `entity_sync_mode != disabled`)
- It serves as a **CQRS read model** for analytics, full-text search, and JOIN queries
- AGE graph remains the **primary source** for graph traversal operations

From **migration 040** onward:
- A one-time backfill populates `entities` from the existing AGE graph
- `entity_sync_mode` transitions to `"full"` after backfill completes

---

## Migration Naming Convention

```
NNN_descriptive_name.sql
  NNN: sequential 3-digit number
  descriptive_name: snake_case, describes what changed
```

Current max: `038_add_source_ids_gin_indexes.sql`  
Next available: `039_*`
