---
title: "SPEC-098 — Entity spine + EDGE arbiter (operator)"
description: "SPEC-098 entity spine ensure behaviour."
---

# SPEC-098 — Entity spine + EDGE arbiter (operator)

Typed fleet embeddings require relational spine rows. Migrations **139** (entities)
and **140** (EDGE arbiter + relationships) are markers; support scripts reconcile
on PG **16 / 17 / 18** (portable SQL).

## Hot path

1. Saturated SOURCE_IDS KEEP still upserts the relational spine (AGE description
   mutation skipped). Reprocess should not fail with `typed fleet mirror resolved 0/N`.
2. Native AGE edge upserts use a **single** arbiter
   `idx_edge_eq_source_target_rel (eq_source_id, eq_target_id, eq_rel_type)`.
   Reprocess of multi-chunk extracts must not fail with
   `ON CONFLICT DO UPDATE command cannot affect row a second time`.
3. When `EDGEQUAKE_NATIVE_GRAPH_WRITES=0` (debug), Cypher MERGE also keys edges
   on `(source_id, target_id, relation_type)` — same multigraph semantics.
4. **Near-complete mirror misses (`999/1000`)** — If SPEC-098 miss samples look like
   `NAME_WITH_->_ARROW->OTHER:REL`, the fail is usually legacy-key parse when entity
   names contain `->`, not a missing spine. Parser prefers **index-guided** splits
   (SPEC-133: both endpoints must resolve in `entities`) and falls back to last-`->`
   (`parse_relationship_legacy_key`). Source-arrow and **target-arrow** names are
   covered when the spine exists. Reprocess after upgrade; do not re-run 139/140
   solely for this class. Residual: pathological multi-both-resolve / `:` inside
   names — see SPEC-133 LAW-133-9 (optional escaped keys).

## Manual re-run

```bash
# Entity spine (AGE vertices → entities)
psql "$DATABASE_URL" -f edgequake/migrations/support/139/apply.sql

# EDGE arbiter hygiene + AGE edges → relationships
psql "$DATABASE_URL" -f edgequake/migrations/support/140/apply.sql
```

Progress:

```sql
SELECT value FROM server_config WHERE key = 'spec098_spine_ensure_progress';
SELECT value FROM server_config WHERE key = 'spec098_edge_arbiter_progress';
```

## EDGE index checklist

```sql
-- Replace 'edgequake' with your graph name (server_config.age_graph_name).
SELECT indexname
FROM pg_indexes
WHERE schemaname = 'edgequake' AND tablename = 'EDGE'
ORDER BY 1;
```

| Index | Required |
|-------|----------|
| `idx_edge_eq_source_target_rel` | **Yes** (3-col multigraph arbiter) |
| `idx_edge_eq_source_target` | **No** — must be dropped |
| `idx_edge_source_target_unique` | **No** — must be dropped |

Runtime bootstrap (`ensure_eq_id_columns` / `reconcile_legacy_graph_arbiters`)
drops legacy UNIQUEs even when the schema is already “ready”.

## 040 vs 139 vs 140

| Migration | Progress key | Role |
|-----------|--------------|------|
| 040 | (see support/040) | Historical CQRS entity dual-write backfill |
| 139 | `spec098_spine_ensure_progress` | Ensure bare `entities` for typed fleet FK |
| 140 | `spec098_edge_arbiter_progress` | Single EDGE arbiter + `relationships` spine |
| 141 | `spec098_document_lifecycle_status` | `documents_valid_status` includes `deleting` / `delete_failed` |

## Delete dual-SSOT checklist (W9–W11)

During an in-flight single or selected bulk delete:

```sql
-- Both projections must agree (LAW-098-9)
SELECT id, status FROM documents WHERE status IN ('deleting', 'delete_failed');
```

KV metadata for the same ids must also show `"status":"deleting"`. After success,
the id must be absent from SQL `documents`, KV `*-metadata`, and `GET /documents`.

```bash
psql "$DATABASE_URL" -f edgequake/migrations/support/141/apply.sql
```

## Delete failure honesty (W12 / LAW-098-11)

If admit logs `event=spec098_sql_deleting_mirror_failed`, the SQL CHECK still rejects
`deleting` / `delete_failed` (pre-141). KV admit still succeeds and the delete task
runs; re-apply support/141 so list dual-write can mirror lifecycle statuses:

```bash
psql "$DATABASE_URL" -f edgequake/migrations/support/141/apply.sql
```

Failed cascades leave `status=delete_failed` (not pipeline `failed`). Batch task
results include `failed: [{document_id, reason}]` for operator RCA.

### Post-proof / shared prune (LAW-098-12)

If deletes fail with `Post-proof failed: N nodes and M edges still reference
document sources`, deploy the cascade Replace write-mode fix, then **retry Delete**
on those `delete_failed` rows. Shared-entity prune must persist without
`eq_merge_graph_properties` re-unioning pruned `source_ids`.
