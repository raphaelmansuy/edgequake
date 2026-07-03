# SPEC-042 — Full Stack Developer Lens

**Audience:** Rust + Docker + frontend engineers  
**Question:** What do I touch when upgrading extensions?

---

## Architecture (triple-track)

```
                 edgequake-api (single binary)
                           │
     ┌─────────────────────┼─────────────────────┐
     ▼                     ▼                     ▼
 Dockerfile.postgres  Dockerfile.postgres.pg17  Dockerfile.postgres.pg18
 PG16 + AGE 1.6.0      PG17 + AGE 1.7.0         PG18 + AGE 1.7.0
 profile: pg16         profile: pg17            profile: pg18
```

SSOT: `extension-pins.sh` — `EQ_POSTGRES_PROFILE=pg16|pg17|pg18`

**Feature adoption SSOT:** [013-version-feature-matrix-official-docs.md](./013-version-feature-matrix-official-docs.md)

---

## Developer workflow

### Bump pgvector or AGE

1. Edit **`edgequake/docker/extension-pins.sh`** (SSOT).
2. Mirror ARG defaults in **`Dockerfile.postgres`** / `.pg17` / `.pg18`.
3. Run `make postgres-image-build*` — fails if `.control` mismatch.
4. Run `./specs/042-update-age-pgvector/e2e/run_version_feature_battle_test.sh all`.
5. Update **`013-version-feature-matrix-official-docs.md`** if new upstream features apply.

### Local dev with stale container

```bash
make db-stop
docker rm -f edgequake-postgres
make db-start   # rebuilds if pgvector < 0.8 or AGE < 1.6
make dev-bg
```

### Verify from API

```bash
curl -s http://localhost:8080/health | jq '.operational.migration'
```

Expected fields after SPEC-042:

```json
{
  "pgvector_extversion": "0.8.3",
  "pgvector_shipped_version": "0.8.3",
  "pgvector_iterative_scan_capable": true,
  "age_extversion": "1.6.0",
  "age_shipped_version": "1.6.0",
  "ready_for_traffic": true
}
```

---

## Code touch map

| Task | Files |
| ---- | ----- |
| Pin bump | `extension-pins.sh`, `Dockerfile.postgres` |
| Bootstrap logic | `reconcile/m042.rs`, `reconcile/m043.rs`, `helpers.rs` |
| Health DTO | `health_types.rs`, `health.rs` |
| Dev ergonomics | `Makefile` db-start |
| Tests | `tests/extension_versions_proof.rs` |

---

## Edge cases (developer checklist)

| Case | Behavior |
| ---- | -------- |
| External RDS without extension | `pgvector_available=false` → no degrade |
| Library newer than catalog | `ALTER EXTENSION UPDATE` in apply.sql |
| Library older than catalog | Exception caught — log NOTICE, `/ready` may 503 for pgvector |
| No DATABASE_URL / memory mode | migration snapshot omitted |
| Parallel HNSW reindex failure | Logged per-index; bootstrap continues |

---

## Do not duplicate

- **DRY:** Never add a third `ALTER EXTENSION` path — use `support/042|043/apply.sql` only.
- **SOLID:** Version comparison lives in `helpers::extension_version_at_least` — reconcile modules call it.
