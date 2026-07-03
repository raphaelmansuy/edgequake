# SPEC-042 — Risk Analysis

**Method:** Impact × likelihood matrix with mitigations tied to code.

---

## Risk matrix

| ID | Risk | Impact | Likelihood | Mitigation | Owner |
| -- | ---- | ------ | ---------- | ---------- | ----- |
| R-01 | pgvector catalog stays 0.7.x on old volume | **High** — filtered ANN broken, `/ready` 503 | Medium | M042 apply + Makefile rebuild gate | Bootstrap |
| R-02 | AGE catalog/library mismatch after image rebuild | **High** — Cypher crashes | Low | M043 apply + db-start AGE gate | Bootstrap |
| R-03 | HNSW REINDEX locks tables at startup | **Medium** — slow startup | Medium | Per-index try/catch; ops run off-hours | apply.sql |
| R-04 | AGE 1.5→1.6 GIN index incompatibility | **Medium** — upgrade fails | Low | Exception handler; M038 reconcile | apply.sql |
| R-05 | Issue #161 expectation of AGE 1.7.0 | **Low** — confusion | High | Document PG16 ceiling in spec + CHANGELOG | PO |
| R-06 | Pin drift Dockerfile vs extension-pins.sh | **Medium** — false green verify | Medium | `check_extension_pins.sh` in E2E | CI |
| R-07 | External RDS without extension packages | **High** — startup fail | Low | connection.rs graceful CREATE EXTENSION | Storage |
| R-08 | SIMD SIGILL on Apple Silicon Docker | **Medium** — pgvector build fail | Low | `OPTFLAGS=""` in Dockerfile | Docker |
| R-09 | Large graph AGE upgrade on PG18 migrate | **High** — hours downtime | Low | Maintenance window; optional path | Ops |
| R-10 | Dual-major CI drift (PG16 image breaks) | **Medium** — false confidence on PG18-only CI | Medium | Both E2E proofs required (REQ-042C-06) | CI |
| R-11 | AGE 1.7-only code merged breaks PG16 | **High** — graph failures on PG16 | Medium | REQ-042C-04 review gate; intersection Cypher | Eng |
| R-12 | Operator assumes PG18 mandatory | **Low** — unnecessary migration | Medium | Dual-track docs + PO messaging | Docs |
| R-13 | M071 HNSW on dim > 2000 vector column ([#275](https://github.com/raphaelmansuy/edgequake/issues/275)) | **High** — upgrade blocker | Medium | `AnnIndexPolicy` + M071 guard + checksum repair | Bootstrap |

---

## Upgrade path matrix (includes dual-track)

| From state | To state | Path | Required? |
| ---------- | -------- | ---- | --------- |
| Fresh install | PG16 0.8.3 + 1.6.0 | `Dockerfile.postgres` | Optional tier |
| Fresh install | PG18 0.8.3 + 1.7.0 | `Dockerfile.postgres.pg18` | **Recommended** |
| PG16 prod volume | Stay PG16 | M042/M043 only | ✅ Valid — no major migrate |
| PG16 prod volume | PG18 + 1.7.0 | `migrate_postgres_major.sh` | Opt-in |
| pgvector 0.7.4 on either major | 0.8.3 | Rebuild image → M042 | Either tier |

---

## Rollback strategy

1. **Application:** Revert to previous EdgeQuake tag — schema migrations forward-only; no downgrade needed for extension pins.
2. **Extensions:** PostgreSQL does not support `ALTER EXTENSION DOWNGRADE` — rollback = restore volume snapshot or rebuild DB.
3. **Mitigation:** Backup before major extension jump (`pg_dump` / volume snapshot).

---

## Monitoring signals

| Signal | Source | Alert if |
| ------ | ------ | -------- |
| `pgvector_iterative_scan_capable=false` | `/health` | > 5 min after deploy |
| `ready_for_traffic=false` | `/ready` | Kubernetes 503 sustained |
| `migration_042_degraded` | backend logs | grep in Loki |
| extversion < shipped | `/health` operational.migration | drift after image bump |
