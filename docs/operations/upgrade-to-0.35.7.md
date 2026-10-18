---
title: "Upgrade to EdgeQuake v0.35.7"
description: "Upgrade from v0.35.5 or v0.35.6 to v0.35.7. Schema stays 207. No new migration."
---

# Upgrade to EdgeQuake v0.35.7

> **From:** v0.35.5 or v0.35.6 (schema **207**) · **To:** v0.35.7 (schema **207**)

No new migration file. Schema stays **207**. `release_for_schema` still maps 207 to v0.35.1, the release that first shipped that schema. An upgrade from 0.35.5 or 0.35.6 applies no new migration.

A greenfield install does not need Apache AGE. The published Postgres images are the same as v0.35.5: pgvector 0.8.5, `pg_trgm`, `btree_gin`, and `uuid-ossp`, and no Apache AGE. A data directory that already loaded AGE still cannot open on these images. Drop AGE with `edgequake migrate --drop-age` on the v0.35.4 image before moving that volume. See [upgrade-to-0.35.5.md](upgrade-to-0.35.5.md).

A platform admin can switch organization after login. Plain-graph label search recovers typos through the trigram index, and a tenant that does not own the workspace is rejected before lineage or prefix discovery. Neither change needs Apache AGE, and neither changes retrieval scoring.

**SPEC-001 Acc:** not re-scored. This delta does not change retrieval scoring. The latest valid medical-mid n=200 pack remains the v0.35.5 archive [`medical-mid-20261016T162456Z`](../../specs/001-benchmark/e2e/artifacts/history/medical-mid-20261016T162456Z/) (`valid: true`). EdgeQuake **0.8054**, LightRAG **0.7920**, delta **+0.0135**, 95% CI **[+0.0056, +0.1506]** (n_paired 40). Those numbers are the v0.35.5 prompt. The harness marks that pack as not a publishable beat because L2 is incomplete. EdgeQuake does not claim a win.

## Operator steps

```bash
pg_dump -Fc "$DATABASE_URL" > edgequake-$(date +%F).dump
edgequake migrate
curl -sf localhost:8080/health
```

Expect `schema.latest_version` 207, `schema.pending_count` 0, `schema.migration_required` false, and `components.graph_available` true. Full command reference: [upgrading.md](upgrading.md).
