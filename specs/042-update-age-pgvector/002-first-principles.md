# SPEC-042 — First Principles

---

## 1. Extension version is infrastructure, not application semver

EdgeQuake application migrations (`_sqlx_migrations`) track **schema**. PostgreSQL extensions (`vector`, `age`) have a separate **catalog version** (`pg_extension.extversion`) tied to shared libraries on disk.

**Invariant:** After a Docker image rebuild, `ALTER EXTENSION … UPDATE` must run so catalog matches the shipped `.so` files.

---

## 2. PG major bounds extension ceiling

Apache AGE releases are **per PostgreSQL major**. The [official download page](https://age.apache.org/download/) documents:

| PostgreSQL | Latest stable AGE |
| ---------- | ----------------- |
| 18 | 1.7.0 |
| 17 | 1.7.0 (PG17 branch) / 1.6.0 (download page stable label) |
| 16 | **1.6.0** |
| 15 | 1.6.0 |

EdgeQuake pins **PG16** → **AGE 1.6.0** is the correct target, not 1.7.0.

---

## 3. pgvector iterative scan is a readiness gate

Filtered ANN (`WHERE metadata … ORDER BY embedding <=> q`) requires pgvector **≥ 0.8.0** for reliable iterative HNSW scan ([pgvector 0.8.0 release](https://www.postgresql.org/about/news/pgvector-080-released-2952/)).

**Invariant:** `/ready` returns 503 when pgvector catalog `< 0.8.0` and extension is installed (`migration_042.is_degraded()`).

---

## 4. Automatic migration — three layers (DRY with SPEC-041)

| Layer | Mechanism | When |
| ----- | --------- | ---- |
| **L0 Image** | `Dockerfile.postgres` compiles extensions | Fresh deploy |
| **L1 db-start** | `ALTER EXTENSION vector UPDATE` + container rebuild if stale | Dev `make dev` |
| **L2 Bootstrap** | M042/M043 markers + `support/042|043/apply.sql` | Every backend startup |

**SSOT for apply SQL:** `migrations/support/042/apply.sql`, `migrations/support/043/apply.sql`.

---

## 5. Single pin file (DRY)

Version strings must not drift across Dockerfile, Makefile, verify script, and docs.

**SSOT:** `edgequake/docker/extension-pins.sh` — all consumers source or mirror it.

---

## 6. AGE upgrade edge cases

From [AGE 1.6.0 release notes](https://github.com/apache/age/releases/tag/PG16%2Fv1.6.0-rc0):

- Upgrading from 1.5.x may require **dropping GIN indexes on agtype** before `ALTER EXTENSION age UPDATE`, then recreating.
- EdgeQuake M038/M046/M078 create **btree + jsonb GIN** indexes — bootstrap reconcile re-applies if missing post-upgrade.

**Principle:** `ALTER EXTENSION age UPDATE` is wrapped in exception handler (idempotent skip on mismatch).

---

## 7. Phase B — PostgreSQL 18 (SPEC-042-B)

**Mission:** Issue #161 is **fully** closed on PG18 (AGE 1.7.0). PG16 remains **supported** — not replaced.

| Principle | PG18 implementation |
| --------- | ------------------- |
| Data move | `pg_dump` / `pg_restore` — opt-in, not forced |
| Extension install | Fresh AGE 1.7.0 on PG18 |
| Catalog sync | Same M042/M043 bootstrap (DRY) |
| Procedure SSOT | `scripts/migrate_postgres_major.sh` |
| Rollback | Restore PG16 dump |

See [010-postgres-18-migration.md](./010-postgres-18-migration.md).

---

## 8. Multi-major compatibility (SPEC-042-C) — PG16 + PG17 + PG18

**Triple-track:** PG16 (AGE 1.6.0), PG17 + PG18 (AGE 1.7.0).

| Tier | PG | AGE | #161 |
| ---- | -- | --- | ---- |
| Legacy supported | 16 | 1.6.0 | Partial |
| Modern supported | 17 | 1.7.0 | Full |
| Recommended | 18 | 1.7.0 | Full |

**PG17 first principle (P6):** Same AGE capability as PG18 — only PostgreSQL server differs. Group PG17/PG18 for `extversion >= 1.7.0` feature gates; keep PG16 at 1.6 intersection.

**Invariants:**

1. Never require PG17/18 for app releases (unless PG-only SQL added — none today).
2. Never fork M042/M043/M078 per major.
3. Gate AGE 1.7-only features on `extversion >= 1.7.0`.
4. Test all three images on release (`check_extension_pins.sh all`).

---

## 9. HNSW dimension ceilings (GitHub #275)

pgvector ANN indexes have **hard dimension walls** unrelated to EdgeQuake semver:

| Type | HNSW max dims |
| ---- | ------------- |
| `vector` | 2000 |
| `halfvec` | 4000 |

**Invariant:** Migrations and runtime DDL must call `AnnIndexPolicy::resolve(dim, mode)` before `CREATE INDEX … hnsw`. dim ∈ (2000, 4000] auto-promotes to `halfvec`; dim > 4000 skips ANN.

See [015-issue-275-hnsw-dimension-guard.md](./015-issue-275-hnsw-dimension-guard.md).
