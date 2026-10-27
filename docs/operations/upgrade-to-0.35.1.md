---
title: "Upgrade to EdgeQuake v0.35.1"
description: "Upgrade from v0.35.0 to v0.35.1. Schema moves 206 to 207."
---

# Upgrade to EdgeQuake v0.35.1

> **From:** v0.35.0 (schema **206**) · **To:** v0.35.1 (schema **207**)

v0.35.0 already shipped the plain-graph train through migration **206**. This patch adds migration **207** and three runtime fixes. From v0.34.0, apply [upgrade-to-0.35.0.md](upgrade-to-0.35.0.md) first, then this page. `edgequake migrate` on 0.35.1 applies 207 with the rest of the safe schema. It still does not drop Apache AGE unless you pass `--drop-age`.

| Migration | What it does | Consent |
|-----------|--------------|---------|
| 207 | Document lineage lookup runs as the bypass owner so the tenant role can use the lineage indexes under forced AGE row security. | `edgequake migrate` |

`release_for_schema` maps 170–206 to v0.35.0 and 207 to v0.35.1.

## What else changed

- After migrate marks `eq_graph_cutover` ready, graph reads and writes stay on `eq_graph_nodes` and `eq_graph_edges`. `EDGEQUAKE_GRAPH_READ` is not read. Boot does not create or query AGE label tables, and it does not reconcile AGE indexes. While the ledger is not `ready`, migrate still copies an existing AGE graph into the plain tables and then marks the ledger `ready`.
- Credentialed browser clients on another origin receive their own `Origin` echoed back, with `Access-Control-Allow-Credentials: true`. The API does not send `Access-Control-Allow-Origin: *`.
- `GET /api/v1/lineage/documents/{id}` uses the migration 207 lookup instead of scanning every node in the tenant.

## Operator steps

```bash
pg_dump -Fc "$DATABASE_URL" > edgequake-$(date +%F).dump
edgequake migrate check
edgequake migrate dry-run
edgequake migrate
edgequake migrate status
```

A database already at 206 prints one pending step (207). A database at 207 prints `latest=207`. Full command reference: [upgrading.md](upgrading.md).
