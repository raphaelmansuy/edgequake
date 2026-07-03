# SPEC-042 — Code is Law

Every claim maps to a live source file (baseline: v0.13.x + SPEC-042).

---

## Extension pins (SSOT)

| Claim | File | Evidence |
| ----- | ---- | -------- |
| SSOT pin constants | `edgequake/docker/extension-pins.sh` | `EQ_PGVECTOR_*`, `EQ_AGE_*` |
| Docker build pins | `edgequake/docker/Dockerfile.postgres:11-17` | `PGVECTOR_VERSION`, `AGE_GIT_REF`, LABELs |
| Build-time verify | `Dockerfile.postgres:60-67` | `grep default_version` on `.control` files |
| Runtime verify | `edgequake/docker/verify-postgres-extensions.sh` | sources `extension-pins.sh` |

---

## Automatic catalog upgrade

| Claim | File | Evidence |
| ----- | ---- | -------- |
| M042 marker (sqlx) | `migrations/042_pgvector_upgrade_marker.sql` | version 42 |
| M042 apply SSOT | `migrations/support/042/apply.sql:24-30` | `ALTER EXTENSION vector UPDATE` |
| M042 reconcile | `migration_bootstrap/reconcile/m042.rs` | `reconcile_migration_042` |
| M043 marker (sqlx) | `migrations/043_age_upgrade_marker.sql` | version 43 |
| M043 apply SSOT | `migrations/support/043/apply.sql:21-27` | `ALTER EXTENSION age UPDATE` |
| M043 reconcile | `migration_bootstrap/reconcile/m043.rs` | `reconcile_migration_043` |
| Bootstrap orchestration | `migration_bootstrap/mod.rs:721-725` | calls reconcile 042/043 |

---

## Readiness gates

| Claim | File | Evidence |
| ----- | ---- | -------- |
| pgvector ≥ 0.8 gate | `migration_bootstrap/helpers.rs:28-39` | `pgvector_supports_iterative_scan` |
| Degraded when < 0.8 | `migration_bootstrap/mod.rs:272-275` | `Migration042Report::is_degraded` |
| `/ready` 503 | `handlers/health.rs:346-354` | `is_ready_for_traffic` |
| db-start pgvector rebuild | `Makefile:1065-1097` | `_PV_SHIP` case on `0.8.*` |

---

## Extension auto-create

| Claim | File | Evidence |
| ----- | ---- | -------- |
| CREATE EXTENSION vector | `connection.rs` | `pg_initialize` |
| CREATE EXTENSION age | `connection.rs` | `LOAD 'age'` + create |

---

## Health visibility (SPEC-042)

| Claim | File | Evidence |
| ----- | ---- | -------- |
| Extension versions in `/health` | `handlers/health_types.rs` | `MigrationHealthSnapshot` fields |
| Snapshot builder | `handlers/health.rs` | `build_migration_health_snapshot` |

---

## HNSW dimension guard (#275)

| Claim | File | Evidence |
| ----- | ---- | -------- |
| SSOT policy | `capabilities.rs` | `AnnIndexPolicy::resolve`, `HNSW_MAX_DIM_*` |
| M071 guard SQL | `migrations/071_hnsw_optimize.sql` | dim probe + halfvec promotion |
| M071 checksum repair | `reconcile/m071.rs` | `repair_migration_071_checksum_if_needed` |
| Runtime DDL | `vector/ddl.rs` | policy-driven index skip |

---

## Official version alignment (external SSOT)

| Technology | Official latest | EdgeQuake PG16 pin |
| ---------- | --------------- | ------------------ |
| pgvector | [0.8.3 on PGXN](https://pgxn.org/dist/vector) | 0.8.3 ✅ |
| Apache AGE PG16 | [1.6.0 download](https://age.apache.org/download/) | 1.6.0 ✅ |
| Apache AGE PG18 | [1.7.0 download](https://age.apache.org/download/) | N/A (PG16 base) |
