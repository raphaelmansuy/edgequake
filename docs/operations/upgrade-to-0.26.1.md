---
title: "Upgrade to EdgeQuake v0.26.1"
description: "Per-release upgrade notes for EdgeQuake v0.26.1: what changed and what to run."
---

# Upgrade to EdgeQuake v0.26.1

> **From:** v0.26.0 · **To:** v0.26.1 · **CD:** GHCR (`edgequake`, `edgequake-frontend`, `edgequake-postgres`)

Ops patch: SPEC-137 migrate honesty (`--drop-confirm` alias, unknown apply flags
fail-closed, classified DROP abort hints). **No new migrations** — schema train
remains **149** from [upgrade-to-0.26.0.md](upgrade-to-0.26.0.md).

## Highlights

| Area | What changed |
|------|----------------|
| Consent | `--confirm-drop` canonical; `--drop-confirm` accepted as the same consent |
| Unknown flags | `edgequake migrate --*` that is not a known apply flag exits non-zero |
| Abort hints | Wave D / W4 / IW2 / 142 / checksum / lock classified (not always `pg_locks`) |
| Preflight | Migrations 144–149 tagged SAFE SCHEMA |

This cut does **not** weaken DROP OLD SQL (125 / 126 / 131 / 142). Uncovered
KV/vector rows still abort. Operators still need `migrate guard` GREEN before
`--confirm-drop`.

## Sequence

```text
1. Backup (optional for this patch — no schema train)
2. Deploy v0.26.1 API (and frontend if you pin it)
3. If leftover 125 / 126 / 131 remain, follow the leftover SPEC-091 ladder in
   upgrade-to-0.26.0.md using this 0.26.1+ binary (not the 0.26.0 image)
4. Verify health version + migrate --help lists --drop-confirm
```

Compose / quickstart pin:

```bash
EDGEQUAKE_VERSION=0.26.1 docker compose -f docker-compose.quickstart.yml up -d
```

## Verify

```bash
curl -s http://localhost:8080/health | jq -r '.version'   # expect 0.26.1
curl -s http://localhost:8080/api-docs/openapi.json | jq -r '.info.version'  # 0.26.1
docker run --rm ghcr.io/raphaelmansuy/edgequake:0.26.1 migrate --help
# lists --confirm-drop and --drop-confirm
```

## Out of scope in this cut

- Weakening fail-closed DROP OLD guards (Track B residue still needs engine jobs)
- Fresh Acc n=200 medical-mid run (attested existing `publish/latest`)
- crates.io publish of EdgeQuake workspace crates (GHCR-only CD)
- Auto `--confirm-drop`

Detail: [`specs/137-issue-migration-25-to-26/09-ops-runbook.md`](../../specs/137-issue-migration-25-to-26/09-ops-runbook.md).
