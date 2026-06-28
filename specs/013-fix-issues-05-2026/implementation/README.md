# SPEC-013 Implementation Proof

Playwright/API tests proving fixes for issues #216–#233.

| Test file | Issue | Proof |
|-----------|-------|-------|
| `issue-218-runtime-config.spec.ts` | #218 | HTML contains runtime `apiUrl` when env set |
| `issue-232-api-keys-list.spec.ts` | #232 | GET returns created key |
| `issue-231-upload-workspace-header.spec.ts` | #231 | OpenAPI + upload with `X-Workspace-ID` |
| `issue-233-workspace-create-defaults.spec.ts` | #233 | Collapsed model section + create without models |
| `issue-216-entity-types-edit.spec.ts` | #216 | PUT entity_types + UI edit |
| `entity-types-strict-limit.spec.ts` | entity strict mode | API + dashboard UI + deeplink `/w/[slug]/workspace` save + screenshots |
| `issue-workspace-server-default-reset.spec.ts` | server-default reset | PUT empty LLM fields clears mock override |

Screenshots: [screenshots/](screenshots/)

### Quick (in-process PostgreSQL + mock LLM)

```bash
make db-wait          # writes DATABASE_URL to /tmp/edgequake-db-url
make spec013-e2e-rust # cargo test --features postgres
```

Manual:

```bash
export DATABASE_URL=postgres://edgequake:edgequake_secret@localhost:5432/edgequake
cd edgequake && cargo test -p edgequake-api --features postgres --test e2e_spec013_github_issues
```

### Intensive (Mistral provider on port 8081)

```bash
export MISTRAL_API_KEY=...
make spec013-mistral-backend-bg   # backend :8081, auth off
make frontend-bg                  # UI :3000, EDGEQUAKE_API_URL → :8081
make spec013-e2e-mistral          # Rust + Playwright intensive suite
```

Optional live document ingest (real Mistral API + PostgreSQL):

```bash
export DATABASE_URL=postgres://edgequake:edgequake_secret@localhost:5432/edgequake
make spec013-e2e-mistral-live
```

Screenshots land in `screenshots/intensive-mistral/` and `playwright-report/`.

### Raw evidence logs

Real command output used by issue proof docs is stored in:

- `implementation/evidence/rust-e2e-spec013-github-issues.log`
- `implementation/evidence/rust-pipeline-entity-type.log`
- `implementation/evidence/rust-mistral-workspace-config.log`
- `implementation/evidence/rust-mistral-health.log`
- `implementation/evidence/playwright-issue-231.log`
- `implementation/evidence/playwright-issues-216-218-232-233.log`
- `implementation/evidence/backend-health-models-8090.json`
