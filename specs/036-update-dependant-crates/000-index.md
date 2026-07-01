# SPEC-036 — Publish Dependent Crates & Detach EdgeQuake from Local Path Patches

**Spec:** `036-update-dependant-crates`  
**Date:** 2026-07-01  
**Completed:** 2026-07-01  
**Method:** Code is law — every claim cites a file, tag, or lock entry.  
**Status:** `COMPLETE` — all three deps from crates.io; `edgequake-llm@0.6.26` published  
**Goal:** EdgeQuake builds from **official crates.io** versions only; no `[patch.crates-io]` or sibling `path` deps in production.

---

## TL;DR — Final State

| Crate | crates.io | EdgeQuake pin | Lock source | Status |
|-------|-----------|---------------|-------------|--------|
| `edgequake-llm` | **0.6.26** ✅ | `"0.6.26"` | `registry+crates.io` | Published + consumed |
| `edgequake-pdf2md` | **0.9.2** ✅ | `version = "0.9.2"` | `registry+crates.io` | Consumed (no republish) |
| `edgeparse-core` | **0.2.5** ✅ | `"0.2.5"` | `registry+crates.io` | Consumed (no republish) |

**Removed from `edgequake/Cargo.toml`:** entire `[patch.crates-io]` block + `path = "../../edgequake-*"` deps.

**Security (first principles, not audit-ignore):** `edgequake-llm@0.6.26` lockfile pins `quinn-proto 0.11.15` (RUSTSEC-2026-0185) and `anyhow 1.0.103` (RUSTSEC-2026-0190).

---

## Execution Summary

```
Phase 1  edgeparse-core 0.2.5     ✅ verify only (already published)
Phase 2  edgequake-llm 0.6.26     ✅ PR #79 merged → tag v0.6.26 → crates.io + GitHub release
Phase 3  edgequake-pdf2md 0.9.2   ✅ Option A (reuse existing; no 0.9.3 hygiene release)
Phase 4  EdgeQuake migration      ✅ Cargo.toml + Cargo.lock updated; tests verified
Phase 5  Spec hygiene             ✅ this update
```

**Remaining (non-blocking):** open EdgeQuake PR/commit for `Cargo.toml` + `Cargo.lock` changes (local, uncommitted).

---

## Documents

| File | Purpose |
|------|---------|
| [002-first-principles.md](./002-first-principles.md) | Why registry-only deps; invariants & edge cases |
| [003-current-assessment.md](./003-current-assessment.md) | Pre/post migration audit |
| [004-dependency-dag.md](./004-dependency-dag.md) | Version matrix, API surface, publish gates |
| [008-implementation-plan.md](./008-implementation-plan.md) | Phased checklist (all phases checked) |
| [009-cross-reference-matrix.md](./009-cross-reference-matrix.md) | Claim → evidence traceability |
| [010-completion-report.md](./010-completion-report.md) | Final verification results |

---

## Success Criteria — Scorecard

| # | Criterion | Result |
|---|-----------|--------|
| 1 | All three crates on crates.io at target versions | ✅ |
| 2 | No `[patch.crates-io]` or path deps for these crates | ✅ |
| 3 | `cargo build --locked` succeeds | ✅ |
| 4 | SPEC-036 targeted tests pass | ✅ (see 010-completion-report) |
| 5 | CHANGELOG + GitHub release for new publish | ✅ llm 0.6.26 |
| 6 | docs.rs build | ⏳ 404 at completion time (normal lag; crate indexed) |

---

## External Links

- [edgequake-llm v0.6.26 release](https://github.com/raphaelmansuy/edgequake-llm/releases/tag/v0.6.26)
- [edgequake-llm on crates.io](https://crates.io/crates/edgequake-llm/0.6.26)
- [edgequake-pdf2md on crates.io](https://crates.io/crates/edgequake-pdf2md/0.9.2)
- [edgeparse-core on crates.io](https://crates.io/crates/edgeparse-core/0.2.5)
