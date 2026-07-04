# SPEC-042 — Update Apache AGE + pgvector (GitHub #161)

**Spec:** `042-update-age-pgvector`  
**Date:** 2026-07-03  
**Method:** Code is law — every claim maps to file, migration, or E2E proof  
**Trigger:** [Issue #161](https://github.com/raphaelmansuy/edgequake/issues/161)

---

## Mission (reformulated — triple-track)

EdgeQuake supports **three PostgreSQL major tiers** with a **single application binary**:

| Tier | PostgreSQL | Extensions | Role |
| ---- | ---------- | ---------- | ---- |
| **Legacy supported** | **16** | pgvector 0.8.3 + AGE 1.6.0 | Existing deployments — no forced migration |
| **Modern supported** | **17** | pgvector 0.8.3 + AGE **1.7.0** | Managed PG17 — full #161 on AGE |
| **Recommended** | **18** | pgvector 0.8.3 + AGE **1.7.0** | New installs — longest support runway |

**First principle:** Application schema/bootstrap is PG-major **agnostic**; Docker images and extension pins are PG-major **specific**. See [012-dual-pg-major-compatibility.md](./012-dual-pg-major-compatibility.md).

| Phase | Target | Status |
| ----- | ------ | ------ |
| **A — PG16 legacy** | pgvector 0.8.3 + AGE 1.6.0 | ✅ |
| **B — PG18 recommended** | pgvector 0.8.3 + AGE 1.7.0 + runbook | ✅ |
| **C — Multi-major policy** | PG16 + PG17 + PG18 | ✅ |
| **D — Official docs battle test** | Feature matrix + E2E probes | ✅ |
| **E — Feature adoption** | halfvec, AGE RLS, uuidv7, COPY loader | 📋 Planned |
| **F — Release cut** | PG18 default `make dev` + CI triple-publish | ✅ |

---

## TL;DR — extension pins by major

| PG Major | pgvector | Apache AGE | Docker image | #161 |
| -------- | -------- | ---------- | ------------ | ---- |
| **16** (legacy) | 0.8.3 | 1.6.0 | `Dockerfile.postgres` | Partial |
| **17** (modern) | 0.8.3 | **1.7.0** | `Dockerfile.postgres.pg17` | **Full** |
| **18** (recommended) | 0.8.3 | **1.7.0** | `Dockerfile.postgres.pg18` | **Full** |

Profile SSOT: `extension-pins.sh` (`EQ_POSTGRES_PROFILE=pg16|pg17|pg18`)

**Optional major migration:** [`scripts/migrate_postgres_major.sh`](../../scripts/migrate_postgres_major.sh) — any major → any major (pins auto-detected from target).

---

## Documents

| File | Lens |
| ---- | ---- |
| [016-dry-solid-improvements.md](./016-dry-solid-improvements.md) | **DRY/SOLID — unified Dockerfile, battle test results** |
| [015-issue-275-hnsw-dimension-guard.md](./015-issue-275-hnsw-dimension-guard.md) | **#275 HNSW dimension guard** |
| [014-feature-adoption-plan.md](./014-feature-adoption-plan.md) | **Phase E — halfvec, RLS, uuidv7, COPY** |
| [013-version-feature-matrix-official-docs.md](./013-version-feature-matrix-official-docs.md) | Official docs battle test |
| [012-dual-pg-major-compatibility.md](./012-dual-pg-major-compatibility.md) | PG16+17+18 decision |
| [010-postgres-18-migration.md](./010-postgres-18-migration.md) | Major migration runbook |
| [011-postgres-18-upgrade-path-matrix.md](./011-postgres-18-upgrade-path-matrix.md) | Upgrade paths |
| … | See prior index entries |

---

## E2E proof (all tiers)

### v0.14.0 Release Proof (published GHCR images)

Full E2E verification against **published Docker images** — see [v0140-release-proof/](./e2e/v0140-release-proof/README.md).

```bash
# Run against published GHCR images (no local build)
./specs/042-update-age-pgvector/e2e/run_v0140_release_e2e.sh all
```

All three tiers passed 8 checks: image pull, container startup, extension verification, PG version gate, 384-d HNSW ANN, halfvec HNSW, AGE Cypher lifecycle, and ingestion schema.

### Local build + battle tests

```bash
# Per-profile builds (individual Dockerfiles)
make postgres-image-build          # PG16
make postgres-image-build-pg17     # PG17
make postgres-image-build-pg18     # PG18

# Unified build (DRY — single Dockerfile, ARGs from extension-pins.sh)
EQ_POSTGRES_PROFILE=pg16 make postgres-image-build-unified
EQ_POSTGRES_PROFILE=pg17 make postgres-image-build-unified
EQ_POSTGRES_PROFILE=pg18 make postgres-image-build-unified

# Verification + battle tests
scripts/check_extension_pins.sh all
./specs/042-update-age-pgvector/e2e/run_version_feature_battle_test.sh all  # official-docs probes
./specs/042-update-age-pgvector/e2e/run_all_battle_tests.sh                 # full suite (#275 + Phase E)
make spec042-battle-test-all

# PG16 → PG17 or PG18 (target pins auto-detected)
./scripts/migrate_postgres_major.sh --source-url "$SRC" --target-url "$DST"
./specs/042-update-age-pgvector/e2e/run_pg18_migration_procedure.sh  # PG16→PG18 sample
```

---

## Related

| Spec / Issue | Relationship |
| ------------ | ------------ |
| [Issue #161](https://github.com/raphaelmansuy/edgequake/issues/161) | Full on PG17/PG18; partial on PG16 |
| [Issue #275](https://github.com/raphaelmansuy/edgequake/issues/275) | HNSW dim guard — fixed in M071 + `AnnIndexPolicy` |
