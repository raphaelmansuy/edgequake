---
title: "Upgrade to EdgeQuake v0.32.1"
description: "Per-release upgrade notes for EdgeQuake v0.32.1: what changed and what to run."
---

# Upgrade to EdgeQuake v0.32.1

> **From:** v0.32.0 · **To:** v0.32.1 · **CD:** GHCR (`edgequake`,
> `edgequake-frontend`, `edgequake-postgres`, `edgequake-keycloak`)

Patch: typed ANN registry keying and graph seed admit so Ask/named-entity
queries do not miss workspace embeddings or lose to popular hubs. **No new
migrations** — schema train remains **168**.

**demo.edgequake.com:** this cut installs the 0.32.1 API and Web UI. Password
auth remains the demo default. After deploy, audit `embedding_models` for
workspace `…0003` via
[embedding-registry-backfill](embedding-registry-backfill.md) — do not silently
rename stamps to another model.

## Highlights

| Area | What changed |
|------|----------------|
| Typed ANN | Preferred workspace/lineage model only; no preferred→env fallthrough |
| Graph admit | Exact labels/seeds before popular hubs (local + global Mix arms) |
| Ingest/projection | Lineage `model_id` on manifests; projection upserts per model |
| Chat / WebUI | Companion `seed_entity_ids` through chat → engine |
| Schema | Still **168** (no migrate required from 0.32.0) |
| Acc | Same 2026-08-15 medical-mid attestation; ANN/admit **not** re-scored |

## Sequence

```text
# Already on 0.32.0 (schema 168):
EDGEQUAKE_VERSION=0.32.1 docker compose pull
EDGEQUAKE_VERSION=0.32.1 docker compose up -d
# migrate is a no-op when already at 168
```

## Verify

```bash
curl -sf localhost:8080/health | jq '{version, schema}'
# expect version "0.32.1", schema.latest_version 168, pending_count 0
```

## Residuals (not release blockers)

- SPEC-001 Acc was not re-run; ANN keying and graph admit changed after the
  2026-08-15 pack.
- Migration backfill/verify jobs still key off process env (ops residual).
- Demo registry may show `text-embedding-3-small`@1024 rows — rebuild/backfill
  only after confirming the producer embedder.
