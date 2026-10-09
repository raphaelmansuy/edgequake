---
title: "Upgrade to EdgeQuake v0.32.0"
description: "Per-release upgrade notes for EdgeQuake v0.32.0: what changed and what to run."
---

# Upgrade to EdgeQuake v0.32.0

> **From:** v0.31.0 · **To:** v0.32.0 · **CD:** GHCR (`edgequake`,
> `edgequake-frontend`, `edgequake-postgres`, `edgequake-keycloak`)

Minor cut: tenant-scoped data-access hardening and query/convert reliability.
Schema train moves **166 → 168**. Run migrate before `/ready` is 200. From
≤0.30.0, migrate to 166 first ([upgrade-to-0.31.0.md](upgrade-to-0.31.0.md)),
then to 168 with this cut.

Migration **167** creates `edgequake_tenant_access` (`NOLOGIN`, `NOBYPASSRLS`)
and `FORCE ROW LEVEL SECURITY` on content tables. Migration **168** adds
`users.failed_login_attempts` and `users.locked_until` (`IF NOT EXISTS`).
Both are additive expand-phase.

**demo.edgequake.com:** this cut installs the 0.32.0 API and Web UI. Decision
mode stays a preview (uncalibrated gates). Password auth remains the demo
default; SSO stays opt-in from v0.30.0.

## Highlights

| Area | What changed |
|------|----------------|
| Schema | **167** tenant RLS role and policies; **168** identity lockout columns |
| Query | Scoped graph-read timeouts; incremental RAG streaming |
| Convert | Vision/PDF convert timeouts retry until `max_retries` |
| Acc | Same 2026-08-15 medical-mid attestation as 0.31.0; query deadlines, graph-read scope, and streaming not re-scored |

## Sequence

```text
# Already on 0.31.0 (schema 166):
EDGEQUAKE_VERSION=0.32.0 docker compose pull
# migrate Job / edgequake migrate (applies 167–168), then API
EDGEQUAKE_VERSION=0.32.0 docker compose up -d
```

## Verify

```bash
curl -sf localhost:8080/health | jq '{version, schema}'
# expect version "0.32.0", schema.latest_version 168, pending_count 0
```

## Residuals (not release blockers)

- SPEC-001 Acc was not re-run for this cut. Query deadlines, graph-read
  scope, and streaming changed after the 2026-08-15 pack.
- Decision extraction remains a preview (uncalibrated gates). See
  [upgrade-to-0.31.0.md](upgrade-to-0.31.0.md).
