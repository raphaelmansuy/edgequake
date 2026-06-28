# Issue #216 — Proof and Evidence

## Real tests executed

- `cargo test -p edgequake-api --features postgres --test e2e_spec013_github_issues -- --nocapture`
- `pnpm exec playwright test e2e/issue-216-entity-types-edit.spec.ts`

## Material evidence

- Raw log: `specs/013-fix-issues-05-2026/implementation/evidence/rust-e2e-spec013-github-issues.log`
  - `test spec013_issue216_update_workspace_entity_types ... ok`
- Raw log: `specs/013-fix-issues-05-2026/implementation/evidence/playwright-issues-216-218-232-233.log`
  - `[chromium] ... Issue #216 entity types update ...`

## UI/UX surface change

- Workspace settings now supports editing entity types for existing workspaces instead of showing a create-time-only configuration.
- User impact: teams can correct extraction schema without creating a new workspace.

## WHY this proves the fix

- The Rust Postgres E2E test performs a real `PUT /api/v1/workspaces/{id}` with `entity_types`, then re-fetches workspace data.
- Passing result proves the full write path works in production storage mode (PostgreSQL), not only in-memory.
- Playwright run confirms user-level flow is exercised from UI/API perspective in a real browser context.
