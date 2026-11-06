---
title: "Upgrade to EdgeQuake v0.40.4"
description: "Upgrade from v0.40.3 to v0.40.4. Schema stays 227."
---

# Upgrade to EdgeQuake v0.40.4

> **From:** v0.40.3 (schema **227**) · **To:** v0.40.4 (schema **227**)

This patch adds no migration. `edgequake migrate` is a no-op on a database that is already at schema 227. Roll the images. Do not change the schema.

## Order of operations

```bash
# 1. Roll the API replicas to 0.40.4.
# 2. Roll the WebUI to 0.40.4.
```

You do not need a backup for a schema change. This cut does not write schema. Take a backup if your own policy requires one before an image roll.

## What changes for an operator

**A saved provider can be offered from Deployment › Providers.** A fleet connection stays out of the offer until its test passes. The card now has **Test connection**. A passing test marks the provider Offered, unlocks its models, and lists it under Organization › Providers when that organization uses every deployment provider. A failed test stays Not offered and shows the error on the card.

**demo.edgequake.com** gets this card with the image roll. No operator step beyond the roll. Schema stays **227**.

**SPEC-001 Acc:** not re-scored on this cut. Mix retrieval is unchanged. The latest valid pack remains [`medical-mid-20261024T031725Z`](../../specs/001-benchmark/e2e/packs/history/medical-mid-20261024T031725Z/) (`valid: true`). EdgeQuake **0.8084**, LightRAG **0.7681**, delta **+0.0403**, 95% CI **[-0.0241, +0.1036]** (n_paired 72). The interval includes 0, so this is a statistical tie. EdgeQuake does not claim a win.
