---
title: "Upgrade to EdgeQuake v0.40.5"
description: "Upgrade from v0.40.4 to v0.40.5. Schema stays 227."
---

# Upgrade to EdgeQuake v0.40.5

> **From:** v0.40.4 (schema **227**) · **To:** v0.40.5 (schema **227**)

This patch adds no migration. `edgequake migrate` is a no-op on a database that is already at schema 227. Roll the images. Do not change the schema.

## Order of operations

```bash
# 1. Roll the API replicas to 0.40.5.
# 2. Roll the WebUI to 0.40.5.
```

You do not need a backup for a schema change. This cut does not write schema. Take a backup if your own policy requires one before an image roll.

## What changes for an operator

**Organization › Providers shows the buttons.** **Add a provider** opens Deployment › Connections. Each offered provider has **Remove** and **Update models**. **Update models** opens that provider on Deployment › Providers, where **Add model** registers an id.

**A saved connection whose test passed stays offered.** `EDGEQUAKE_ALLOWED_PROVIDERS` still hides built-in cards that are not on the list. It does not hide a fleet connection after a passing test. Removing it is the **Remove** button.

**demo.edgequake.com** gets these buttons with the image roll. Schema stays **227**.

**SPEC-001 Acc:** not re-scored on this cut. Mix retrieval is unchanged. The latest valid pack remains [`medical-mid-20261024T031725Z`](../../specs/001-benchmark/e2e/packs/history/medical-mid-20261024T031725Z/) (`valid: true`). EdgeQuake **0.8084**, LightRAG **0.7681**, delta **+0.0403**, 95% CI **[-0.0241, +0.1036]** (n_paired 72). The interval includes 0, so this is a statistical tie. EdgeQuake does not claim a win.
