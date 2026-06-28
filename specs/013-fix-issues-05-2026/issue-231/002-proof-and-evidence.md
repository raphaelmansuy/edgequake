# Issue #231 — Proof and Evidence

## Real tests executed

- `cargo test -p edgequake-api --features postgres --test e2e_spec013_github_issues -- --nocapture`
- `pnpm exec playwright test e2e/issue-231-upload-workspace-header.spec.ts`

## Material evidence

- Raw log: `specs/013-fix-issues-05-2026/implementation/evidence/rust-e2e-spec013-github-issues.log`
  - `test spec013_issue231_document_upload_workspace_header ... ok`
  - `test spec013_issue231_models_endpoint_reports_defaults ... ok`
- Raw log: `specs/013-fix-issues-05-2026/implementation/evidence/playwright-issue-231.log`
  - `Issue #231 workspace upload header ... 1 passed`

## UI/UX surface change

- Swagger/OpenAPI now surfaces workspace-scoping headers on upload flows.
- User-visible effect: users can correctly target a workspace during upload operations from API tools and avoid mis-scoped ingestion.

## WHY this proves the fix

- Rust test posts a real document upload with `X-Tenant-ID` + `X-Workspace-ID` and asserts successful creation.
- Playwright test validates the same header-scoped upload behavior over HTTP from browser test runner.
- Combined evidence proves header-based workspace scoping works end-to-end in PostgreSQL mode.
