---
title: "Upgrade to EdgeQuake v0.28.3"
description: "Per-release upgrade notes for EdgeQuake v0.28.3: what changed and what to run."
---

# Upgrade to EdgeQuake v0.28.3

> **From:** v0.28.2 · **To:** v0.28.3 · **CD:** GHCR (`edgequake`,
> `edgequake-frontend`, `edgequake-postgres`)

Patch cut: SPEC-152 EQ-MCP-1.0 AgentView + OAuth AS, plus GCP pull hygiene.
**Schema stays at 160** — no new migration. Pull new API/frontend images.

## Highlights

| Area | What changed |
|------|----------------|
| Schema | Unchanged (**160**) |
| MCP | EQ-MCP-1.0 AgentView + AS surface (SPEC-152) |
| Deploy | Image prune / cache reclaim before compose pull |

## Sequence

```text
docker compose pull   # or set EDGEQUAKE_VERSION=0.28.3
docker compose up -d
```

## Verify

```bash
curl -sf localhost:8080/health | jq '{version, schema}'
# expect version "0.28.3", schema.latest_version 160
```
