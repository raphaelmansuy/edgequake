---
title: "Upgrade to EdgeQuake v0.39.1"
description: "Upgrade from v0.39.0 to v0.39.1. Schema stays 226."
---

# Upgrade to EdgeQuake v0.39.1

> **From:** v0.39.0 (schema **226**) · **To:** v0.39.1 (schema **226**)

This patch adds no migration. `edgequake migrate` is a no-op on a database that is already at schema 226. Roll the images. Do not change the schema.

## Order of operations

```bash
# 1. Roll the API replicas to 0.39.1.
# 2. Roll the WebUI to 0.39.1.
```

You do not need a backup for a schema change. This cut does not write schema. Take a backup if your own policy requires one before an image roll.

## What changes for an operator

**Picture search (SPEC-182).** A citation now includes `doc_kind`, `mime_type`, and `source_modality`. The three fields are additive. An older client can ignore them. A picture search with no text now sends the image alone. The server does not add a language instruction to an empty query.

**Health checks.** `/health` lists provider models. It does not run a chat completion. A probe that waited for a local model load can use a shorter timeout. `/live` is unchanged.

**Scores in the WebUI.** A reciprocal-rank score is not a percent. The citations badge names the evidence band. It does not print a confidence percent.

**SPEC-001 Acc:** not re-scored on this cut. Mix retrieval is unchanged. The latest valid pack remains [`medical-mid-20261024T031725Z`](../../specs/001-benchmark/e2e/artifacts/history/medical-mid-20261024T031725Z/) (`valid: true`). EdgeQuake **0.8084**, LightRAG **0.7681**, delta **+0.0403**, 95% CI **[-0.0241, +0.1036]** (n_paired 72). The interval includes 0, so this is a statistical tie. EdgeQuake does not claim a win.
