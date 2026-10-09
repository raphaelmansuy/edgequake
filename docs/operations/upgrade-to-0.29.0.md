---
title: "Upgrade to EdgeQuake v0.29.0"
description: "Per-release upgrade notes for EdgeQuake v0.29.0: what changed and what to run."
---

# Upgrade to EdgeQuake v0.29.0

> **From:** v0.28.5 · **To:** v0.29.0 · **CD:** GHCR (`edgequake`,
> `edgequake-frontend`, `edgequake-postgres`)

Minor cut: SPEC-157 side-by-side query companion, SPEC-155 documents workspace
+ query composer, SPEC-156 ingestion, `edgeparse-ocr` (Tesseract in the API
image). Schema train moves **162 → 163**. Run migrate before `/ready` is 200.
From ≤0.28.3, migrate to 162 first (see
[upgrade-to-0.28.4.md](upgrade-to-0.28.4.md)), then to 163 with this cut.

## Highlights

| Area | What changed |
|------|----------------|
| Schema | **163** — `messages.feedback_rating` / `feedback_reason` / `finish_reason` (manifest registers 161–163) |
| Query | Companion pane (PDF citation + answer graph); kill switch `NEXT_PUBLIC_QUERY_COMPANION=0` |
| Documents | Docking workspace, intake strip, layout modes (SPEC-155) |
| Ingestion | Cloud fan-out honesty; `run_progress` ledger; gleaning / embed join (SPEC-156) |
| PDF | `edgeparse-ocr` backend; Tesseract + tessdata harvested into distroless API image |
| Deps | `edgequake-llm` **0.10.9**, `edgeparse-core` **0.3.2** |

## Sequence

```text
# Already on 0.28.4 / 0.28.5 (schema 162):
EDGEQUAKE_VERSION=0.29.0 docker compose pull
# migrate Job / edgequake migrate (applies 163), then API
EDGEQUAKE_VERSION=0.29.0 docker compose up -d

# From ≤0.28.3: follow upgrade-to-0.28.4.md (migrate to 162), then pull 0.29.0 + migrate
```

## Verify

```bash
curl -sf localhost:8080/health | jq '{version, schema}'
# expect version "0.29.0", schema.latest_version 163, pending_count 0
```
