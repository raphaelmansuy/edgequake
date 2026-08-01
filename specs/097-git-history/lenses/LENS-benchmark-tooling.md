# Lens — Benchmark tooling

## `archive_run` contract (post SPEC-097)

| File | Written locally | Tracked in git |
|------|-----------------|----------------|
| `scorecard.json`, `SUMMARY.md`, `BUSINESS_REPORT.md`, `EXEC_SUMMARY.txt`, `meta.json`, `eq_workspace.json`, `progress.json`, `LIVE.md` | Yes | Yes (thin) |
| `predictions_*.json`, `eval_*.json`, `eval_*.raw.json`, `logs/progress.jsonl` | Yes (forensics) | **No** |
| `LOCAL_ONLY.md` | Yes | Yes (points at ignored fat) |

## Operator notes

- Ladder `PROGRESS.md` still aggregates from `history/*/scorecard.json`.  
- Forensics scripts that need predictions must use a local archive path after a bench run.  
- Never `git add -f` fat JSON.
