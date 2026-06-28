# SPEC-013 — GitHub secrets and CI gates

## Repository secrets

| Secret | Required for | Notes |
|--------|----------------|-------|
| `MISTRAL_API_KEY` | Nightly `spec013-proof.yml` | Live PDF ingest/query; costs API quota |
| _(none)_ | PR `spec013-proof-pr.yml` | Mock LLM + Postgres only |

**Setup:** GitHub → Settings → Secrets and variables → Actions → New repository secret → `MISTRAL_API_KEY`.

If the secret is missing, the nightly workflow runs a single **warning job** instead of failing silently.

## Workflows

| Workflow | Trigger | What it proves |
|----------|---------|----------------|
| [spec013-proof-pr.yml](../../.github/workflows/spec013-proof-pr.yml) | Pull request (scoped paths) | `e2e_spec013_github_issues` + `postgres_workspace_vector_stats` |
| [spec013-proof.yml](../../.github/workflows/spec013-proof.yml) | `workflow_dispatch`, cron 04:00 UTC | `make spec013-proof-ci` (3× Mistral + issues + stats) |

## Local equivalents

```bash
make spec013-proof-pr          # Same as PR CI (no Mistral)
make spec013-proof             # Full Rust proof (needs MISTRAL_API_KEY)
make spec013-proof-full        # stop → Rust → backend+frontend → Playwright
make spec013-entity-type-audit-all   # #217 legacy graph scan (API up)
```
