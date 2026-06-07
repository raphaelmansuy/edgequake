# SPEC-017 edgequake-webui — E2E proof index

**Last verified:** 2026-06-04 23:11 UTC — **9/9 Playwright passed**

| # | Proof | Layer |
|---|-------|-------|
| 001 | [API barrel split](001-api-barrel-split-proof.md) | Vitest |
| 002 | [QueryMode parity](002-query-mode-parity-proof.md) | Vitest |
| 003 | [Status badge DRY](003-status-badge-dry-proof.md) | Code + E2E |
| 004 | [Playwright UI + screenshots](004-playwright-route-smoke-proof.md) | Live stack `03`–`07` |
| 005 | [Full pipeline sync+async](005-playwright-live-pipeline-proof.md) | API + UI |

**Runner:**

```bash
./specs/017-dry-and-solid-audit/013-edgequake-webui/001-audit/e2e/run_playwright_proof.sh
```

**Log:** `001-test-run.log` | **Health:** `002-health-response.json` | **Screenshots:** `screenshots/03`–`07`
