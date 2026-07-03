# SPEC-042 — Version Feature Matrix (Official Docs Battle Test)

**Date:** 2026-07-03  
**Method:** Every claim cites an official source; every adopted feature has an E2E probe in `e2e/run_version_feature_battle_test.sh`.  
**Status:** Grounded against upstream docs as of 2026-07-03.

---

## Official sources (SSOT for this document)

| Component | Primary source | Verified version |
| --------- | -------------- | ---------------- |
| pgvector | [GitHub CHANGELOG](https://github.com/pgvector/pgvector/blob/master/CHANGELOG.md), [PGXN 0.8.3](https://pgxn.org/dist/vector) | **0.8.3** (2026-06-17) |
| pgvector usage | [pgvector README — iterative scans](https://github.com/pgvector/pgvector#iterative-index-scans) | 0.8.0+ |
| Apache AGE | [age.apache.org/download](https://age.apache.org/download/), [GitHub releases](https://github.com/apache/age/releases) | 1.6.0 (PG16), **1.7.0** (PG17/PG18 branches) |
| PostgreSQL 16 | [Release 16 notes](https://www.postgresql.org/docs/release/16.0/) | 16.x (EOL ~2028-11) |
| PostgreSQL 17 | [Release 17 notes](https://www.postgresql.org/docs/release/17.0/) | 17.x (EOL ~2029-11) |
| PostgreSQL 18 | [Release 18 notes](https://www.postgresql.org/docs/release/18.0/) | 18.x (EOL ~2030-11) |

### Upstream discrepancy (documented, not ignored)

The [AGE download page](https://age.apache.org/download/) labels **PG17 stable as 1.6.0**, while GitHub ships **`PG17/v1.7.0-rc0`** with full 1.7.0 release notes. EdgeQuake follows **GitHub release tags** (same pattern as PG16 `PG16/v1.6.0-rc0` → catalog `1.6.0`). Operators on managed PG17 should confirm host AGE ≥ 1.7.0 before expecting modern-tier features.

---

## Triple-track capability matrix

| Feature | PG16 tier | PG17 tier | PG18 tier | EdgeQuake status |
| ------- | --------- | --------- | --------- | ---------------- |
| **pgvector extversion** | 0.8.3 | 0.8.3 | 0.8.3 | ✅ Pinned all images |
| **HNSW iterative scan** (≥0.8.0) | ✅ | ✅ | ✅ | ✅ **Adopted** — `search_tuning.rs` |
| **IVFFlat iterative scan** (≥0.8.0) | ✅ | ✅ | ✅ | ✅ **Adopted** — filtered queries |
| **Improved filtered ANN cost estimation** (0.8.0) | ✅ | ✅ | ✅ | ✅ Passive (planner) |
| **PG18 Hamming/Jaccard perf fix** (0.8.3) | N/A | N/A | ✅ | ✅ Passive when on PG18 |
| **halfvec / sparsevec / binary_quantize** (0.8.0) | Available | Available | Available | 📋 **Phase E-01** — [014](./014-feature-adoption-plan.md#e-01--halfvec-storage-50-disk-savings) |
| **AGE extversion** | 1.6.0 | 1.7.0 | 1.7.0 | ✅ Pinned per profile |
| **AGE Cypher (openCypher subset)** | ✅ | ✅ | ✅ | ✅ Core product |
| **AGE id-column indexes** (1.7.0 #2117) | ❌ | ✅ | ✅ | ✅ Passive after upgrade/migrate |
| **AGE RLS support** (1.7.0 #2309) | ❌ | ✅ | ✅ | 📋 **Phase E-02** — [014](./014-feature-adoption-plan.md#e-02--age-17-rls-for-tenant-isolation) |
| **AGE pg COPY CSV loader** (1.7.0 #2310) | ❌ | ✅ | ✅ | 📋 **Phase E-04** — [014](./014-feature-adoption-plan.md#e-04--age-pg-copy-bulk-loader) |
| **AGE direct array field access** (1.7.0 #2302) | ❌ | ✅ | ✅ | ✅ Passive (runtime perf) |
| **PG server AIO subsystem** | ❌ | ❌ | ✅ (PG18) | ✅ Passive — seq scan/vacuum |
| **PG skip scan (multicolumn btree)** | ❌ | ❌ | ✅ (PG18) | ✅ Passive — benefits M038 btree |
| **PG uuidv7()** | ❌ | ❌ | ✅ (PG18) | 📋 **Phase E-03** — [014](./014-feature-adoption-plan.md#e-03--uuidv7-for-document-ids-pg18) |
| **PG virtual generated columns (default)** | ❌ | ❌ | ✅ (PG18) | N/A — no generated cols today |
| **PG17 streaming read I/O** | ❌ | ✅ (PG17) | ✅ (PG18 builds on it) | ✅ Passive |
| **PG17 improved vacuum memory** | ❌ | ✅ | ✅ | ✅ Passive |

**Legend:** ✅ Adopted or passive benefit | 📋 Available upstream, not wired in app | ❌ Not on this tier

---

## pgvector — what EdgeQuake adopts (grounded)

### 1. Iterative index scans (0.8.0+) — **ADOPTED**

**Official:** [pgvector README — Iterative Index Scans](https://github.com/pgvector/pgvector#iterative-index-scans)

> With approximate indexes, filtering is applied *after* the index is scanned. Starting with 0.8.0, you can enable iterative index scans…

**EdgeQuake code:** `search_tuning.rs` sets per-transaction GUCs when `filtered=true` and `extversion >= 0.8.0`:

```rust
// HNSW filtered: strict_order + max_scan_tuples
SET LOCAL hnsw.iterative_scan = strict_order
SET LOCAL hnsw.max_scan_tuples = 20000

// IVFFlat filtered: relaxed_order
SET LOCAL ivfflat.iterative_scan = relaxed_order
```

**Gate:** `pgvector_supports_iterative_scan()` in `helpers.rs` + M042 bootstrap `/ready` degrade if catalog `< 0.8.0`.

**Battle test probe:** `BT-PV-01` — GUC SET succeeds; `BT-PV-02` — filtered HNSW query returns rows.

### 2. HNSW ef_search tuning — **ADOPTED**

**Official:** [pgvector README — HNSW](https://github.com/pgvector/pgvector#hnsw)

Default `hnsw.ef_search = 40`. EdgeQuake scales `ef_search = clamp(top_k * 4, 40, 1000)`.

**Battle test probe:** `BT-PV-03` — `SET LOCAL hnsw.ef_search` in transaction.

### 3. halfvec / quantization — **Phase E-01 (planned)**

**Official:** [pgvector README — Half-Precision Vectors](https://github.com/pgvector/pgvector#half-precision-vectors)

Available on all tiers once pgvector ≥ 0.8.0. Full implementation plan: [014-feature-adoption-plan.md § E-01](./014-feature-adoption-plan.md#e-01--halfvec-storage-50-disk-savings).

### 4. 0.8.3 maintenance fixes — **PASSIVE**

**Official:** [CHANGELOG 0.8.3](https://github.com/pgvector/pgvector/blob/master/CHANGELOG.md)

- HNSW vacuum corruption fix — benefits all tiers on 0.8.3 pin.
- PG18 Hamming/Jaccard regression fix — benefits PG18 image only.

---

## Apache AGE — what EdgeQuake adopts (grounded)

### 1. Cypher graph storage (1.6.0 intersection) — **ADOPTED**

**Official:** [AGE PG16/v1.6.0 release](https://github.com/apache/age/releases/tag/PG16%2Fv1.6.0-rc0)

EdgeQuake uses parameterized Cypher via `cypher_query_bound`, MERGE/CREATE, map projection, EXISTS subqueries — all 1.6.0 features. **Invariant:** new Cypher must work on 1.6.0 until PG16 tier retired.

**Battle test probe:** `BT-AGE-01` — create graph, MERGE vertex, MATCH return.

### 2. AGE 1.7.0 upgrade path — **ADOPTED (infra)**

**Official:** [PG17/v1.7.0 release notes](https://github.com/apache/age/releases/tag/PG17/v1.7.0-rc0)

> WARNING: upgrade script `age--1.6.0--1.7.0.sql` may take a while for large graphs due to id-column indexes.

**EdgeQuake:** PG17/PG18 images ship fresh 1.7.0; PG16→PG17/PG18 migration via `migrate_postgres_major.sh` + M043 bootstrap.

**Battle test probe:** `BT-AGE-02` — `extversion >= 1.7.0` on pg17/pg18 profiles.

### 3. AGE RLS (1.7.0 #2309) — **Phase E-02 (planned)**

**Official:** 1.7.0 release notes — Row Level Security support added.

Full plan: [014 § E-02](./014-feature-adoption-plan.md#e-02--age-17-rls-for-tenant-isolation). Gate: `extension_version_at_least(age, "1.7.0")` + PG17+.

### 4. AGE id indexes (1.7.0 #2117) — **PASSIVE**

Created automatically on upgrade or fresh 1.7.0 install. Improves VLE traversal — no app code change.

---

## PostgreSQL server — tier-specific benefits

### PG16 (legacy tier)

**Official:** [PostgreSQL 16 release](https://www.postgresql.org/docs/release/16.0/)

EdgeQuake benefits: mature pgvector/AGE ecosystem, `pg_trgm` + `btree_gin` for FTS (migration 045).

**Not available:** PG17 streaming I/O, PG18 AIO, `uuidv7()`.

### PG17 (modern tier)

**Official:** [PostgreSQL 17 release — Overview](https://www.postgresql.org/docs/release/17.0/)

Relevant passive benefits for EdgeQuake:

- **Streaming read I/O** — faster sequential scans over large vector/graph tables.
- **Improved VACUUM memory** — lower memory during autovacuum on busy ingest.
- **B-tree improvements** — multi-value searches (metadata filters).

**Battle test probe:** `BT-PG-17` — `SHOW server_version` starts with `17.`

### PG18 (recommended tier)

**Official:** [PostgreSQL 18 release — Overview](https://www.postgresql.org/docs/release/18.0/)

Relevant passive benefits:

- **Asynchronous I/O (AIO)** — sequential scan, bitmap heap scan, vacuum ([release notes § E.5.3.1.3](https://www.postgresql.org/docs/release/18.0/)).
- **B-tree skip scan** — multicolumn indexes usable without leading column (helps composite btree on workspace vectors).
- **`uuidv7()`** — timestamp-ordered UUIDs (future ID generation).
- **pg_upgrade retains stats** — faster major migrations.

**Battle test probe:** `BT-PG-18-01` — `SELECT uuidv7()` succeeds; `BT-PG-18-02` — `current_setting('io_method', true)` or server version `18.`

---

## Feature adoption roadmap (Phase E)

| ID | Feature | Min tier | Plan |
| -- | ------- | -------- | ---- |
| E-01 | halfvec embeddings | pgvector 0.8+ (all) | [014 § E-01](./014-feature-adoption-plan.md#e-01--halfvec-storage-50-disk-savings) |
| E-02 | AGE RLS tenant policies | AGE 1.7+ / PG17+ | [014 § E-02](./014-feature-adoption-plan.md#e-02--age-17-rls-for-tenant-isolation) |
| E-03 | uuidv7() for document IDs | PG18 | [014 § E-03](./014-feature-adoption-plan.md#e-03--uuidv7-for-document-ids-pg18) |
| E-04 | AGE pg COPY bulk loader | AGE 1.7+ / PG17+ | [014 § E-04](./014-feature-adoption-plan.md#e-04--age-pg-copy-bulk-loader) |
| E-05 | PG18 AIO tuning (`io_method`) | PG18 | Ops doc only (passive) |

---

## Battle test registry

| Probe ID | Asserts | Profiles |
| -------- | ------- | -------- |
| BT-PIN | Dockerfile ↔ extension-pins.sh | all |
| BT-PV-01 | `SET hnsw.iterative_scan` accepted | all |
| BT-PV-02 | Filtered HNSW ANN returns ≥1 row | all |
| BT-PV-03 | halfvec type exists in catalog | all |
| BT-AGE-01 | Cypher MERGE/MATCH round-trip | all |
| BT-AGE-02 | AGE extversion ≥ tier minimum | pg16=1.6, pg17/pg18=1.7 |
| BT-PG-17 | server_version major = 17 | pg17 |
| BT-PG-18-01 | `uuidv7()` exists | pg18 only |
| BT-PG-18-02 | server_version major = 18 | pg18 |
| BT-M042 | support/042/apply.sql idempotent | all |
| BT-M043 | support/043/apply.sql idempotent | all |

Run:

```bash
./specs/042-update-age-pgvector/e2e/run_version_feature_battle_test.sh
# or single profile:
./specs/042-update-age-pgvector/e2e/run_version_feature_battle_test.sh pg17
```

---

## Cross-reference

| Doc | Relationship |
| --- | ------------ |
| [002-first-principles.md](./002-first-principles.md) | Invariants §3 iterative scan, §8 triple-track |
| [006-postgres-expert-lens.md](./006-postgres-expert-lens.md) | DBA operational view |
| [012-dual-pg-major-compatibility.md](./012-dual-pg-major-compatibility.md) | Tier policy |
| [008-implementation-plan.md](./008-implementation-plan.md) | Phase D battle test + Phase E roadmap |
| [014-feature-adoption-plan.md](./014-feature-adoption-plan.md) | **Phase E detail (E-01…E-04)** |
