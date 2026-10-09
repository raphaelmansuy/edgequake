---
title: "Upgrade to EdgeQuake v0.32.2"
description: "Per-release upgrade notes for EdgeQuake v0.32.2: what changed and what to run."
---

# Upgrade to EdgeQuake v0.32.2

> **From:** v0.32.1 · **To:** v0.32.2 · **CD:** GHCR (`edgequake`,
> `edgequake-frontend`, `edgequake-postgres`, `edgequake-keycloak`)

Patch: PDF viewer paint window (blank “Page N” placeholders), SPEC-161 MCP
control surface (EQ-MCP-1.1), and workspace clippy/fmt hygiene. **No new
migrations** — schema train remains **168**.

**demo.edgequake.com:** this cut installs the 0.32.2 API and Web UI. Password
auth remains the demo default.

## Highlights

| Area | What changed |
|------|----------------|
| PDF viewer | Reserved sheet height; paint index overscan ∪ scrollport so long PDFs do not show an out-of-window placeholder in the pane |
| MCP | SPEC-161 async control: ingest/upload/delete/task_get, download, graph image; default profile `control` |
| Lint | Workspace `clippy -D warnings` / rustfmt clean |
| Schema | Still **168** (no migrate required from 0.32.1) |
| Acc | Same 2026-08-15 medical-mid attestation; MCP/PDF UI **not** re-scored |

## Sequence

```text
# Already on 0.32.1 (schema 168):
EDGEQUAKE_VERSION=0.32.2 docker compose pull
EDGEQUAKE_VERSION=0.32.2 docker compose up -d
# migrate is a no-op when already at 168
```

## Verify

```bash
curl -sf localhost:8080/health | jq '{version, schema}'
# expect version "0.32.2", schema.latest_version 168, pending_count 0
```

## Residuals (not release blockers)

- SPEC-001 Acc was not re-run; this cut is UI + MCP + lint.
- MCP write tools stay off unless `EDGEQUAKE_MCP_PROFILE` advertises writes.
- Demo PDF viewer: confirm page 1 paints a canvas, not a “Page 4” label.
