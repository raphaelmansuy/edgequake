# PostgreSQL Triple-Track — SPEC-042 Migration Guide

**Spec:** `042-update-age-pgvector` · [Issue #161](https://github.com/raphaelmansuy/edgequake/issues/161)  
**Date:** 2026-07-03

EdgeQuake ships **one application binary** against **three supported PostgreSQL major tiers**. Extension pins and Docker images are major-specific; schema bootstrap (M042/M043/M078) is major-agnostic.

---

## Tier matrix

| Tier | PostgreSQL | pgvector | Apache AGE | GHCR tag | `make dev` profile |
| ---- | ---------- | -------- | ---------- | -------- | ------------------ |
| **Legacy supported** | 16 | 0.8.3 | 1.6.0 | `:VERSION-pg16` / `latest-pg16` | `EQ_POSTGRES_PROFILE=pg16` |
| **Modern supported** | 17 | 0.8.3 | 1.7.0 | `:VERSION-pg17` / `latest-pg17` | `EQ_POSTGRES_PROFILE=pg17` |
| **Recommended** | 18 | 0.8.3 | 1.7.0 | `:VERSION` / `latest` (default) | `EQ_POSTGRES_PROFILE=pg18` (default) |

**SSOT for pins:** `edgequake/docker/extension-pins.sh`

---

## First principles

1. **Extension version is infrastructure** — catalog `extversion` must match shipped `.so` files after image rebuild (`ALTER EXTENSION … UPDATE` at db-start + M042/M043 bootstrap).
2. **PG major bounds AGE ceiling** — AGE 1.7.0 requires PG17 or PG18; PG16 stays on AGE 1.6.0.
3. **pgvector ≥ 0.8.0 is a readiness gate** — `/ready` returns 503 when iterative-scan capability is missing.
4. **Major bump ≠ extension bump** — PG16 → PG18 requires `pg_dump` / `pg_restore` or `pg_upgrade`, not `ALTER EXTENSION` alone.

---

## New installs (recommended — PG18)

```bash
# Local development (default since SPEC-042)
make dev                    # builds PG18 via docker-compose

# Or explicit profile
EQ_POSTGRES_PROFILE=pg18 make dev

# Prebuilt GHCR stack (after release publish)
make stack                  # pulls edgequake-postgres:latest (= PG18)
```

Verify extensions after startup:

```bash
curl -s http://localhost:8080/health | python3 -m json.tool | grep -E 'pgvector|age_'
# Expect: pgvector_extversion 0.8.3, age_extversion 1.7.0 on PG18
```

---

## Staying on PG16 (legacy)

Existing PG16 deployments are **not forced to migrate**.

```bash
# Local dev on PG16
EQ_POSTGRES_PROFILE=pg16 make dev

# GHCR prebuilt
EDGEQUAKE_POSTGRES_TAG=latest-pg16 make stack
```

Issue #161 is **partially** closed on PG16 (AGE 1.6.0 ceiling). Full closure requires PG17 or PG18.

---

## Optional major migration (PG16 → PG17 or PG18)

Use the procedural SSOT script — target pins are auto-detected from the destination server:

```bash
./scripts/migrate_postgres_major.sh \
  --source-url "$PG16_DATABASE_URL" \
  --target-url "$PG18_DATABASE_URL" \
  --dump-file /tmp/edgequake-pg16-$(date +%Y%m%d).dump
```

E2E proof (dev):

```bash
./specs/042-update-age-pgvector/e2e/run_pg18_migration_procedure.sh
```

### Rollback

Restore the PG16 dump into a PG16 container (`EDGEQUAKE_POSTGRES_TAG=latest-pg16`). Application schema rows in `_sqlx_migrations` travel with the dump.

---

## Operator checklist

| Step | Command / check |
| ---- | ---------------- |
| Pin consistency | `make check-extension-pins` |
| Build all tiers | `make postgres-image-build && make postgres-image-build-pg17 && make postgres-image-build-pg18` |
| Battle test | `make postgres-battle-test` |
| Bootstrap proof | `cargo test -p edgequake-api --features postgres migration_readiness_proof` |
| Health visibility | `GET /health` → `migration.pgvector_*`, `migration.age_*` |

---

## CI/CD published images

On each release tag, `.github/workflows/release-docker.yml` publishes:

| Image | Architecture |
| ----- | ------------ |
| `ghcr.io/raphaelmansuy/edgequake-postgres:VERSION` | PG18 default |
| `ghcr.io/raphaelmansuy/edgequake-postgres:VERSION-pg16` | PG16 legacy |
| `ghcr.io/raphaelmansuy/edgequake-postgres:VERSION-pg17` | PG17 modern |
| `ghcr.io/raphaelmansuy/edgequake-postgres:VERSION-pg18` | PG18 explicit |

---

## Related docs

- [SPEC-042 index](../../specs/042-update-age-pgvector/000-index.md)
- [PG18 migration runbook](../../specs/042-update-age-pgvector/010-postgres-18-migration.md)
- [Triple-track decision](../../specs/042-update-age-pgvector/012-dual-pg-major-compatibility.md)
- [Bootstrap first principles](./bootstrap-first-principles.md)
