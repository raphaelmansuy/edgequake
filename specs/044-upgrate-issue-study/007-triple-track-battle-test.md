# SPEC-044 — Triple-Track Battle Test (PG16 / PG17 / PG18)

**Date:** 2026-07-06  
**Status:** Plan — gates release v0.14.2 Cypher fix  
**SSOT pins:** [`edgequake/docker/extension-pins.sh`](../../../edgequake/docker/extension-pins.sh)  
**Cross-ref:** [SPEC-042 §013](../042-update-age-pgvector/013-version-feature-matrix-official-docs.md)

---

## Mission

Prove the **Cypher `$1` bind fix** (`cypher_*_bound`) works on **every supported PostgreSQL major** with the **correct Apache AGE catalog version** per tier — not only on the developer's local PG18 default.

**Invariant:** The prepared-statement contract for `cypher(graph, $$…$$, $1)` is identical on AGE **1.6.0** (PG16) and **1.7.0** (PG17/PG18). EdgeQuake graph hot-path Cypher must pass on **all three** before v0.14.2 ships.

---

## Official sources (ground truth)

| Topic | Official URL | What we rely on |
| ----- | ------------ | --------------- |
| AGE download matrix | [age.apache.org/download](https://age.apache.org/download/) | PG16→1.6.0, PG17→1.6.0 (site), PG18→**1.7.0** |
| AGE prepared statements | [AGE manual — Prepared Statements](https://age.apache.org/age-manual/master/advanced/prepared_statements.html) | 3rd arg **must** be `$1`; map keys exclude `$` |
| Literal vs parameter | [apache/age#315](https://github.com/apache/age/issues/315) | Inline `'{"k":"v"}'` → `must be a parameter` |
| PG16 AGE 1.6.0 | [GitHub PG16/v1.6.0-rc0](https://github.com/apache/age/releases/tag/PG16%2Fv1.6.0-rc0) | Legacy tier release |
| PG17 AGE 1.7.0 | [GitHub PG17/v1.7.0-rc0](https://github.com/apache/age/releases/tag/PG17%2Fv1.7.0-rc0) | Modern tier; upgrade script warning |
| PG18 AGE 1.7.0 | [GitHub PG18/v1.7.0-rc0](https://github.com/apache/age/releases/tag/PG18%2Fv1.7.0-rc0) | Recommended tier; fresh install |
| pgvector 0.8.3 | [pgvector CHANGELOG](https://github.com/pgvector/pgvector/blob/master/CHANGELOG.md) | All tiers |
| PostgreSQL 16 | [Release 16](https://www.postgresql.org/docs/release/16.0/) | EOL ~2028-11 |
| PostgreSQL 17 | [Release 17](https://www.postgresql.org/docs/release/17.0/) | EOL ~2029-11 |
| PostgreSQL 18 | [Release 18](https://www.postgresql.org/docs/release/18.0/) | EOL ~2030-11 |

### Download page vs GitHub (documented, not ignored)

| Tier | [age.apache.org/download](https://age.apache.org/download/) | EdgeQuake ship tag ([`extension-pins.sh`](../../../edgequake/docker/extension-pins.sh)) | Battle test expects |
| ---- | ----------------------------------------------------------- | --------------------------------------------------------------------------------------- | ------------------- |
| PG16 | AGE **1.6.0** | `PG16/v1.6.0-rc0` → catalog `1.6.0` | `extversion >= 1.6.0` |
| PG17 | AGE **1.6.0** (site) | `PG17/v1.7.0-rc0` → catalog `1.7.0` | `extversion >= 1.7.0` |
| PG18 | AGE **1.7.0** | `PG18/v1.7.0-rc0` → catalog `1.7.0` | `extversion >= 1.7.0` |

EdgeQuake follows **GitHub release tags** (same rule as SPEC-042). Operators on managed PG17 listing only 1.6.0 on the download page must verify catalog `extversion` before enabling 1.7-only features.

---

## EdgeQuake triple-track matrix (battle test targets)

| Profile | `EQ_POSTGRES_MAJOR` | Postgres image | AGE git ref | `EQ_AGE_MIN` | pgvector | Cypher param contract |
| ------- | ------------------- | -------------- | ----------- | ------------ | -------- | --------------------- |
| **pg16** | 16 | `edgequake-postgres:pg16` | `PG16/v1.6.0-rc0` | **1.6.0** | 0.8.3 | **REQUIRED** |
| **pg17** | 17 | `edgequake-postgres:pg17` | `PG17/v1.7.0-rc0` | **1.7.0** | 0.8.3 | **REQUIRED** |
| **pg18** | 18 | `edgequake-postgres:local` (default) | `PG18/v1.7.0-rc0` | **1.7.0** | 0.8.3 | **REQUIRED** |

---

## Battle test probes (per profile)

Each profile runs the **same Cypher param contract** plus **tier-specific version gates**.

| Probe ID | Layer | PG16 | PG17 | PG18 | Pass criteria |
| -------- | ----- | ---- | ---- | ---- | ------------- |
| **BT-044-TT-00** | Pin SSOT | ✅ | ✅ | ✅ | `check_extension_pins.sh` |
| **BT-044-TT-01** | SQL | ✅ | ✅ | ✅ | `server_version_num/10000` = expected major |
| **BT-044-TT-02** | SQL | ✅ | ✅ | ✅ | `age extversion >= EQ_AGE_MIN` |
| **BT-044-TT-03** | SQL | ✅ | ✅ | ✅ | `vector extversion >= 0.8.3` |
| **BT-044-TT-04** | SQL | ✅ | ✅ | ✅ | Negative: inline `::agtype` 3rd arg **rejected** |
| **BT-044-TT-05** | SQL | ✅ | ✅ | ✅ | Positive: `PREPARE … $1` + `EXECUTE` delete node |
| **BT-044-TT-06** | SQL | ✅ | ✅ | ✅ | Positive: `PREPARE … $1` + `EXECUTE` match read |
| **BT-044-TT-07** | SQL | ✅ | ✅ | ✅ | Positive: edge delete via `$1` map |
| **BT-044-TT-08** | Rust | ✅ | ✅ | ✅ | `spec022_postgres_cypher_prepared_node_crud_injection_safe` |
| **BT-044-TT-09** | Rust | ✅ | ✅ | ✅ | `postgres_integration::test_postgres_age_graph_crud` |
| **BT-044-TT-10** | Rust | ✅ | ✅ | ✅ | `storage_backend_contract` postgres batch + `has_node` |
| **BT-044-TT-11** | Source | ✅ | ✅ | ✅ | No `params_lit}'::agtype` in `cypher_exec.rs` |
| **BT-044-TT-12** | Tier | uuidv7 **absent** | uuidv7 absent | uuidv7 **present** | PG feature gate |
| **BT-044-TT-13** | Tier | id-column indexes N/A | passive 1.7.0 | passive 1.7.0 | Log only |
| **BT-044-TT-14** | SQL | ✅ | ✅ | ✅ | M042/M043 bootstrap apply idempotent |

Probes **BT-044-TT-04–07** are implemented in [`e2e/sql/cypher_param_contract.sql`](./e2e/sql/cypher_param_contract.sql) (AGE manual pattern).

Probes **BT-044-TT-08–10** exercise the **Rust** `cypher_*_bound` path against a live container (post P0a fix).

---

## AGE 1.6.0 vs 1.7.0 — what differs for SPEC-044

| Concern | PG16 (AGE 1.6.0) | PG17/PG18 (AGE 1.7.0) | Impact on Cypher bind fix |
| ------- | ---------------- | --------------------- | ------------------------ |
| Prepared `$1` third arg | ✅ [manual](https://age.apache.org/age-manual/master/advanced/prepared_statements.html) | ✅ Same API | **Must pass both** |
| `ON CREATE SET` in MERGE | ❌ [#2347 unreleased on 1.6](https://github.com/apache/age/issues/2347) | May differ | N/A — ingest uses per-key SET |
| RLS (#2309) | ❌ | ✅ | BT-044-40 optional when `EDGEQUAKE_AGE_RLS=1` |
| id-column indexes (#2117) | ❌ | ✅ | Passive — not probed in SPEC-044 |
| Upgrade duration | M043 `ALTER EXTENSION age UPDATE` | 1.6→1.7 script can be slow on large graphs | Ops — not bind contract |

**Conclusion:** The Cypher parameter bug (C-1) is **tier-agnostic**; triple-track testing guards against PG-major-specific sqlx/prepare behaviour, not AGE grammar differences.

---

## Execution

### Full triple-track (recommended release gate)

```bash
# Build images once (or SKIP_IMAGE_BUILD=1 if already built)
make postgres-image-build postgres-image-build-pg17 postgres-image-build-pg18

# SPEC-044 triple-track Cypher battle test (pg16 + pg17 + pg18)
make spec044-battle-test-all

# Or directly:
./specs/044-upgrate-issue-study/e2e/run_triple_track_cypher_proof.sh all
```

### Single profile (debug)

```bash
./specs/044-upgrate-issue-study/e2e/run_triple_track_cypher_proof.sh pg16
./specs/044-upgrate-issue-study/e2e/run_triple_track_cypher_proof.sh pg17
./specs/044-upgrate-issue-study/e2e/run_triple_track_cypher_proof.sh pg18
```

### SQL-only (no Rust compile)

```bash
SKIP_RUST_TESTS=1 ./specs/044-upgrate-issue-study/e2e/run_triple_track_cypher_proof.sh all
```

### Reports

Per-profile logs: `specs/044-upgrate-issue-study/e2e/reports/<profile>-cypher-report.txt`

---

## CI integration (required for v0.14.2)

| Job | Profiles | Probes |
| --- | -------- | ------ |
| `postgres-age-tests` (extend) | pg16 **or** matrix | BT-044-TT-08–11 minimum |
| New `spec044-triple-track` job | pg16 + pg17 + pg18 | Full `run_triple_track_cypher_proof.sh all` |
| `release-gates.yml` | pg18 default + pg16 spot | BT-044-TT-04–10 |

**Remove** `continue-on-error: true` on graph contract steps before enabling as hard gate ([008 plan P0c](./008-implementation-plan.md)).

---

## Relationship to SPEC-042 battle tests

| Suite | Focus | Composes with SPEC-044 |
| ----- | ----- | ---------------------- |
| `run_version_feature_battle_test.sh` | Extension versions, MERGE smoke | Run **before** SPEC-044 (infra healthy) |
| `run_triple_track_cypher_proof.sh` | **Cypher `$1` bind + Rust CRUD** | SPEC-044 specific |
| `run_all_battle_tests.sh` | Full SPEC-042 | Add step 6: SPEC-044 triple-track |

Suggested release order:

```text
check_extension_pins → SPEC-042 version matrix → SPEC-044 triple-track Cypher → app E2E
```

---

## Definition of done (triple-track)

- [ ] `run_triple_track_cypher_proof.sh all` exit 0 on CI
- [ ] BT-044-TT-04 fails on **unfixed** `cypher_exec.rs` (inline literal) on all tiers
- [ ] BT-044-TT-05–07 pass on **fixed** code on pg16, pg17, pg18
- [ ] BT-044-TT-08–10 pass on all tiers (no SKIP)
- [ ] Reports archived under `e2e/reports/`
- [ ] [009-cross-reference-matrix.md](./009-cross-reference-matrix.md) E2E-044-TT rows VERIFIED
