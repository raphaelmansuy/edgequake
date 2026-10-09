---
title: "Upgrade to EdgeQuake v0.28.2"
description: "Per-release upgrade notes for EdgeQuake v0.28.2: what changed and what to run."
---

# Upgrade to EdgeQuake v0.28.2

> **From:** v0.28.1 · **To:** v0.28.2 · **CD:** GHCR (`edgequake`,
> `edgequake-frontend`, `edgequake-postgres`)

Patch cut: SPEC-149 bounded multi-batch staging for dense document persist.
**Schema stays at 160** — no new migration. Pull new API/frontend images;
reprocess any document that failed with the 10 000-record admission error.

## Highlights

| Area | What changed |
|------|----------------|
| Schema | Unchanged (**160**) |
| Ingest | Chunk-aligned `PreparedIngestionBatch` packs under `MAX_BATCH_RECORDS` (10 000) |
| Deploy | GCP Caddy `/api/*` + Caddy recreate from 0.28.1-era config fixes remain |

## Sequence

```text
# Already on 0.28.x with schema 160:
docker compose pull   # or set EDGEQUAKE_VERSION=0.28.2
docker compose up -d

# Documents that failed with:
#   batch contains N records; maximum is 10000
# → reprocess / re-upload after the API image is 0.28.2.
```

## Verify

```bash
curl -sf localhost:8080/health | jq '{version, schema}'
# expect version "0.28.2", schema.latest_version 160
```

## Notes

- Do **not** raise `MAX_BATCH_RECORDS` as a workaround; the product fix is
  multi-batch staging (SPEC-149 contracts).
- Full `FinalizeIngestion` publication CAS remains a SPEC-149 follow-up;
  each staging `commit_batch` still advances document revision / projection
  events as in 0.28.1.
