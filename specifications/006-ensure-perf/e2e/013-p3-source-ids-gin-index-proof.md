# E2E Proof 013 — P3 Source IDs GIN Index Migration

**Spec:** SPEC-006 P3  
**Requirement:** TR-006-006 — index-backed prefix queries  
**Status:** ✅ Verified 2026-06-06 (static; apply at deploy)

---

## First Principle

SQL push-down without indexes = correct semantics, **wrong latency**. Prefix scans on `source_id` / `source_ids` must be index-backed at scale.

---

## Code Is Law

Migration: `edgequake/migrations/038_add_source_ids_gin_indexes.sql`

| Index | Target |
|-------|--------|
| `idx_*_vertex_source_id` | btree on `properties->>'source_id'` |
| `idx_*_vertex_source_ids_gin` | GIN on `properties->'source_ids'` |
| `idx_*_edge_source_ids_gin` | GIN on edge `source_ids` |

---

## Automated Proof

```bash
./scripts/spec006_source_ids_migration.sh
```

**Runtime proof (ops):** after `make postgres-start` + migration apply:

```sql
SELECT indexname FROM pg_indexes
WHERE indexname LIKE '%source_ids%'
   OR indexname LIKE '%vertex_source_id';
```
