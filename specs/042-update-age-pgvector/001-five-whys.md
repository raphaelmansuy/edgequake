# SPEC-042 — Five Whys (Issue #161)

---

## Why upgrade AGE and pgvector?

**Because** operators want security fixes, planner improvements, and Cypher compatibility with upstream Apache AGE and pgvector communities ([#161](https://github.com/raphaelmansuy/edgequake/issues/161)).

---

## Why does EdgeQuake lag official "latest"?

**Because** SPEC-022 identified pgvector **0.7.4** blocking iterative scan; pins were bumped to **0.8.3** + AGE **1.6.0** but docs/spec-022 still referenced old versions.

---

## Why not ship AGE 1.7.0 now?

**Because** 1.7.0 stable builds target **PostgreSQL 17/18** only ([AGE download page](https://age.apache.org/download/)). EdgeQuake Docker uses **PG16-bookworm** — the PG16 ceiling is **1.6.0**.

---

## Why do volumes survive extension library upgrades?

**Because** PostgreSQL data directories persist `pg_extension.extversion` across container rebuilds. New `.so` files without `ALTER EXTENSION UPDATE` → runtime errors or silent capability gaps.

---

## Why bootstrap migrations 042/043?

**Because** sqlx migrations record schema markers; **blocking** `ALTER EXTENSION` + `REINDEX` run in post-hooks (same pattern as M038) — idempotent, logged, surfaced in `/health`.

**Root cause addressed:** Infra pin drift + catalog/library mismatch on persistent volumes.
