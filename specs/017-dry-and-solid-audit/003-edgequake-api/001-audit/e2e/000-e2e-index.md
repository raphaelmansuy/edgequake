# SPEC-017 edgequake-api — E2E proof index

**Last verified:** 2026-06-04 12:22 UTC

| # | Proof | Layer |
|---|-------|-------|
| 001 | [P0 workspace pipeline factory](001-p0-workspace-pipeline-factory-proof.md) | Rust contract + integration |
| 002 | [Query/chat routing parity](002-query-chat-parity-proof.md) | Rust HTTP integration |
| 003 | [QueryError semantic mapping](003-query-error-mapping-proof.md) | Rust unit + HTTP |
| 004 | [Playwright API UI](004-playwright-api-ui-proof.md) | Live stack + screenshots 01–06 |
| 005 | [Full pipeline via API](005-full-pipeline-api-proof.md) | Sync + async + PDF |
| 006 | [Query bootstrap DRY](006-query-bootstrap-dry-proof.md) | API-DRY-003 |

**Runners:**

```bash
./specs/017-dry-and-solid-audit/003-edgequake-api/001-audit/e2e/run_api_e2e.sh
./specs/017-dry-and-solid-audit/003-edgequake-api/001-audit/e2e/run_api_e2e.sh --playwright
```

**Log:** `001-test-run.log` | **Screenshots:** `screenshots/01`–`06-*.png`
