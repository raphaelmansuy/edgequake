---
title: "Upgrade to EdgeQuake v0.28.3"
description: "Patch upgrade notes for EdgeQuake v0.28.3: MCP AgentView and OAuth authorization server, no schema change, and verification commands."
---

# Upgrade to EdgeQuake v0.28.3

> **From:** v0.28.2 · **To:** v0.28.3 · **CD:** GHCR (`edgequake`, `edgequake-frontend`, `edgequake-postgres`)

This patch adds the EQ-MCP-1.0 AgentView and an OAuth authorization server (AS) for MCP clients (SPEC-152). It also cleans up image pulls on GCP. The schema stays at **160**, so you only need to pull the new API and frontend images.

## Highlights

| Area | What changed |
|------|--------------|
| Schema | Unchanged (**160**) |
| MCP | EQ-MCP-1.0 AgentView and the AS surface (SPEC-152) |
| Deploy | Image prune and cache reclaim before `compose pull` |

## Upgrade path

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart LR
    A["Running 0.28.2 (schema 160)"] --> B["Pull 0.28.3 images"]
    B --> C["docker compose up -d"]
    C --> D["/health shows 0.28.3 and schema 160"]
```

Notice that the schema is unchanged, so pulling the new images is the whole upgrade.

## Steps

```bash
docker compose pull        # or set EDGEQUAKE_VERSION=0.28.3
docker compose up -d
```

## Verify

```bash
curl -sf localhost:8080/health | jq '{version, schema}'
# expect version "0.28.3" and schema.latest_version 160
```
