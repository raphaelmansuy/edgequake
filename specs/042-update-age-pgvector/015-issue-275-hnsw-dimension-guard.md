# SPEC-042 — Issue #275: HNSW Dimension Guard

**Trigger:** [GitHub #275](https://github.com/raphaelmansuy/edgequake/issues/275) — *"The migration triggers an exceedance of the maximum number of dimensions in the index"*  
**Date:** 2026-07-03  
**Status:** `IMPLEMENTED`  
**Method:** First principles + Code is law

---

## TL;DR

Upgrading to **v0.13.3** on a database with **existing vector content** at embedding dimension **> 2000** (e.g. `text-embedding-3-large` @ 3072, Vertex `gemini-embedding-001` @ 3072) hard-fails at **migration 071** when sqlx rebuilds HNSW with `vector_cosine_ops` on a `vector(N)` column.

pgvector HNSW ceilings ([official README](https://github.com/pgvector/pgvector#hnsw)):

| Column type | Max HNSW dimensions |
| ----------- | ------------------- |
| `vector`    | **2000**            |
| `halfvec`   | **4000**            |
| `> 4000`    | No HNSW — sequential scan only |

---

## First principles

### 1. Index type limits are physics, not app semver

HNSW index dimension caps are enforced by **pgvector shared libraries**, independent of EdgeQuake version. Application migrations must **probe `atttypmod` dimension** before `CREATE INDEX … USING hnsw`.

### 2. Column type and index opclass must agree

`USING hnsw (embedding vector_cosine_ops)` requires `vector` column.  
`USING hnsw (embedding halfvec_cosine_ops)` requires `halfvec` column.

**Invariant:** For dim ∈ (2000, 4000], promote `vector → halfvec` *before* index build.

### 3. Single SSOT (DRY)

| Layer | SSOT |
| ----- | ---- |
| Rust runtime DDL | `capabilities.rs` → `AnnIndexPolicy::resolve(dim, mode)` |
| SQL migrations | Mirror constants in comments; logic in `071_hnsw_optimize.sql`, `support/080/apply.sql` |
| Checksum repair | `reconcile/m071.rs` (same pattern as M078 / #273) |

### 4. Fail-open for extreme dimensions

dim > 4000: **skip ANN index**, log NOTICE, continue startup. Queries fall back to sequential scan (documented limitation).

---

## Code is law — evidence map

| Claim | File | Evidence |
| ----- | ---- | -------- |
| HNSW max 2000 / 4000 constants | `edgequake-storage/.../capabilities.rs` | `HNSW_MAX_DIM_VECTOR`, `HNSW_MAX_DIM_HALFVEC` |
| Runtime policy resolver | `capabilities.rs` | `AnnIndexPolicy::resolve` |
| New table DDL uses policy | `vector/ddl.rs` | `create_table` skips ANN when `!hnsw_viable` |
| M071 dimension guard | `migrations/071_hnsw_optimize.sql` | promote halfvec + opclass branch |
| M080 dim > 4000 skip | `migrations/support/080/apply.sql` | `CONTINUE` with NOTICE |
| M071 checksum repair | `reconcile/m071.rs` | `repair_migration_071_checksum_if_needed` |
| Bootstrap hook | `migration_bootstrap/mod.rs` | runs repair before `MIGRATOR.run()` |
| Lockfile | `migrations/checksums.lock` | updated SHA-384 for 071 |

---

## Root cause (5 WHY)

1. **Why** does v0.13.3 upgrade fail? → sqlx migration **071** errors on `CREATE INDEX … hnsw`.
2. **Why** does CREATE INDEX fail? → pgvector rejects dimension **> 2000** on `vector` HNSW.
3. **Why** is dimension > 2000? → Workspace uses **3072-d** embedding models (OpenAI large, Vertex).
4. **Why** did M071 not handle it? → IMP-04 optimized `ef_construction` only; no `atttypmod` probe.
5. **Why** no shared guard? → Dimension policy was not centralized (violates DRY).

---

## Fix plan (implemented)

| Step | Action | Status |
| ---- | ------ | ------ |
| F-01 | `AnnIndexPolicy` SSOT in `capabilities.rs` | ✅ |
| F-02 | M071: dim probe → halfvec promotion → correct opclass | ✅ |
| F-03 | M080: skip dim > 4000 | ✅ |
| F-04 | `ddl.rs`: policy-driven column type + skip ANN | ✅ |
| F-05 | M071 checksum repair for already-applied installs | ✅ |
| F-06 | Unit tests `ann_index_policy_tests` | ✅ |
| F-07 | Update `checksums.lock` | ✅ |
| F-08 | `run_hnsw_dimension_battle_test.sh` (BT-275-01…03) | ✅ |

---

## Battle test probes

```bash
make hnsw-dimension-battle-test    # BT-275 per PG tier
make spec042-battle-test-all       # full SPEC-042 suite
```

| Probe | Assertion |
| ----- | --------- |
| BT-275-01 | `vector(3072)` + `vector_cosine_ops` HNSW **fails** |
| BT-275-02 | M071 promotes → `halfvec(3072)` + `halfvec_cosine_ops` HNSW |
| BT-275-03 | dim=5000 → **no** HNSW index after M071 |

---

## Operator matrix

| Workspace dim | Column after M071 | HNSW | Query mode |
| ------------- | ------------------- | ---- | ---------- |
| ≤ 2000 | `vector(N)` | `vector_cosine_ops` | ANN |
| 2001–4000 | `halfvec(N)` (auto-promoted) | `halfvec_cosine_ops` | ANN |
| > 4000 | `vector(N)` unchanged | **none** | Sequential scan |

---

## Verification

```bash
# Unit tests
cargo test -p edgequake-storage ann_index_policy_tests
cargo test -p edgequake-api migration_071_checksum

# Checksum gate
./scripts/check_migration_checksums.sh

# Simulate 3072-d workspace upgrade (requires Postgres + pgvector)
# 1. Create eq_*_vectors with vector(3072) + sample rows
# 2. Run backend bootstrap — M071 must complete without dimension error
```

---

## Related

| Item | Link |
| ---- | ---- |
| Issue #275 | [HNSW too many dimensions](https://github.com/raphaelmansuy/edgequake/issues/275) |
| SPEC-041 #271 context | M071 dim guard was P1 separate track — now closed |
| Phase E-01 halfvec | [014-feature-adoption-plan.md](./014-feature-adoption-plan.md) |
| pgvector reference | [zz-reference/001-pgvector/007-code-audit.md](../../zz-reference/001-pgvector/007-code-audit.md) |
