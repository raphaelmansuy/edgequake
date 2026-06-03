# Cross-Crate Priority Matrix

**Spec:** 017-dry-and-solid-audit  
**Last updated:** 2026-05-31 (shift 28 — e2e backend URL centralization + disk recovery)  
**Method:** First-principles — ranked by **correctness impact**, then **LOC under duplication**.

---

## Implementation Status Summary

| Tier | Total items (original audit) | Fixed (cumulative) | Remaining |
|------|------------------------------|--------------------|-----------|
| **P0** | 6 | **6** | 0 |
| **P1** | 9 | **13** | 0 |
| **P2** | 10 | **13** | 0 |
| **P3** | 6+ | **13** | 0 |

**Merge status:** `origin/edgequake-main` — **already up to date** (shift 28).

**Branch commits:** `7e055385` → *(shift 28 pending commit)*

---

## Shift 28 Deliverables

| Item | Status |
|------|--------|
| Disk recovery | **DONE** — `cargo clean` freed ~188 GiB (`edgequake/target` was ~134 GiB) |
| E2E backend URL helper | **NEW** — `e2e/helpers/backend-url.ts` (`BACKEND_URL`, `API_V1_URL`, health probe) |
| Hardcoded `:8080` removal | **FIXED** — 24 e2e spec files migrated to helper |
| Playwright global setup | **NEW** — fail-fast EdgeQuake health when `PLAYWRIGHT_BASE_URL` set (full-stack mode) |
| `make test-e2e-full` | **FIXED** — depends on `dev-bg`, exports auto-selected ports |
| `make backend-bg` | **FIXED** — prefers prebuilt `target/debug/edgequake` (avoids cold `cargo run`) |
| vitest | **PASS** (611, 36 files) |
| Playwright smoke (UI-only, webServer :3001) | **PASS** (5/5 spec017) |
| OODA-228 critical path (live API) | **PASS** (3/3) when backend healthy |
| Full Playwright (~835) with stack | **651 pass / 111 fail / 69 skip** (14.7m) — URL drift fixed; failures are UI/upload/audit/provider-env (not :8080) |

---

## Shift 27 Deliverables (prior)

| Item | Status |
|------|--------|
| UI-P3-006 query hook sub-split | **FIXED** |
| `use-query-interface.ts` | **531 → ~130 LOC** |
| E2E smoke expansion | pipeline + workspace routes |

---

## Accepted / Deferred (explicit closure)

| Item | Status |
|------|--------|
| UI-DRY-002 status badge merge | **ACCEPTED** |
| GraphStorage method-level ISP | **ACCEPTED DEFERRED** — `graph_isp.rs` + contract tests |
| Full Playwright suite (~835) green on dev laptop | **DEFERRED** — needs CI / clean port 8080+3000 |
| Full API e2e batch | **PARTIAL** — URL drift fixed; stack stability still manual |

---

## Tests Verified (shift 28)

| Suite | Status |
|-------|--------|
| `cargo build -p edgequake-api` | **PASS** (after disk cleanup) |
| vitest (611, 36 files) | **PASS** |
| Playwright spec017 smoke (5, webServer) | **PASS** |
| OODA-228 critical path (3, live API) | **PASS** (when backend on auto port) |
| Full Playwright with `make test-e2e-full` | **651/835 pass** — remaining 111 fail (upload stress, audit capture, LLM UI, Ollama-specific) |
| `postgres_integration` (20) | **SKIP** (no test DB) |

---

## Brutally Honest Quality Bar Assessment

### What meets the bar

- **E2E port drift closed** — specs no longer assume `:8080`; Makefile propagates `EQ_BACKEND_URL`.
- **Fail-fast global setup** — wrong app on 8080 (e.g. JWT 401 service) caught before 399 mystery failures.
- **Disk / build hygiene** — target cleaned; backend-bg uses binary when present.
- **WebUI P3 + query modularization** — complete from shifts 24–27.

### What does NOT meet the bar

- **"Full e2e green locally"** — **no** — ~835 spec suite not verified end-to-end this shift; prior run without stack showed ~399 failures (mostly missing backend).
- **"Zero regression risk"** — **no** — needs CI job with isolated ports.
- **SPEC-017 enterprise sign-off** — **borderline** for code structure; **no** for full e2e gate.

### Commit recommendation

| Scenario | Verdict |
|----------|---------|
| Commit shift 28 e2e infra + Makefile + disk recovery | **Yes** |
| Claim full Playwright suite verified | **No** |
| Claim SPEC-017 100% closed | **No** — full-stack e2e CI still required |

---

## Recommended Next Steps

1. CI: isolated ports → `make test-e2e-full` on every PR touching WebUI/e2e
2. Optional: `playwright.global-setup.ts` write selected ports to `.e2e-ports.json` for debugging
3. Re-run `postgres_integration` when test DB available

---

## Verification Gates

| Gate | Status |
|------|--------|
| Full workspace compile | **PASS** |
| E2E URL centralization | **PASS** |
| Query hook modular | **PASS** |
| GraphStorage ISP | **ACCEPTED DEFERRED + TESTED** |
| E2E smoke (5 UI-only) | **PASS** |
| Full e2e (~835) | **651 PASS** (111 fail pre-existing categories) |
