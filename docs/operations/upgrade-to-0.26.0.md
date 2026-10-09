---
title: "Upgrade to EdgeQuake v0.26.0"
description: "Per-release upgrade notes for EdgeQuake v0.26.0: what changed and what to run."
---

# Upgrade to EdgeQuake v0.26.0

> **From:** v0.25.0 · **To:** v0.26.0 · **CD:** GHCR (`edgequake`, `edgequake-frontend`, `edgequake-postgres`)
>
> **CLI honesty (SPEC-137) ships in [v0.26.1](upgrade-to-0.26.1.md).** Use the
> 0.26.1+ binary for leftover DROP OLD (`--drop-confirm` alias). Schema train
> stays **149** — 0.26.1 adds no migrations.

Minor train: PDF pack-to-budget (SPEC-135), manuscript page-as-unit convert (SPEC-134),
Langfuse dev sibling (SPEC-124), and partner reliability (#377, #383–#386, SPEC-101).
**New migration: 149.** LD-15 still applies — the API never auto-migrates at boot.

**crates.io deps:** pin `edgequake-llm` **0.10.8**, `edgequake-pdf2md` **0.9.11**, `edgeparse-core` **0.2.5**; `edgequake-sdk` **0.4.0** (no path patches).

Prior: [upgrade-to-0.25.0.md](upgrade-to-0.25.0.md) (SPEC-124…133 + mig 148).

**SPEC-001 Acc:** this cut **attests** existing [`publish/latest`](../../specs/001-benchmark/e2e/artifacts/publish/latest/)
(`valid: true`, medical-mid, `2026-08-15T11:02:18Z`) — no fresh n=200 Acc run; **PDF geometry not re-scored**.

## Highlights

| Area | What changed |
|------|----------------|
| Mig **149** | `tasks.document_id` column + index + backfill (#384) |
| SPEC-135 | PDF pack-to-budget (default ON); page span `page_start`/`page_end`; MM index-once; citation `p.N–M` |
| SPEC-134 | Manuscript page-as-unit convert; lift extract off disabled reasoning |
| SPEC-124 | `make dev-langfuse` / `dev-bg-langfuse` + `make spec124-langfuse-e2e` |
| #377 / SPEC-136 | Absorb stamp-once `legacy_vector_id` unique violations |
| #383–#386 | Saga compensation, in-flight task honesty, reprocess metadata rollback |
| SPEC-101 | Wizard persist honesty (create/reconfigure embedding overrides) |

## Sequence

```text
1. Backup (recommended — schema train 149; required before any DROP OLD)
2. Deploy v0.26.0 images (or binary) but do not start API replicas yet if schema is behind
3. Run migrate against the target DB (LD-15):

   edgequake migrate dry-run
   edgequake migrate
   # applies SAFE SCHEMA 149 if pending. No confirm required for 149.

4. Start API + frontend pinned to 0.26.0
5. Verify health version + OpenAPI info.version + PDF ingest smoke
```

### Leftover SPEC-091 DROP OLD (125 / 126 / 131)

Serving on 0.25 with pending KV/vector drops is **legal**. Those versions are
**not** part of the 149 train. If `dry-run` / preflight still lists 125, 126, or
131, you are on the mid-cutover ladder — follow
[spec091-upgrade-from-v0.22.0.md](spec091-upgrade-from-v0.22.0.md) and
[upgrade-to-0.24.2.md](upgrade-to-0.24.2.md) (engine jobs + guard GREEN), then
run the confirm step with a **v0.26.1+** binary ([upgrade-to-0.26.1.md](upgrade-to-0.26.1.md)):

```text
edgequake migrate guard
# backup first
edgequake migrate --confirm-drop
# alias (SPEC-137, 0.26.1+): edgequake migrate --drop-confirm
edgequake migrate          # deferred SPEC-105 assert 142
```

Unknown apply flags fail closed (e.g. `--confirm-drp`). Do **not** set
`EDGEQUAKE_MIGRATION_CONFIRM_DROP=1` in a shared env file.

SQL abort on uncovered rows is fail-closed safety (Wave D / W4 / IW2). Do not
skip guards. Detail: [`specs/137-issue-migration-25-to-26/09-ops-runbook.md`](../../specs/137-issue-migration-25-to-26/09-ops-runbook.md).

Compose / quickstart pin (prefer **0.26.4** for leftover 091 CLI):

```bash
EDGEQUAKE_VERSION=0.26.4 docker compose -f docker-compose.quickstart.yml up -d
```

## Operator notes

### PDF chunking (SPEC-135)

Product PDF ingest now **packs** converted markdown to the workspace tiktoken budget (default ON).

```bash
# Roll back to pre-135 Recursive inner (ops kill switch)
EDGEQUAKE_PDF_PACK=0

# Disable cross-page span packing (hard page emit only)
EDGEQUAKE_PDF_CROSS_PAGE_PACK=0
```

- Applies to **future ingestions only** — no auto-rebuild of existing workspaces.
- Historical `chunks.page_start`/`page_end` stay NULL until explicit **Rebuild KG**.
- Acc PDF geometry was **not** re-scored for this cut.

### Langfuse (SPEC-124)

Unchanged from 0.25.0 for production. Local dev:

```bash
make dev-langfuse          # isolated Langfuse v4 + EdgeQuake stack
make spec124-langfuse-e2e  # one-command Settings + sessions proof
```

### Manuscript PDF (SPEC-134)

Manuscript-class pages use page-as-unit convert. See [specs/134-manuscrit/](../../specs/134-manuscrit/).

## Verify

```bash
curl -s http://localhost:8080/health | jq -r '.version'   # expect 0.26.0
curl -s http://localhost:8080/api-docs/openapi.json | jq -r '.info.version'  # 0.26.0
```

## Out of scope

- crates.io publish of EdgeQuake **workspace** crates (GHCR-only CD)
- Auto-rebuild KG on upgrade
- Fresh Acc n=200 medical-mid run
