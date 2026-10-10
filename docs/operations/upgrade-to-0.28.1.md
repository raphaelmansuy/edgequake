---
title: "Upgrade to EdgeQuake v0.28.1"
description: "Patch upgrade notes for EdgeQuake v0.28.1: no schema change, CI and compose hygiene, and how to verify the version."
---

# Upgrade to EdgeQuake v0.28.1

> **From:** v0.28.0 · **To:** v0.28.1 · **CD:** GHCR (`edgequake`, `edgequake-frontend`, `edgequake-postgres`)

This patch cleans up CI, docs and compose settings after v0.28.0. It adds no migration, so the schema stays at **160**. If you already migrated to 0.28.0, you do not need `edgequake migrate` for this bump.

## Highlights

| Area | What changed |
|------|--------------|
| Schema | Unchanged (**160**) |
| CI | Postgres-gated search unit tests; rustdoc private-link fix; invariant FAILED grep |
| Website | Starlight `title` frontmatter so the GitHub Pages Astro build passes |
| Quickstart | No fixed `container_name`; the API accepts the opt-in `EDGEQUAKE_ALLOW_MOCK_PROVIDER` |

## Upgrade path

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart LR
    A{"Already on 0.28.0 with schema 160?"} -->|yes| B["Pull 0.28.1 images"]
    A -->|no| C["Follow upgrade-to-0.28.0.md first"]
    C --> B
    B --> D["docker compose up -d"]
    D --> E["/health shows 0.28.1 and schema 160"]
```

The schema is unchanged, so restarting the stack is enough once you are on 0.28.0.

## Steps

```bash
# Already on 0.28.0 with schema 160
docker compose pull        # or set EDGEQUAKE_VERSION=0.28.1
docker compose up -d

# Fresh install, or coming from before 0.28.0:
# follow upgrade-to-0.28.0.md first (migrate to 160), then pull 0.28.1 images.
```

## Verify

```bash
curl -sf localhost:8080/health | jq '{version, schema}'
# expect version "0.28.1" and schema.latest_version 160
```

## Notes

- The default LLM path is still Ollama or OpenAI. Set `EDGEQUAKE_LLM_PROVIDER=mock EDGEQUAKE_ALLOW_MOCK_PROVIDER=1` only for smoke tests and CI.
- Issues [#400](https://github.com/raphaelmansuy/edgequake/issues/400), [#404](https://github.com/raphaelmansuy/edgequake/issues/404) and [#405](https://github.com/raphaelmansuy/edgequake/issues/405) were closed against the v0.28.0 pin. This patch does not reopen them.
