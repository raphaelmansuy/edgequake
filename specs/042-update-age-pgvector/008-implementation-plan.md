# SPEC-042 — Implementation Plan

**Status:** Implemented  
**Principle:** Minimal diff — infrastructure already had M042/M043; this spec adds SSOT, visibility, gates, proof.

---

## Phase 0 — Research ✅

- [x] Official pgvector 0.8.3 ([PGXN](https://pgxn.org/dist/vector))
- [x] Official AGE PG16 1.6.0 ([download page](https://age.apache.org/download/))
- [x] Issue #161 scope vs PG16 constraint

---

## Phase 1 — SSOT pins (DRY)

| Step | File | Action |
| ---- | ---- | ------ |
| 1.1 | `edgequake/docker/extension-pins.sh` | Create SSOT exports |
| 1.2 | `verify-postgres-extensions.sh` | Source SSOT |
| 1.3 | `Dockerfile.postgres` | Comment + align with SSOT |
| 1.4 | `scripts/check_extension_pins.sh` | Grep gate Dockerfile ↔ SSOT |

---

## Phase 2 — Bootstrap hardening (SOLID)

| Step | File | Action |
| ---- | ---- | ------ |
| 2.1 | `helpers.rs` | Add `extension_version_at_least()` |
| 2.2 | `reconcile/m043.rs` | Re-apply when catalog < shipped (mirror m042) |
| 2.3 | `Migration043Report` | Add `shipped_extversion` field |

---

## Phase 3 — Operator visibility

| Step | File | Action |
| ---- | ---- | ------ |
| 3.1 | `health_types.rs` | Extension version fields on `MigrationHealthSnapshot` |
| 3.2 | `health.rs` | Populate from bootstrap report |

---

## Phase 4 — Dev ergonomics

| Step | File | Action |
| ---- | ---- | ------ |
| 4.1 | `Makefile` db-start | AGE version check (parallel to pgvector) |
| 4.2 | `Makefile` db-start | `ALTER EXTENSION age UPDATE` after vector |

---

## Phase 5 — E2E proof

| Step | Command | Pass criteria |
| ---- | ------- | ------------- |
| 5.1 | `make postgres-image-build` | verify script green |
| 5.2 | `run_extension_upgrade_proof.sh` | extversion ≥ pins |
| 5.3 | `cargo test extension_versions_proof` | bootstrap report fields |
| 5.4 | `scripts/check_extension_pins.sh` | no drift |

---

## Phase 6 — Docs sync

| Step | File |
| ---- | ---- |
| 6.1 | `specs/022-edgequake-study/README.md` | Update version table to 0.8.3 |
| 6.2 | CHANGELOG | Entry for #161 |

---

## Phase B — PostgreSQL 18 (SPEC-042-B) 🎯 ACTIVE

**Mission:** Close [#161](https://github.com/raphaelmansuy/edgequake/issues/161) with **AGE 1.7.0** on **PostgreSQL 18**.

| Step | File | Action | Status |
| ---- | ---- | ------ | ------ |
| B1 | `extension-pins.sh` | PG18 profile (`EQ_POSTGRES_PROFILE=pg18`) | ✅ |
| B2 | `Dockerfile.postgres.pg18` | PG18 + AGE 1.7.0 image | ✅ |
| B3 | `010-postgres-18-migration.md` | Operator runbook | ✅ |
| B4 | `011-postgres-18-upgrade-path-matrix.md` | Upgrade paths | ✅ |
| B5 | `scripts/migrate_postgres_major.sh` | Procedural SSOT | ✅ |
| B6 | `e2e/run_pg18_migration_procedure.sh` | E2E proof | ✅ |
| B7 | `Makefile` | `postgres-image-build-pg18` | ✅ |
| B8 | `docker-compose.yml` | PG18 **recommended** profile; PG16 **retained** | ✅ |
| B9 | `reconcile/m043.rs` | Gate AGE min 1.7.0 when PG18 detected | ✅ |
| B10 | CI release | Triple-tier postgres publish + battle test targets | ✅ |

---

## Phase C — Multi-major (PG16 + PG17 + PG18)

| Step | File | Action | Status |
| ---- | ---- | ------ | ------ |
| C1 | `012-dual-pg-major-compatibility.md` | Triple-track decision | ✅ |
| C2 | `Dockerfile.postgres.pg17` | PG17 + AGE 1.7.0 | ✅ |
| C3 | `extension-pins.sh` | `pg17` profile | ✅ |
| C4 | `make postgres-image-build-pg17` | Build + verify | ✅ |
| C5 | `migrate_postgres_major.sh` | Auto-detect target 16/17/18 | ✅ |
| C6 | `check_extension_pins.sh all` | Three-way pin gate | ✅ |

## Phase D — Battle test ✅

| Step | File | Action | Status |
| ---- | ---- | ------ | ------ |
| D1 | `013-version-feature-matrix-official-docs.md` | Feature matrix vs official upstream | ✅ |
| D2 | `e2e/run_version_feature_battle_test.sh` | Triple-tier executable probes | ✅ |

---

## Phase E — Feature adoption (planned)

**Detail:** [014-feature-adoption-plan.md](./014-feature-adoption-plan.md)

| Step | Feature | Min tier | Status |
| ---- | ------- | -------- | ------ |
| E-01 | **halfvec storage** (~50% disk) | pgvector 0.8+ (all) | 📋 |
| E-02 | **AGE 1.7 RLS** tenant isolation | PG17+ / AGE 1.7 | 📋 |
| E-03 | **`uuidv7()` document IDs** | PG18 | 📋 |
| E-04 | **AGE pg COPY bulk loader** | PG17+ / AGE 1.7 | 📋 |

Recommended order: E-03 → E-01 → E-04 → E-02 (see 014 for rationale).

```bash
./specs/042-update-age-pgvector/e2e/run_version_feature_battle_test.sh all
# Phase E acceptance probes (when implemented): BT-PV-04, E-02.7, E-03.5, E-04.6
```
