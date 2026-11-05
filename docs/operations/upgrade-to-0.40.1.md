---
title: "Upgrade to EdgeQuake v0.40.1"
description: "Upgrade from v0.40.0 to v0.40.1. Schema stays 227."
---

# Upgrade to EdgeQuake v0.40.1

> **From:** v0.40.0 (schema **227**) · **To:** v0.40.1 (schema **227**)

This patch adds no migration. `edgequake migrate` is a no-op on a database that is already at schema 227. Roll the images. Do not change the schema.

## Order of operations

```bash
# 1. Roll the API replicas to 0.40.1.
# 2. Roll the WebUI to 0.40.1.
```

You do not need a backup for a schema change. This cut does not write schema. Take a backup if your own policy requires one before an image roll.

## What changes for an operator

**Provider settings honesty (SPEC-184).** Deployment, organization, and workspace provider cards report the real auth requirement (`Key set` / `Key not set` / `Google Cloud identity`). Vertex no longer claims a key it cannot use. Connection shapes expose `auth`, `requires_key`, `accepts_saved_key`, and `stores_secret` so the add-provider form hides a key field the shape cannot store.

**Vertex identity.** Chat and image embedding share one verified Google Cloud identity. A machine where ADC and `gcloud auth print-access-token` are different principals no longer 403s every chat query while embeddings succeed. `/health` probes whether the identity may use the project, not only whether a token exists.

**Documents upload (SPEC-185).** The text-only notice sits beside the drop zone, collapses to one line, and is dismissible per workspace. The drop zone chooses its control layout from measured height so parser and vision selects stay visible when the preview panel is open.

**Query errors.** A provider 403 / auth / rate-limit failure keeps an actionable sentence instead of collapsing to "Something went wrong".

**SPEC-001 Acc:** not re-scored on this cut. Mix retrieval is unchanged. The latest valid pack remains [`medical-mid-20261024T031725Z`](../../specs/001-benchmark/e2e/packs/history/medical-mid-20261024T031725Z/) (`valid: true`). EdgeQuake **0.8084**, LightRAG **0.7681**, delta **+0.0403**, 95% CI **[-0.0241, +0.1036]** (n_paired 72). The interval includes 0, so this is a statistical tie. EdgeQuake does not claim a win.
