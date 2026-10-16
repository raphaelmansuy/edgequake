---
title: "Upgrade to EdgeQuake v0.35.4"
description: "Upgrade from v0.35.3 to v0.35.4. Schema stays 207."
---

# Upgrade to EdgeQuake v0.35.4

> **From:** v0.35.3 (schema **207**) · **To:** v0.35.4 (schema **207**)

No new migration. `edgequake migrate` on a database already at 207 is a no-op for schema. From v0.35.0, apply [upgrade-to-0.35.1.md](upgrade-to-0.35.1.md) first. Apache AGE is still dropped only when you pass `--drop-age`.

`release_for_schema` still maps 207 to v0.35.1, the release that first shipped that schema. v0.35.4 serves the same schema.

**SPEC-001 Acc:** medical-mid n=200, archive [`medical-mid-20261016T111544Z`](../../specs/001-benchmark/e2e/artifacts/history/medical-mid-20261016T111544Z/) (`valid: true`). EdgeQuake **0.7877**, LightRAG **0.7990**, delta **-0.0113**, 95% CI **[-0.0361, +0.0806]** (n_paired 53). Statistical tie.

## What changed

- **#493.** After the ledger is `ready`, deleting a document removes that document's citations from `eq_graph_edges`. A relationship that other documents still cite stays. Singular `source_document_id` and `source_chunk_id` are found, and a citation the projection manifest missed is cleaned by the existing cascade.
- **#494.** `edgequake migrate` can finish the plain-graph cutover when the database role has `statement_timeout` of 10s. The copy is not left with `eq_graph_cutover.status = 'backfilling'` because of that timeout. A timeout that is set for the cutover itself still cancels the copy and rolls it back.
- Gold answers include every supporting clause from the retrieved context.

## Operator steps

```bash
pg_dump -Fc "$DATABASE_URL" > edgequake-$(date +%F).dump
edgequake migrate status
```

A database at 207 prints `latest=207`. If a previous cutover stopped at `backfilling`, run `edgequake migrate` again on this binary. Full command reference: [upgrading.md](upgrading.md).
