---
title: "Upgrade to EdgeQuake v0.24.4"
description: "Per-release upgrade notes for EdgeQuake v0.24.4: what changed and what to run."
---

# Upgrade to EdgeQuake v0.24.4

> **From:** v0.24.3 · **To:** v0.24.4 · **CD:** GHCR (`edgequake`, `edgequake-frontend`, `edgequake-postgres`)

Patch train: partner ingest/delete reliability (#370/#374/#375/#376), SPEC-123 env/config cascade,
SPEC-122 admit honesty, SPEC-121 format matrix, plus SPEC-114 / SPEC-015V / SPEC-116/117.
**New migrations: 145–147.** LD-15 still applies — the API never auto-migrates at boot.

Prior: [upgrade-to-0.24.3.md](upgrade-to-0.24.3.md) (SPEC-112 pools).

## Highlights

| Area | What changed |
|------|----------------|
| Mig **145** | SPEC-119 AGE singular edge citation indexes (`source_chunk_id` / `source_document_id`) |
| Mig **146** | `conversations.mode = 'bypass'` (chat bypass) |
| Mig **147** | `messages.llm_provider` / `llm_model` lineage columns |
| #376 / SPEC-118 | `injection::` doc IDs under relational chunk authority |
| #375 / SPEC-119 | Delete/reprocess no longer Seq-Scan timeouts on singular citations |
| #374 / SPEC-120 | Same-workspace `legacy_vector_id` race absorbed |
| #370 / SPEC-121 | Format matrix honesty — PDF supported; **DOCX not supported** |
| SPEC-123 | Request > Workspace > Tenant > Env cascade for parser + models |

## Sequence

```text
1. Backup (recommended — schema train 145–147)
2. Deploy v0.24.4 images (or binary) but do not start API replicas yet if schema is behind
3. Run migrate against the target DB (LD-15):

   edgequake migrate dry-run
   edgequake migrate

   # No --confirm-drop required for 145–147 (additive / index / CHECK)

4. Start API + frontend pinned to 0.24.4
5. Verify health version + OpenAPI info.version + a PDF upload + delete/reprocess smoke
```

Compose / quickstart pin:

```bash
EDGEQUAKE_VERSION=0.24.4 docker compose -f docker-compose.quickstart.yml up -d
```

## Format matrix (SPEC-121 / #370)

| Format | Supported |
|--------|-----------|
| PDF, TXT, MD, JSON, images | Yes |
| DOCX, Excel / Office | **No** (product lock — not a regression) |

See [FAQ](../faq.md#what-document-formats-are-supported) and
[document upload quick reference](../api-reference/document-upload-quick-reference.md).

## Verify

```bash
curl -s http://localhost:8080/health | jq -r '.version'   # expect 0.24.4
curl -s http://localhost:8080/api-docs/openapi.json | jq -r '.info.version'  # 0.24.4
```

## Out of scope in this cut

- DOCX/Excel ingest (tracked as future study under SPEC-121)
- #361 / #365 bulk-upload wall-clock SLO (SPEC-122 is admit honesty, not throughput claim)
- crates.io publish of workspace crates (GHCR-only CD)
