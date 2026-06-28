# Issue #218 — Proof and Evidence

## Real tests executed

- `pnpm exec playwright test e2e/issue-218-runtime-config.spec.ts`

## Material evidence

- Raw log: `specs/013-fix-issues-05-2026/implementation/evidence/playwright-issues-216-218-232-233.log`
  - `Issue #218 runtime config ... login HTML injects runtime config script`
  - `Issue #218 runtime config ... runtime config object is valid JSON in page`
- Screenshot artifact: `specs/013-fix-issues-05-2026/implementation/screenshots/issue-218/login-runtime-config.png`
- Additional screenshot: `specs/013-fix-issues-05-2026/implementation/screenshots/intensive-mistral/218-runtime-config.png`

## UI/UX surface change

- Login/runtime pages now receive runtime config from the deployed environment at request time.
- User-visible effect: frontend connects to the correct backend URL/auth mode in container deployments (no stale build-time localhost config).

## WHY this proves the fix

- The Playwright test inspects the served `/login` page at runtime (real browser), verifies injected runtime config script/object.
- This directly validates the dynamic layout behavior required by the fix (`force-dynamic`) and catches the original static-bake regression.
