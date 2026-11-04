---
title: "Upgrade to EdgeQuake v0.40.0"
description: "Upgrade from v0.39.1 to v0.40.0. Schema moves 226 → 227."
---

# Upgrade to EdgeQuake v0.40.0

> **From:** v0.39.1 (schema **226**) · **To:** v0.40.0 (schema **227**)

Run `edgequake migrate` before you start the new API replicas. One migration lands.

| Version | File | Adds |
| ------- | ---- | ---- |
| **227** | `227_spec180_bm25_doc_stats.sql` | BM25 document counts ignore empty image chunks. Existing normal workspaces have `bm25_doc_count` and `bm25_total_len` recomputed from the postings. |

The migration is `expand`. It replaces two trigger functions. It adds no column. The counter update touches only normal workspaces whose stored counts disagree with the postings. A v0.39.1 binary keeps working against the migrated schema, so the migration can run ahead of the image bump.

## Order of operations

```bash
# 1. Back up, then migrate. The advisory lock makes this safe with the old
#    replicas still serving.
edgequake migrate

# 2. Roll the API replicas to 0.40.0.
# 3. Roll the WebUI to 0.40.0.
```

`EDGEQUAKE_SCHEMA_GATE` controls what a replica does when it boots ahead of the migration. The default `fail` exits with code 78. Helm and compose set `wait`, which serves `/live` and answers `503` on `/ready` until the ledger reports the schema current.

## What changes for an operator

**Default questions.** The Query empty state asks only what the corpus can answer. A graph workspace names real entities. A picture workspace asks about pictures. An empty workspace shows no question cards.

**Normal retrieval.** A picture chunk must clear cosine `0.48`, and a text chunk must clear `0.10`, before it enters the prompt. A query that matches nothing returns an empty context and `relevance_dropped` on the response. The image grid uses the same floor.

**BM25 on a picture workspace that later receives text.** Empty image chunks no longer inflate the document count. Ranking of later text uses a document length computed from chunks that actually contain terms.

**SPEC-001 Acc:** not re-scored on this cut. The Acc pack measures a graph workspace. The latest valid pack remains [`medical-mid-20261024T031725Z`](../../specs/001-benchmark/e2e/artifacts/history/medical-mid-20261024T031725Z/) (`valid: true`). EdgeQuake **0.8084**, LightRAG **0.7681**, delta **+0.0403**, 95% CI **[-0.0241, +0.1036]** (n_paired 72). The interval includes 0, so this is a statistical tie. EdgeQuake does not claim a win.
