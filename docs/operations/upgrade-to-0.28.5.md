---
title: "Upgrade to EdgeQuake v0.28.5"
description: "Per-release upgrade notes for EdgeQuake v0.28.5: what changed and what to run."
---

# Upgrade to EdgeQuake v0.28.5

> **From:** v0.28.4 · **To:** v0.28.5 · **CD:** GHCR (`edgequake`,
> `edgequake-frontend`, `edgequake-postgres`)

Patch cut: docs/ops honesty + release_gates SPEC-154 vitest. **Schema stays
at 162**. From ≤0.28.3, migrate to 162 first (see
[upgrade-to-0.28.4.md](upgrade-to-0.28.4.md)).

## Highlights

| Area | What changed |
|------|----------------|
| Schema | Unchanged (**162**) if already on 0.28.4 |
| CI | `release_gates.sh` runs SPEC-154 WebUI vitest |
| Docs | CHANGELOG / upgrade guides / binding pins synced |

## Sequence

```text
# Already on 0.28.4 (schema 162):
EDGEQUAKE_VERSION=0.28.5 docker compose pull && docker compose up -d

# From ≤0.28.3: follow upgrade-to-0.28.4.md (migrate to 162), then pull 0.28.5
```

## Verify

```bash
curl -sf localhost:8080/health | jq '{version, schema}'
# expect version "0.28.5", schema.latest_version 162
```
