# SPEC-042 — Postgres / AGE / pgvector Expert Lens

**Audience:** DBA, SRE, graph storage maintainers  
**Stack:** PostgreSQL 16 / 17 / 18 — see triple-track in [012-dual-pg-major-compatibility.md](./012-dual-pg-major-compatibility.md)

---

## Official stable versions (2026-07-03)

### pgvector

| Source | Version | Notes |
| ------ | ------- | ----- |
| [PGXN](https://pgxn.org/dist/vector) | **0.8.3** | Released 2026-06-17 |
| [GitHub releases](https://github.com/pgvector/pgvector/releases) | v0.8.3 | HNSW iterative scan, PG18 fixes in 0.8.2+ |

**Upgrade command** (per upstream docs):

```sql
ALTER EXTENSION vector UPDATE;
-- then REINDEX vector ANN indexes if minor version crossed 0.8 boundary
```

EdgeQuake M042 apply.sql automates both.

### Apache AGE

| PG Major | Official stable | Git tag | EdgeQuake profile |
| -------- | --------------- | ------- | ----------------- |
| 18 | 1.7.0 | PG18/v1.7.0-rc0 | `pg18` ✅ |
| **17** | **1.7.0** | **PG17/v1.7.0-rc0** | **`pg17` ✅** |
| 16 | 1.6.0 | PG16/v1.6.0-rc0 | `pg16` ✅ |
| 15 | 1.6.0 | PG15/v1.6.0-rc0 | — |

Source: [age.apache.org/download](https://age.apache.org/download/)

**Note:** Upstream uses `-rc0` suffix on release tags; `extversion` in catalog reports **`1.6.0`**.

---

## AGE 1.6.0 upgrade hazards (from release notes)

1. **agtype GIN operator changes** — drop GIN indexes on agtype before upgrade from 1.5.x, recreate after.
2. EdgeQuake indexes use **`agtype_to_json(properties)::jsonb`** GIN (M038) — compatible pattern; reconcile re-creates if missing.

### AGE 1.7.0 (PG17+ only)

- Long-running `age--1.6.0--1.7.0.sql` on large graphs when restoring PG16 dumps to PG17/PG18.
- Row Level Security support — defer adoption in EdgeQuake until feature spec.
- **Shipped on:** `pg17` and `pg18` profiles (`Dockerfile.postgres.pg17`, `.pg18`).

---

## pgvector 0.8.x operational GUCs

After upgrade, EdgeQuake filtered ANN benefits from:

```sql
SET hnsw.iterative_scan = strict_order;  -- or relaxed_order
SET hnsw.ef_search = 100;                -- tune recall/latency
```

Gate: `pgvector_supports_iterative_scan(extversion)` in bootstrap helpers.

---

## Index rebuild scope (M042)

Reindex targets:

```sql
pg_tables.tablename LIKE 'eq\_%\_vectors'
AND indexdef ILIKE '%USING hnsw%' OR '%USING ivfflat%'
```

**O(n) cost:** proportional to vector row count × index count. Runs at startup — acceptable for dev/small prod; large prod may prefer maintenance window + manual `apply_042.sh`.

---

## Verification queries

```sql
SELECT extname, extversion,
       (SELECT default_version FROM pg_available_extensions WHERE name = e.extname) AS shipped
FROM pg_extension e
WHERE extname IN ('vector', 'age');

SELECT indexrelname, pg_get_indexdef(indexrelid)
FROM pg_index i
JOIN pg_class c ON c.oid = i.indexrelid
WHERE c.relname LIKE '%hnsw%' OR c.relname LIKE '%embedding%';
```

---

## Expert verdict

| Area | Grade | Rationale |
| ---- | ----- | --------- |
| Pin currency (PG16) | **A** | Latest stable for PG16 tier |
| Pin currency (PG17/PG18) | **A** | AGE 1.7.0 + pgvector 0.8.3 verified |
| Catalog sync | **A-** | Automatic M042/M043; manual RDS still ops burden |
| Upgrade safety | **B+** | Exception-wrapped ALTER; GIN edge case documented |
| Multi-major ops | **A-** | Three images + `check_extension_pins.sh all` |
