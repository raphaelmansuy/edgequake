# 20 — E2E Closure Plan (P-G13/P-G14 gaps)

> **Spec**: 021-storage-study
> **File**: 06-first-principles/20-e2e-closure-improvement-plan.md
> **Date**: 2026-06-26
> **Method**: First Principles + DRY + SOLID. Closes the acceptance gaps left by
> plan-19 §12.6 after P-G13/P-G14 shipped without automated proof.

---

## 0. First principles

| Principle | Implication |
|-----------|-------------|
| **Observability must be testable** | If `/live` vs `/health` semantics matter, they need contract + E2E tests — not terminal anecdotes. |
| **Identity must be atomic** | Single-flight without a registry or DB constraint is a TOCTOU race; process-local registry closes the dev/single-worker window. |
| **Honest UI** | `stale: true` in API must be visible — authoritative-looking stale counts are worse than a badge. |
| **DRY** | One `find_active_pdf_processing_task` on `TaskStorage`; one `PdfAdmissionRegistry` on `TaskRuntime`. |
| **SOLID** | SRP: admission registry ≠ task storage ≠ health probes. ISP: trait method optional with default scan fallback. |

---

## 1. Work items

### P-G15 — Indexed single-flight + TOCTOU registry ✅ (2026-06-26)

**Status**: `TaskStorage::find_active_pdf_processing_task` (memory O(n) scan, postgres JSONB query);
`PdfAdmissionRegistry` on `TaskRuntime`; wired in `create_pdf_processing_task`.

### P-G16 — Rust E2E contracts ✅

**File**: `e2e_spec021_ingest_resilience.rs` — 5 tests green.

### P-G17 — Playwright E2E ✅

**File**: `spec021-ingest-resilience.spec.ts` — 2 mocked UI tests green.

### P-G18 — Stale stats UI ✅

`WorkspaceStats.stale: bool`; `StatsCard` `(updating)` badge; `app/page.tsx` + `(dashboard)/page.tsx`.

---

## 2. Out of scope (plan-19 remainder)

P-G2 closed (plan-23). Remaining plan-19: P-G8 Bypass/Mix, query caching, etc.

---

## 3. Verification

```bash
make test-spec021
cargo test -p edgequake-api --test e2e_spec021_ingest_resilience
cd edgequake_webui && bun test src/lib/api/__tests__/backend-readiness.test.ts
cd edgequake_webui && pnpm exec playwright test spec021-ingest-resilience.spec.ts
```
