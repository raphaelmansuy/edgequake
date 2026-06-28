# SPEC-013 Brutal Assessment (First Principles)

**Audit date:** 2026-05-28 (updated after deeplink entity-types parity)  
**Auditor mode:** release gate + scope traceability + operational reality  
**Verdict:** **GO to tag a release** — with explicit scope boundaries; deeplink workspace UI gap **closed**.

---

## 1. First-principles release invariant

A release is shippable only when all of the following hold:

```text
(1) Correctness   — each claimed fix has a falsifiable proof
(2) Build         — compile + lint gates clean on touched crates
(3) Regression    — automated gates cover the critical path
(4) Operability   — deploy/runbook risks are named, not hidden
(5) Honesty       — marketing scope matches proven scope
```

This audit evaluates SPEC-013 against (1)–(5), not against “every GitHub issue ever filed in May 2026.”

---

## 2. Scope honesty (what this release actually is)

The folder title references issues **#216–#233**, but **proven, documented deliverables** are:

| Deliverable | GitHub / theme | Proof |
|-------------|----------------|-------|
| Runtime layout config | #218 | Playwright `issue-218-*`, `layout.tsx` `force-dynamic` |
| API keys list | #232 | API test + Playwright `issue-232-*` |
| Upload tenant/workspace headers | #231 | API test + Playwright `issue-231-*` |
| Workspace create server defaults UX | #233 | API test + Playwright `issue-233-*` |
| Edit `entity_types` on workspace | #216 | API test + Playwright `issue-216-*` |
| Post-parse entity type enforcement | #217 | Pipeline unit tests + runbook; **not** full re-ingest E2E in PR gate |
| Server-default LLM/embedding reset | SPEC-013 extension | API test + Playwright + `server_runtime_*_config` unit tests |
| Strict vs permissive entity types | SPEC-013 extension | Pipeline unit + API + Playwright (dashboard **and** `/w/[slug]/workspace`) + screenshots |
| PDF cancel API behavior | ancillary | In-process API tests; live-worker test skipped without `SPEC013_LIVE_API_URL` |
| Vector stats init | storage hardening | `postgres_workspace_vector_stats` |

**Not proven in this release train:** individual fixes for every issue number 219–230 (no spec folders, no dedicated proofs). Do **not** close those issues from this tag alone.

**Brutal takeaway:** Ship as **“SPEC-013 workspace + entity extraction + deploy config fixes”**, not as **“all issues #216–#233 resolved.”**

---

## 3. Fresh gate evidence (this audit run)

Commands executed on 2026-05-28 after `make stop` (PR preflight requires no competing backend):

| Gate | Result | Notes |
|------|--------|-------|
| `make spec013-proof-pr` | **PASS** | 10/10 `e2e_spec013_github_issues` + 1/1 vector stats |
| `cargo clippy -p edgequake-pipeline -p edgequake-core -p edgequake-api --all-targets --features postgres -- -D warnings` | **PASS** | Prior P0 blocker cleared |
| `cargo test -p edgequake-pipeline --lib entity_type` | **PASS** | 9/9 |
| `cargo test -p edgequake-core workspace_model_update --lib --test-threads=1` | **PASS** | 2/2; serial to avoid env pollution |
| `pnpm --dir edgequake_webui exec tsc --noEmit` | **PASS** | |
| `make spec013-proof-ui` | **PASS** | **10/10** Playwright (incl. deeplink entity-types strict; ports 8083 / 3001) |
| `cargo test --workspace --lib` | **PASS** | Sanity (not a formal release gate) |

**CI parity:** PR workflow runs `make spec013-proof-pr` only (mock LLM, no `MISTRAL_API_KEY`). Nightly `spec013-proof.yml` covers Mistral + 3× repeat — **not** required to merge, **recommended** before production promotion.

---

## 4. Correctness deep dive

### 4.1 Strong areas

- **#218** — Root cause (static SSR baking env) addressed at the framework boundary; E2E reads HTML for runtime script injection.
- **#232** — Stub replaced with real KV prefix scan; list-after-create is deterministic in tests.
- **#231** — OpenAPI + handler path use `TenantContext`; upload E2E sends `X-Workspace-ID`.
- **Server-default reset** — Empty-string clear contract is explicit; `server_runtime_llm_config()` aligns reset with **active** `EDGEQUAKE_LLM_PROVIDER` (fixes mismatch where `EDGEQUAKE_DEFAULT_*` said `ollama` but `/health` said `mistral`).
- **Entity strict toggle** — Default `true` preserves #217 behavior; `false` is opt-in permissive mode with unit-tested prompt + enforcement paths.
- **Deeplink workspace parity** — `/w/[slug]/workspace` uses `useWorkspaceSlugResolver` + shared `WorkspaceEntityTypesCard` (DRY with dashboard `/workspace`).

### 4.2 Residual correctness risks (honest)

| Risk | Severity | Why it still exists | Mitigation |
|------|----------|---------------------|------------|
| **#217 legacy graph nodes** | Medium (data) | Enforcement applies on **new** ingestions only | [issue-217/003-historical-cleanup-runbook.md](issue-217/003-historical-cleanup-runbook.md); `make spec013-entity-type-audit-all` |
| **No PR-gate ingest→graph entity-type E2E** | Low–Medium | Mock LLM + cost; policy covered by unit tests | Run Mistral nightly or manual ingest sample before customer demo |
| **Env var priority confusion** | Low | `default_llm_config()` vs `server_runtime_*` serve different purposes | Ops doc: “Server default” in UI = runtime provider |
| **Port / stack collisions** | Low (CI/dev) | Dynamic ports 3001, 8083; preflight fails if dev backend up | `make stop` before `spec013-proof-pr`; use `spec013-wait-stack` |
| **Live PDF cancel on completed** | Low | Skipped without `SPEC013_LIVE_API_URL` | Optional manual check on staging |

None of these are **ship-stoppers** for a patch release if release notes state them clearly.

---

## 5. Quality & engineering discipline

| Dimension | Score | Brutal notes |
|-----------|-------|----------------|
| Feature correctness (proven scope) | **9.1/10** | Core fixes match root causes; dashboard + deeplink entity UX aligned |
| Test robustness | **9.0/10** | 10/10 UI gate; #217 ingest E2E still thin; env-sensitive tests need `--test-threads=1` |
| Build hygiene | **9.4/10** | Clippy `-D warnings` clean on touched crates; not full-workspace clippy in gate |
| DRY / SOLID | **9.2/10** | `WorkspaceEntityTypesCard`, `workspace_model_update`, `entity_type_policy`, `spec013_postgres` harness |
| Operational determinism | **8.4/10** | Makefile preflight + wait-stack help; still easy to mis-run proofs with stack up |
| Documentation / traceability | **9.0/10** | Per-issue proof docs + implementation evidence folder |
| Release honesty | **8.5/10** | README still says “#216–#233” — tighten in release notes |

**Composite: 9.0/10** — above **8.5** ship threshold for a **patch/minor** release of documented fixes.

---

## 6. Verdict

### Ship?

| Audience | Recommendation |
|----------|----------------|
| **Merge to main + tag (e.g. v0.12.4)** | **Yes** — after normal code review and green PR CI |
| **Production without ops read** | **No** — require #217 runbook for upgraded deployments |
| **Claim “all #216–#233 closed”** | **No** — only documented items above |

### Pre-tag checklist (15 minutes)

```bash
make stop
make spec013-proof-pr
cargo clippy -p edgequake-pipeline -p edgequake-core -p edgequake-api --all-targets --features postgres -- -D warnings
pnpm --dir edgequake_webui exec tsc --noEmit
make backend-bg frontend-bg && make spec013-proof-ui
# Optional before prod: MISTRAL_API_KEY=... make spec013-proof
```

### Post-tag (first week)

1. Run `make spec013-entity-type-audit-all` on staging/production workspaces with strict entity lists.
2. Close GitHub issues **only** when linked to a row in §2.

---

## 7. Re-audit trigger

Re-run this assessment if any of: entity extraction prompt changes, workspace default resolution changes, Playwright gate list changes, or a new issue is added under SPEC-013 without proof.
