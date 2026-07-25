# SPEC-088 — Data-Layer Operation Inventory & Hardening

Mission SSOT: [00-mission.md](./00-mission.md)

## How to read these docs

| File | Purpose |
|---|---|
| [00-inventory.md](./00-inventory.md) | Master table of every Ref ID |
| [postgres.md](./postgres.md) | All `DATA-PG-*` operations |
| [pgvector.md](./pgvector.md) | All `DATA-PGVEC-*` operations |
| [age.md](./age.md) | All `DATA-AGE-*` operations |
| [complexity-matrix.md](./complexity-matrix.md) | Complexity × limits × failure modes |
| [version-matrix.md](./version-matrix.md) | PG16 / PG17 / PG18 results |
| [indexes.md](./indexes.md) | Index catalog → consuming Ref IDs |
| [benchmarks/](./benchmarks/) | Per-op EXPLAIN + scaling notes |
| [improvements.md](./improvements.md) | Phase 5–6 **done & proved** SSOT (IMP catalog, evidence table, regression snapshot) |

Canonical copies also live under `docs/data-layer/` (Definition of Done path).

## Proven performance (Phase 6 summary)

Request-path data access is optimized for **index-backed** work and **RT collapse**
(O(K log N) / O(1) RT batches), not O(N) scans or O(K) network hops.

| Domain | Win | How proven |
|---|---|---|
| Graph | Native batch/BFS/delete/clear; Cypher opt-out only | `e2e_spec088` IMP-031*, `e2e_spec060` index plan |
| Vectors | Filtered ANN iterative_scan + partial HNSW auto | IMP-001/002*, contract 075, return-K e2e |
| Tasks | Fair claim UNION + SKIP LOCKED + claim index | IMP-140*, `postgres_claim_lease` 8/8 |
| KV/API | Dual-key + multi-key `get_by_ids_ordered` SSOT | IMP-075* source contracts + unit/e2e |

Full before/after table: **[improvements.md § Proven performance](./improvements.md#proven-performance-improvements-evidence-based)**.

## Ref ID scheme

```
DATA-<ENGINE>-<DOMAIN>-<OPERATION>-<NNN>
ENGINE ∈ PG | PGVEC | AGE
```

- **Immutable**: never renumber, reuse, or delete. Deprecate with `@status deprecated`.
- **Code**: `edgequake_storage::dataop` constants + `@dataop` annotation blocks.
- **SQL**: `/* DATA-… */` comment prefix via `dataop::sql_comment` (visible in `pg_stat_statements`).
- **Metrics**: `TimedStorageOp::start_dataop(REF)`.

## Stack (verified)

| Component | Pin |
|---|---|
| PostgreSQL | 16 / 17 / **18** (default) |
| pgvector | **0.8.5** (≥0.8.2 CVE floor) |
| Apache AGE | **1.8.0** (PG18) |
| Driver | sqlx 0.8 |
| Migrations | sqlx migrate + checksum lock + every-boot reconcile |

## Lint

```bash
python3 specs/088-data-layer/scripts/lint_dataop_xref.py
```

## Tests

```bash
# Unit (no DB): registry integrity
cargo test -p edgequake-storage --lib dataop

# Phase 6 IMP e2e + source contracts (requires DATABASE_URL for DB-backed cases)
export DATABASE_URL=postgres://edgequake:edgequake_secret@localhost:5432/edgequake
cargo test -p edgequake-storage --features postgres --test e2e_spec088_improvements

# Ops matrix (235+ Ref IDs)
cargo test -p edgequake-storage --features postgres --test data_layer_ops_matrix -- --test-threads=4

# Data-layer contract tests (optional DB)
cargo test -p edgequake-storage --test data_layer_registry -- --nocapture
cargo test -p edgequake-storage --test data_layer_limits -- --nocapture

# Expand plan smoke (Bitmap Index Scan)
cargo test -p edgequake-storage --features postgres --test e2e_spec060_age_expand_perf

# Fair claim lease e2e
cargo test -p edgequake-tasks --features postgres --test postgres_claim_lease -- --test-threads=1
```

Filter by Ref ID:

```bash
cargo test -p edgequake-storage --test data_layer_limits DATA_PGVEC_VECTORS_ANN_QUERY_001
```
