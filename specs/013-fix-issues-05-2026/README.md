# SPEC-013: GitHub Issues Fix (May 2026)

Cross-reference index for issues [#216](https://github.com/raphaelmansuy/edgequake/issues/216)–[#233](https://github.com/raphaelmansuy/edgequake/issues/233).

| Issue | Folder | Root cause (verified) | Proof & evidence |
|-------|--------|----------------------|------------------|
| [#218](https://github.com/raphaelmansuy/edgequake/issues/218) | [issue-218/](issue-218/) | `layout.tsx` static SSR bakes build-time defaults | [003-proof-and-evidence.md](issue-218/003-proof-and-evidence.md) |
| [#232](https://github.com/raphaelmansuy/edgequake/issues/232) | [issue-232/](issue-232/) | `list_api_keys` stub returns `keys: []` (TODO) | [003-proof-and-evidence.md](issue-232/003-proof-and-evidence.md) |
| [#231](https://github.com/raphaelmansuy/edgequake/issues/231) | [issue-231/](issue-231/) | OpenAPI missing headers; batch upload ignores `TenantContext` | [002-proof-and-evidence.md](issue-231/002-proof-and-evidence.md) |
| [#233](https://github.com/raphaelmansuy/edgequake/issues/233) | [issue-233/](issue-233/) | Workspace create UI always shows full model matrix | [002-proof-and-evidence.md](issue-233/002-proof-and-evidence.md) |
| [#217](https://github.com/raphaelmansuy/edgequake/issues/217) | [issue-217/](issue-217/) | Prompt allows free-form types; no post-parse enforcement | [002-proof-and-evidence.md](issue-217/002-proof-and-evidence.md), [003-historical-cleanup-runbook.md](issue-217/003-historical-cleanup-runbook.md) |
| [#216](https://github.com/raphaelmansuy/edgequake/issues/216) | [issue-216/](issue-216/) | `UpdateWorkspaceRequest` lacks `entity_types`; UI read-only | [002-proof-and-evidence.md](issue-216/002-proof-and-evidence.md) |
| Workspace server-default reset | [issue-workspace-server-default/](issue-workspace-server-default/) | Save with “Server default” omitted LLM fields; backend could not clear mock | [002-proof-and-evidence.md](issue-workspace-server-default/002-proof-and-evidence.md) |
| Entity strict / permissive mode | [entity_extraction/](entity_extraction/) | Optional free-form types when `entity_types_strict` is false | [003-test-plan.md](entity_extraction/003-test-plan.md) |

Implementation proof: [implementation/](implementation/)

**Scope note:** The banner “#216–#233” is the mission umbrella; only rows above have spec folders and release proofs. See [009-brutal-assessment.md](009-brutal-assessment.md) before tagging.

## Proof commands

```bash
make spec013-proof-preflight   # Fail fast (keys, DB, dev backend not competing)
make spec013-proof-pr          # PR gate: mock API + vector stats (no Mistral)
make spec013-proof             # Rust: API + Mistral PDF + vector stats (--test-threads=1)
make spec013-proof-ui          # Playwright #216–#233 (stack must be up)
make spec013-proof-full        # stop → Rust proof → start stack → Playwright
make spec013-proof-ci          # 3× repeat + SLO env defaults
make spec013-entity-type-audit TENANT_ID=... WORKSPACE_ID=...
make spec013-entity-type-audit-all   # Scan all workspaces (#217 legacy)
```

CI: PR → `spec013-proof-pr.yml`; nightly → `spec013-proof.yml` (needs `MISTRAL_API_KEY`). See [010-github-secrets-and-ci.md](010-github-secrets-and-ci.md).

Brutal assessment (release readiness): [009-brutal-assessment.md](009-brutal-assessment.md)
