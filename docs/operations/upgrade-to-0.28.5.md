---
title: "Upgrade to EdgeQuake v0.28.5"
description: "Patch upgrade notes for EdgeQuake v0.28.5: docs and release-gate fixes, schema stays at 162, and the upgrade path from older 0.28 releases."
---

# Upgrade to EdgeQuake v0.28.5

> **From:** v0.28.4 · **To:** v0.28.5 · **CD:** GHCR (`edgequake`, `edgequake-frontend`, `edgequake-postgres`)

This patch syncs the docs and runs the SPEC-154 WebUI vitest suite in the release gates. The schema stays at **162**. If you run 0.28.3 or older, first migrate to 162 by following [upgrade-to-0.28.4.md](upgrade-to-0.28.4.md).

## Highlights

| Area | What changed |
|------|--------------|
| Schema | Unchanged (**162**) if you are already on 0.28.4 |
| CI | `release_gates.sh` runs the SPEC-154 WebUI vitest suite |
| Docs | CHANGELOG, upgrade guides and binding pins are in sync |

## Upgrade path

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart TB
    A{"Already on 0.28.4 (schema 162)?"} -->|yes| B["Pull 0.28.5 images and restart"]
    A -->|"no, 0.28.3 or older"| C["Migrate to 162 via upgrade-to-0.28.4.md"]
    C --> B
    B --> D["/health shows 0.28.5 and schema 162"]
```

Notice that installs older than 0.28.4 must pass through schema 162 before this patch.

## Steps

```bash
# Already on 0.28.4 (schema 162)
EDGEQUAKE_VERSION=0.28.5 docker compose pull && docker compose up -d
```

## Verify

```bash
curl -sf localhost:8080/health | jq '{version, schema}'
# expect version "0.28.5" and schema.latest_version 162
```
