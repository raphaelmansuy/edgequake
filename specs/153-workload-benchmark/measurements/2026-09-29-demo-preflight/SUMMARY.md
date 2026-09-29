# SUMMARY — 2026-09-29-demo-preflight

## Result: PASS (preflight + L7 smoke)

| Pin | Value |
|-----|-------|
| Base | https://demo.edgequake.com |
| Tenant | `00000000-0000-0000-0000-000000000002` |
| Workspace | `00000000-0000-0000-0000-000000000003` |
| Version | 0.28.3 |
| LLM | openai / gpt-5.4-mini |
| Embed | text-embedding-3-small @ 1536 |

## Checks

- `/live` → OK
- `/ready` → ready
- `/health` → healthy, migration_required=false
- MCP `eq_search` (mix, cheap) → 5 hits, `ret_7023f21a-…`, oracle_ok

## Not claimed

- No knee / goodput numbers (runner not implemented)
- No L8 answer-token rates (MCP query/read smoke only)
- SLO thresholds remain hypotheses

See `env.json`, `preflight-health.json`, `summary.json`.
