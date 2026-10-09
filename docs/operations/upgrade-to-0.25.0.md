---
title: "Upgrade to EdgeQuake v0.25.0"
description: "Per-release upgrade notes for EdgeQuake v0.25.0: what changed and what to run."
---

# Upgrade to EdgeQuake v0.25.0

> **From:** v0.24.4 · **To:** v0.25.0 · **CD:** GHCR (`edgequake`, `edgequake-frontend`, `edgequake-postgres`)

Minor train: Langfuse observability (SPEC-124), structure-aware markdown pack (SPEC-125),
provider KV / prompt cache (SPEC-126), PDF layout overlay (SPEC-128), LLM omit/Responses
(SPEC-131 / #379), and partner reliability (#378–#381 / SPEC-129–133).
**New migration: 148.** LD-15 still applies — the API never auto-migrates at boot.

**crates.io deps:** pin `edgequake-llm` **0.10.8** and `edgequake-pdf2md` **0.9.11** (no path patches).

Prior: [upgrade-to-0.24.4.md](upgrade-to-0.24.4.md) (SPEC-118…123 + mig 145–147).

**SPEC-001 Acc:** this cut **attests** existing [`publish/latest`](../../specs/001-benchmark/e2e/artifacts/publish/latest/)
(`valid: true`, medical-mid, `2026-08-15T11:02:18Z`) — no fresh n=200 Acc run.

## Highlights

| Area | What changed |
|------|----------------|
| Mig **148** | SPEC-128 `document_pages` + `page_layout_regions` (PDF user-space layout; RLS) |
| SPEC-124 | Langfuse OTLP/HTTP + Settings deep-link + local `make langfuse-up` |
| SPEC-125 | Structure-aware markdown pack (no orphan heading chunks) |
| SPEC-126 | Provider KV / prompt cache (`EDGEQUAKE_PROMPT_CACHE`, default on) |
| SPEC-128 | PDF layout overlay UI + figure prune; needs pdf2md **0.9.11** |
| SPEC-131 / #379 | `EDGEQUAKE_LLM_OMIT_*` + `EDGEQUAKE_LLM_API_FORMAT=responses` (llm **0.10.8**) |
| #381 / SPEC-129 | CHECK-safe document status SSOT (`re_embedding` → `processing`) |
| #380 / SPEC-130 | Sink→fleet-mirror relationship UUIDs |
| #378 / SPEC-132 | Multi-PDF admit honesty (PDF routes only; non-blocking wake) |
| SPEC-133 | Fleet-mirror target-`->` parse (diagram/handwriting PDFs) |

## Sequence

```text
1. Backup (recommended — schema train 148)
2. Deploy v0.25.0 images (or binary) but do not start API replicas yet if schema is behind
3. Run migrate against the target DB (LD-15):

   edgequake migrate dry-run
   edgequake migrate

   # No --confirm-drop required for 148 (additive tables + RLS)

4. Start API + frontend pinned to 0.25.0
5. Verify health version + OpenAPI info.version + PDF overlay smoke + (optional) Langfuse Settings card
```

Compose / quickstart pin:

```bash
EDGEQUAKE_VERSION=0.25.0 docker compose -f docker-compose.quickstart.yml up -d
```

## Operator notes

### Langfuse (SPEC-124)

Set `LANGFUSE_PUBLIC_KEY`, `LANGFUSE_SECRET_KEY`, and `LANGFUSE_BASE_URL` (see [OBSERVABILITY.md](../OBSERVABILITY.md)).
Local stack: `make langfuse-up` then restart the backend. Secrets stay env-only (Settings shows status + deep-link).

### LLM transport (SPEC-131 / #379)

For Bedrock Mantle Gemma/Grok that reject sampling params:

```bash
EDGEQUAKE_LLM_OMIT_TEMPERATURE=true
# optional:
EDGEQUAKE_LLM_OMIT_REASONING_EFFORT=true
# GPT-5.6 Mantle Responses:
EDGEQUAKE_LLM_API_FORMAT=responses
```

### Multi-PDF upload (SPEC-132 / #378)

Admit PDFs via `POST /api/v1/documents/pdf` or `POST /api/v1/pdf/batch` only — not the generic multipart documents route.

### Fleet mirror (SPEC-130 / SPEC-133)

Relationship UUID pass-through + index-guided parse when entity names contain `->` (source or target). No operator flag.

## Verify

```bash
curl -s http://localhost:8080/health | jq -r '.version'   # expect 0.25.0
curl -s http://localhost:8080/api-docs/openapi.json | jq -r '.info.version'  # 0.25.0
edgequake migrate dry-run   # 148 applied / no pending additive
```

## Out of scope in this cut

- Fresh Acc n=200 re-run (attested existing pack)
- DOCX/Excel ingest (SPEC-121 product lock)
- crates.io publish of EdgeQuake **workspace** crates (GHCR-only CD; sibling `edgequake-llm` / `edgequake-pdf2md` are published separately)
