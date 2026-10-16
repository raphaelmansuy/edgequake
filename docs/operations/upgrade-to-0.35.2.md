---
title: "Upgrade to EdgeQuake v0.35.2"
description: "Upgrade from v0.35.1 to v0.35.2. Schema stays 207."
---

# Upgrade to EdgeQuake v0.35.2

> **From:** v0.35.1 (schema **207**) · **To:** v0.35.2 (schema **207**)

No new migration. `edgequake migrate` on a database already at 207 is a no-op. From v0.35.0, apply [upgrade-to-0.35.1.md](upgrade-to-0.35.1.md) first. Apache AGE is still dropped only when you pass `--drop-age`.

`release_for_schema` still maps 207 to v0.35.1, the release that first shipped that schema. v0.35.2 serves the same schema.

## What changed

- Once `eq_graph_cutover.status` is `ready`, `EDGEQUAKE_GRAPH_READ=age` does not recreate AGE label tables and does not route graph IO back to Apache AGE.
- Document lineage GET reads the stored lineage row.
- A knowledge injection keeps its workspace tenant, and the workspace list returns that row.
- The query page keeps the question visible until the saved answer arrives.

## Operator steps

```bash
pg_dump -Fc "$DATABASE_URL" > edgequake-$(date +%F).dump
edgequake migrate status
```

A database at 207 prints `latest=207`. Full command reference: [upgrading.md](upgrading.md).
