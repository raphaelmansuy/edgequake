---
title: "Upgrade to EdgeQuake v0.24.3"
description: "Per-release upgrade notes for EdgeQuake v0.24.3: what changed and what to run."
---

# Upgrade to EdgeQuake v0.24.3

> **From:** v0.24.2 · **To:** v0.24.3 · **CD:** GHCR (`edgequake`, `edgequake-frontend`, `edgequake-postgres`)

Ops patch: SPEC-112 shared-PostgreSQL connection-pool harden + UTF-8 truncate SSOT.
No new migrations required for this cut. Cluster A / Clear All remain on **v0.24.2**
([upgrade-to-0.24.2.md](upgrade-to-0.24.2.md)).

## Highlights

| Area | What changed |
|------|----------------|
| Pool identity | `application_name=edgequake:<role>` (`query` / `ingest` / `queue` / `admin`) |
| Reaping | Explicit sqlx idle (600s) + max lifetime (1800s); env overrides |
| Budget | Startup check: `instances × pool_sum` vs PG capacity (`warn` default / `fail`) |
| Shutdown | Graceful HTTP drain then `pool.close()` on all role pools |
| Health | `/health` includes per-role `db_pools` util |
| UTF-8 | Truncate SSOT — no mid-codepoint panics on span/LLM previews |

## Sequence

```text
1. Backup (optional for this patch — no schema train)
2. Deploy v0.24.3 API (+ frontend if you pin it)
3. For shared PG with co-tenants (QL etc.), set shared-DB pool sizes before restart:

   export EDGEQUAKE_DB_POOL_SIZE_QUERY=8
   export EDGEQUAKE_DB_POOL_SIZE_INGEST=6
   export EDGEQUAKE_DB_POOL_SIZE_QUEUE=2
   export EDGEQUAKE_DB_POOL_SIZE_ADMIN=1
   export EDGEQUAKE_DB_POOL_INSTANCE_COUNT=<replicas including rollout overlap>
   # optional: EDGEQUAKE_DB_POOL_BUDGET_MODE=fail

4. Prefer SIGTERM so pools close after drain (SIGKILL skips close)
5. Verify attribution + health
```

Detail: [`specs/112-connection-pool/07-ops-runbook.md`](../../specs/112-connection-pool/07-ops-runbook.md),
[`configuration.md`](configuration.md) § Connection pool.

## Verify

```bash
curl -s http://localhost:8080/health | jq -r '.version'          # expect 0.24.3
curl -s http://localhost:8080/health | jq '.db_pools'            # per-role max/util
# On shared PG:
# SELECT application_name, state, count(*) FROM pg_stat_activity
#   WHERE backend_type = 'client backend' GROUP BY 1, 2;
# Expect edgequake:query|ingest|queue|admin (not empty)
```

## Out of scope in this cut

- Raising PostgreSQL `max_connections` as the product fix
- Mandatory PgBouncer (recommended for shared fleets — see ops runbook)
- #361 bulk-upload concurrency
- SPEC-111 Cluster A / Clear All (already in v0.24.2)
