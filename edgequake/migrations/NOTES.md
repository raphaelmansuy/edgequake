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
| Vision page / chart PNGs  | `document_mm_assets`         | —                     | BYTEA; RLS in 085; `asset_id` REST; FK cascade |
| Non-PDF originals         | `document_originals`         | —                     | BYTEA upload bytes                     |
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

Current max: `098_batch_deletion_task_type_and_claim_index.sql`  
Next available: `099_*`

### Every-boot reconcile SSOT (not checksum-locked)

| Version | support path | Purpose |
|---------|--------------|---------|
| 083 | `support/083/apply.sql` | Native UNIQUE / eq_* arbiter preference |
| 086 | `support/086/apply.sql` | EDGE BFS `idx_edge_source_id` / `idx_edge_target_id` (SPEC-070) |
| 092 | `support/092/apply.sql` | eq_* denorm columns/triggers (SPEC-069); sqlx `092_*.sql` is marker-only |

See SPEC-070: `specs/001-benchmark/001-edgquake-improvements/070-db-ops-excellence.md`.
