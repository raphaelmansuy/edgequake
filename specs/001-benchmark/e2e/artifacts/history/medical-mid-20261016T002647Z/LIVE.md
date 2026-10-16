# bench001 LIVE — `medical-mid`

- **updated:** `2026-10-16T00:26:46Z`
- **started:** `2026-10-16T00:11:34Z`
- **run elapsed:** `15m13s`
- **phase:** `score_parallel` (done)
- **progress:** `50/50`
- **ETA (phase):** `0s`
- **detail:** elapsed=10m00s

## Corpus / chunking

- **docs:** `1`  (done `—`)
- **chunk size / overlap:** `1200` / `100`
- **indexed chunks:** `—`
- **corpus chars:** `1052159`  capped=`False`
- **questions:** `200`

## Pipeline

✓ prepare → ○ ingest_eq → ○ query_parallel → ✓ score_parallel → ○ report  |  ✓ query_eq

## Recent ticks

| at (UTC) | phase | status | detail | eta | run |
|----------|-------|--------|--------|-----|-----|
| 2026-10-16T00:11:34Z | prepare | done | n_questions=200 docs=1 chars=1052159 chunk=1200/100 | — | 0s |
| 2026-10-16T00:16:46Z | query_eq | running | EQ query 200/200 id=Medical-65be7ca5 | 0s | 5m12s |
| 2026-10-16T00:16:46Z | query_eq | done | n=200 | — | 5m12s |
| 2026-10-16T00:26:46Z | score_parallel | running | EQ=done LR=done eval∥=1 | — | 15m13s |
| 2026-10-16T00:26:46Z | score_parallel | done | elapsed=10m00s | — | 15m13s |

## Monitor

```bash
make bench001-watch STAGE=medical-mid
# or:  watch -n 2 cat /Users/raphaelmansuy/Github/03-working/edgequake/specs/001-benchmark/e2e/artifacts/medical-mid/LIVE.md
# or:  tail -f /Users/raphaelmansuy/Github/03-working/edgequake/specs/001-benchmark/e2e/artifacts/medical-mid/logs/progress.jsonl
```
