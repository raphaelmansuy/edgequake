---
title: "Upgrade to EdgeQuake v0.24.2"
description: "Per-release upgrade notes for EdgeQuake v0.24.2: what changed and what to run."
---

# Upgrade to EdgeQuake v0.24.2

> **From:** v0.24.0 / v0.24.1 · **To:** v0.24.2 · **CD:** GHCR (`edgequake`, `edgequake-frontend`, `edgequake-postgres`)

Ship-with-runbook release: Cluster A + Clear All (#366) + SPEC-110 migrate 118/121 +
SPEC-109 reasoning effort + **SPEC-091/098 fleet-mirror parse** for entity names
containing `->`. Confirm-drop remains consent-gated.

## Highlights

| Area | What changed |
|------|----------------|
| SPEC-111 Cluster A | Drop readiness = coverage; fleet = provenance-only; KV residue cast; iw2 normalize |
| Clear All / #366 | Authoritative empty membership; wipe purges residual KV list ghosts |
| SPEC-110 | Migrations **118**/**121** `DISTINCT ON` (checksum repair allowlist for already-applied bodies) |
| SPEC-109 | Configurable `reasoning_effort` (API + WebUI + cache key) |
| SPEC-091/098 | Relationship legacy key uses **last** `->` (fixes `999/1000` KG persist) |
| Migration **143** | `legacy_vector_id` columns for provenance stamp |
| Cancel/purge | Worker persist tolerates task-row purge races (no false ERROR) |

## Sequence

```text
1. Backup (pg_dump -Fc / snapshot)
2. Deploy v0.24.2 images (API + frontend; postgres image if you pin it)
3. edgequake migrate
   - expandable path includes 143–144 (workspace-scoped legacy_vector_id unique)
4. If checksum drift on 125/131 (or 118/121 after SPEC-110 body repair):
   EDGEQUAKE_ALLOW_CHECKSUM_REPAIR=125,131,118,121 DATABASE_URL=… edgequake migrate
   (local `make_dev` migrate already passes the allowlist)
5. Engine jobs if mid-cutover:
   - w3-chunk-embedding-backfill
   - iw2-fleet-embedding-backfill
   - iw2-fleet-provenance-stamp
6. dry-run / guard → edgequake migrate --confirm-drop (only when ready)
7. edgequake migrate   # deferred 142 emptiness assert
8. Reprocess any docs Failed with typed fleet mirror 999/N + arrow-in-name misses
```

Detail: [`specs/111-issues/09-ops-runbook.md`](../../specs/111-issues/09-ops-runbook.md),
[`specs/111-issues/11-release-partner-notes.md`](../../specs/111-issues/11-release-partner-notes.md).

Near-miss KG persist (`999/1000`): [`spec098-entity-spine-ensure.md`](spec098-entity-spine-ensure.md) § Hot path item 4 — **reprocess after upgrade**, do not re-run spine ensure solely for that class.

## Verify

```bash
curl -s http://localhost:8080/health | jq -r '.version'   # expect 0.24.2
curl -s http://localhost:8080/api-docs/openapi.json | jq -r '.info.version'
```

Clear All on a workspace with residual KV ghosts → list stays **0**.  
Argus-class markdown with `->` in entity names → KG persist Completes after reprocess.

## Out of scope in this cut

- #361 bulk-upload concurrency
- Auto confirm-drop
- Full rewrite of vector id delimiters (target names with `->` remain ambiguous)
