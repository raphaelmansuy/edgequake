---
title: "Upgrade to EdgeQuake v0.40.6"
description: "Upgrade from v0.40.5 to v0.40.6. Schema stays 227."
---

# Upgrade to EdgeQuake v0.40.6

> **From:** v0.40.5 (schema **227**) · **To:** v0.40.6 (schema **227**)

This patch adds no migration. `edgequake migrate` is a no-op on a database that is already at schema 227. Roll the images. Do not change the schema.

## Order of operations

```bash
# 1. Roll the API replicas to 0.40.6.
# 2. Roll the WebUI to 0.40.6.
```

You do not need a backup for a schema change. This cut does not write schema. Take a backup if your own policy requires one before an image roll.

## What changes for an operator

**A saved Vertex connection is enough.** Manuscript OCR, extraction, and embeddings use the service account stored on the fleet connection. The server does not need `GOOGLE_CLOUD_PROJECT` for that path. A deployment that has no saved Vertex secret still requires the environment variable.

**Reprocess documents that failed with `GOOGLE_CLOUD_PROJECT not set`.** The failure was permanent on v0.40.5. The new image does not resume those jobs by itself.

**demo.edgequake.com** picks this up with the image roll. Schema stays **227**. Do not rotate the secrets key.

**SPEC-001 Acc:** not re-scored on this cut. Mix retrieval is unchanged. The latest valid pack remains [`medical-mid-20261024T031725Z`](../../specs/001-benchmark/e2e/packs/history/medical-mid-20261024T031725Z/) (`valid: true`). EdgeQuake **0.8084**, LightRAG **0.7681**, delta **+0.0403**, 95% CI **[-0.0241, +0.1036]** (n_paired 72). The interval includes 0, so this is a statistical tie. EdgeQuake does not claim a win.
