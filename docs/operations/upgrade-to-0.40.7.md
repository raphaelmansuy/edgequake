---
title: "Upgrade to EdgeQuake v0.40.7"
description: "Upgrade from v0.40.6 to v0.40.7. Schema stays 227."
---

# Upgrade to EdgeQuake v0.40.7

> **From:** v0.40.6 (schema **227**) · **To:** v0.40.7 (schema **227**)

This patch adds no migration. `edgequake migrate` is a no-op on a database that is already at schema 227. Roll the images. Do not change the schema.

## Order of operations

```bash
# 1. Roll the API replicas to 0.40.7.
# 2. Roll the WebUI to 0.40.7.
```

You do not need a backup for a schema change. This cut does not write schema. Take a backup if your own policy requires one before an image roll.

## What changes for an operator

**Creating a workspace uses the saved Vertex credential.** An Images workspace probes `gemini-embedding-2` before the row is written. That probe now opens the fleet service account, so the server does not need `GOOGLE_CLOUD_PROJECT` when the connection is already saved. A deployment with no saved Vertex secret still requires the environment variable.

**A taken address is a conflict.** Creating or renaming a workspace onto a slug that already exists returns `workspace_slug_taken`. The create dialog says the address is taken instead of a generic retry banner.

**demo.edgequake.com** picks this up with the image roll. Schema stays **227**. Do not rotate the secrets key.

**SPEC-001 Acc:** not re-scored on this cut. Mix retrieval is unchanged. The latest valid pack remains [`medical-mid-20261024T031725Z`](../../specs/001-benchmark/e2e/packs/history/medical-mid-20261024T031725Z/) (`valid: true`). EdgeQuake **0.8084**, LightRAG **0.7681**, delta **+0.0403**, 95% CI **[-0.0241, +0.1036]** (n_paired 72). The interval includes 0, so this is a statistical tie. EdgeQuake does not claim a win.
