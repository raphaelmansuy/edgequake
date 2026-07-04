# SPEC-042/017 — Fix PG18+ Volume Mount Crash (#280)

**Date:** 2026-07-04
**Issue:** [#280](https://github.com/raphaelmansuy/edgequake/issues/280)
**Upstream:** [docker-library/postgres#1259](https://github.com/docker-library/postgres/pull/1259)

## 5-Why Root Cause Analysis

| # | Why | Answer |
|---|-----|--------|
| 1 | Why does the PG18 container crash? | The entrypoint detects data at `/var/lib/postgresql/data` but PG18 expects `pg_ctlcluster` layout at `/var/lib/postgresql/18/docker` |
| 2 | Why is data at the wrong path? | `docker-compose.quickstart.yml` hardcodes `edgequake-pg-data:/var/lib/postgresql/data` |
| 3 | Why is it hardcoded? | Written when PG16 was default — `/var/lib/postgresql/data` was universal for PG<=17 |
| 4 | Why wasn't it updated for PG18? | Internal compose files were parameterized, but the quickstart (user entry point) was missed |
| 5 | Why was the quickstart missed? | No E2E test verifies volume persistence across container restarts for the quickstart compose file |

**Root cause:** The `/var/lib/postgresql/data` mount path is incompatible with PG18+, which uses `pg_ctlcluster` directory layout.

## First Principle

The volume mount path **MUST match the `VOLUME` declaration** in the base Docker image:

| PG Major | Base Image `VOLUME` | Correct Mount Path | Reason |
|----------|--------------------|--------------------|--------|
| PG<=17 | `/var/lib/postgresql/data` | `/var/lib/postgresql/data` | Traditional `PGDATA` layout |
| PG18+ | `/var/lib/postgresql` | `/var/lib/postgresql` | New `pg_ctlcluster` layout ([docker-library/postgres#1259](https://github.com/docker-library/postgres/pull/1259)) |

Mounting at a parent of the declared `VOLUME` path does NOT override the anonymous volume — data is lost on container recreation. This is a Docker design constraint.

## Affected Files

| File | Image Default | Old Mount | Fix |
|------|--------------|-----------|-----|
| `docker-compose.quickstart.yml` | PG18 (`latest`) | `/var/lib/postgresql/data` | `/var/lib/postgresql` |
| `docs/cookbook.md` | PG18 example | `/var/lib/postgresql/data` | `/var/lib/postgresql` |
| `docs/integrations/open-webui.md` | PG18 example | `/var/lib/postgresql/data` | `/var/lib/postgresql` |

**No change needed:**
- `edgequake/docker/docker-compose.test.yml` — uses `Dockerfile.postgres` (PG16), mount at `/var/lib/postgresql/data` is correct
- `edgequake/docker/docker-compose.yml` — already parameterized `${POSTGRES_DATA_DIR:-/var/lib/postgresql}`
- `edgequake/docker/docker-compose.prebuilt.yml` — already parameterized
- `Makefile` — PG-major branching logic already correct

## E2E Battle Test

Verify for each profile:
1. PG16: mount at `/var/lib/postgresql/data` — data persists across restart
2. PG17: mount at `/var/lib/postgresql/data` — data persists across restart
3. PG18: mount at `/var/lib/postgresql` — data persists across restart, no crash loop
