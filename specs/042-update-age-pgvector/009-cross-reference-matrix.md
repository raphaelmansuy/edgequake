# SPEC-042 — Cross-Reference Matrix

**Method:** Every row = claim → evidence → test → status

---

## Issue #161 traceability

| ID | Claim | Code law | E2E proof | Status |
| -- | ----- | -------- | --------- | ------ |
| I-01 | pgvector latest stable 0.8.3 | `extension-pins.sh`, PGXN | verify script | ✅ |
| I-02 | AGE latest for PG16 is 1.6.0 | `extension-pins.sh`, age.apache.org | verify script | ✅ |
| I-03 | AGE 1.7.0 requires PG17+ | `006-postgres-expert-lens.md` | N/A doc | ✅ |
| I-04 | Auto catalog upgrade | M042/M043 apply.sql | bootstrap proof | ✅ |
| I-05 | Health exposes versions | `health_types.rs` | curl /health | ✅ |

---

## REQ traceability

| REQ | IDs | Verified by |
| --- | --- | ----------- |
| REQ-042-01 | I-01, I-02 | `check_extension_pins.sh` |
| REQ-042-02 | I-01, I-02 | `postgres-image-build` |
| REQ-042-03 | I-04 | `support/042/apply.sql` |
| REQ-042-04 | I-04 | `support/043/apply.sql` |
| REQ-042-05 | I-05 | `extension_versions_proof.rs` |
| REQ-042-06 | — | `Makefile` db-start |
| REQ-042-07 | REQ-042-01 | verify script sources pins |
| REQ-042-08 | I-04, I-05 | `run_extension_upgrade_proof.sh` |

---

## SPEC lineage

| Prior spec | Relationship |
| ---------- | ------------ |
| SPEC-022 | Identified 0.7.4 gap — superseded pins |
| SPEC-006 | Migration bootstrap pattern origin |
| SPEC-039 | Docker E2E proof template |
| SPEC-041 | Three-layer migration repair pattern |

---

## File dependency graph

```
extension-pins.sh (pg16 | pg18 profiles)
    ├── Dockerfile.postgres / Dockerfile.postgres.pg18
    ├── verify-postgres-extensions.sh
    ├── Makefile (postgres-image-build*)
    ├── scripts/migrate_postgres_major.sh
    └── e2e/run_*_proof.sh

migrations/support/042/apply.sql
    └── reconcile/m042.rs → migration_bootstrap/mod.rs → health.rs

migrations/support/043/apply.sql
    └── reconcile/m043.rs → migration_bootstrap/mod.rs → health.rs
```

---

## Issue closure checklist (#161)

**Phase A (PG16):**

1. pgvector 0.8.3 + AGE 1.6.0 in Docker image ✅
2. M042/M043 bootstrap ✅

**Phase B (PG18 — full #161 closure, optional migrate):**

1. Link `010-postgres-18-migration.md` + `011-postgres-18-upgrade-path-matrix.md`
2. `make postgres-image-build-pg18` green
3. E2E migration procedure exit 0
4. Close #161: PG18 = full; PG16 = latest-for-major

**Phase C (dual-track — PG16 retained):**

1. Link `012-dual-pg-major-compatibility.md`
2. Both `postgres-image-build` + `postgres-image-build-pg18` green
3. No release requires PG18 exclusively

---

## Phase B traceability (SPEC-042-B)

| ID | Claim | Code law | E2E proof | Status |
| -- | ----- | -------- | --------- | ------ |
| B-01 | PG18 image ships AGE 1.7.0 | `Dockerfile.postgres.pg18` | verify script | ✅ |
| B-02 | PG18 profile in SSOT | `extension-pins.sh` | `check_extension_pins.sh pg18` | ✅ |
| B-03 | Major migration procedure | `migrate_postgres_major.sh` | dry-run + restore | ✅ |
| B-04 | Operator runbook | `010-postgres-18-migration.md` | — | ✅ |
| B-05 | Upgrade paths | `011-postgres-18-upgrade-path-matrix.md` | — | ✅ |
| B-06 | PG16→PG18 E2E | `run_pg18_migration_procedure.sh` | exit 0 | ✅ |

---

## Phase C traceability (dual-major)

| ID | Claim | Evidence | Status |
| -- | ----- | -------- | ------ |
| C-01 | PG16 remains supported | `Dockerfile.postgres` | ✅ |
| C-02 | PG17 modern tier | `Dockerfile.postgres.pg17` | ✅ |
| C-03 | PG18 recommended tier | `Dockerfile.postgres.pg18` | ✅ |
| C-04 | Single app binary | No PG-major Cargo features | ✅ |
| C-05 | Triple-track documented | `012-*.md` | ✅ |
| C-06 | Migrate script any→any | `migrate_postgres_major.sh` | ✅ |
| C-07 | All pin profiles | `check_extension_pins.sh all` | ✅ |
| D-01 | Official docs feature matrix | `013-version-feature-matrix-official-docs.md` | ✅ |
| D-02 | Battle test E2E all tiers | `run_version_feature_battle_test.sh` | ✅ |
| D-03 | Iterative scan adopted (pgvector ≥0.8) | `search_tuning.rs` | ✅ |
| D-04 | halfvec / AGE RLS planned | 014 Phase E | 📋 |

---

## Phase E traceability (feature adoption)

| ID | Claim | Evidence | Status |
| -- | ----- | -------- | ------ |
| E-01 | halfvec ~50% disk savings | [014 § E-01](./014-feature-adoption-plan.md#e-01--halfvec-storage-50-disk-savings) | 📋 |
| E-02 | AGE RLS tenant isolation | [014 § E-02](./014-feature-adoption-plan.md#e-02--age-17-rls-for-tenant-isolation) | 📋 |
| E-03 | uuidv7 document IDs on PG18 | [014 § E-03](./014-feature-adoption-plan.md#e-03--uuidv7-for-document-ids-pg18) | 📋 |
| E-04 | AGE pg COPY bulk loader | [014 § E-04](./014-feature-adoption-plan.md#e-04--age-pg-copy-bulk-loader) | 📋 |
| E-05 | PG16 tier unaffected | REQ-042E-05 | 📋 |
