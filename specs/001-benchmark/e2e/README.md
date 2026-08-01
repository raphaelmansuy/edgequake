# SPEC-001 e2e artifacts

## Layout

```text
artifacts/
  smoke/          # full Acc n=40 (publication)
  smoke-fast/     # Acc gate n=8 (capped corpus OK)
  LIVE.md         # live progress board (docs / chunks / ETA)
  publish/        # Acc SSOT (latest + peers) — always commit thin claims
  history/        # archived runs (see thin vs fat below)
```

### Thin vs fat archives (SPEC-097 / GH-351)

| Class | Examples | In git? |
|-------|----------|---------|
| **Thin SSOT** | `scorecard.json`, `SUMMARY.md`, `BUSINESS_REPORT.md`, `meta.json`, `LOCAL_ONLY.md` | Yes |
| **Fat forensics** | `predictions_*.json`, `eval_*.json`, `eval_*.raw.json`, `logs/progress.jsonl` | **No** (gitignored; local-only) |

Do not `git add -f` fat JSON. Regenerate with `make bench001-*`. Policy: [`specs/097-git-history/`](../../097-git-history/).

## Publication Acc pins (required)

| Role | Provider / model |
|------|------------------|
| Text LLM | `mistral` / `mistral-small-latest` |
| Vision | `mistral` / `mistral-small-latest` |
| Embedding | `mistral` / **`mistral-embed`** (embed API — not a chat model) |
| Judge | `mistral` / `mistral-small-latest` |
| Chunk | **1200** / overlap **100**, adaptive **off**, fusion **rrf** |
| Corpus | **FULL** medical (`BENCH001_INGEST_MAX_CHARS=0`) |

## Full Acc benchmark (n=40) — publication

```bash
# Always restarts Acc backend; force-ingests FULL corpus; fails if pins/corpus wrong
make bench001-full
# alias: make bench001

# Monitor
make bench001-watch STAGE=smoke

# Warm query-only after a successful full ingest
BENCH001_QUERY_ONLY=1 make bench001-full
```

Artifacts: `specs/001-benchmark/e2e/artifacts/smoke/`. Cost-gated medical+novel: `make bench001-core`.

## Fast Acc loop (n=8, capped 100k)

```bash
BENCH001_FORCE_INGEST=1 make bench001-smoke-fast-acc
make bench001-smoke-fast-acc
make bench001-acc-backend
```

### Monitor live progress + ETA

```bash
make bench001-watch STAGE=smoke
python3 -m bench001.cli live smoke
tail -f specs/001-benchmark/e2e/artifacts/smoke/logs/progress.jsonl
```

LIVE.md shows pipeline glyphs, phase ETA, **docs / chunk size / indexed chunks / corpus chars**.

**Analysis:** [011-publication-acc-report.md](../011-publication-acc-report.md) — first principles, EQ gaps vs LightRAG, July 2026 SOTA.

Backend is double-fork detached (`tools/bench001/scripts/start_acc_backend.py`). Official judge auto-scrubs `LLM_API_KEY=FAKE*`.

See [010-smoke-then-core-runbook.md](../010-smoke-then-core-runbook.md).
