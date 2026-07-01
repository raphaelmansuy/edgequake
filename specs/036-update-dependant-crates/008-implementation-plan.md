# SPEC-036 — Implementation Plan

**Status:** `COMPLETE` (2026-07-01)

---

## Phase 0 — Preflight

- [x] **P0.1** PR #79 CI green — rustfmt + audit via dependency bumps
- [x] **P0.2** Local gate on edgequake-llm@0.6.26
- [x] **P0.3** `edgequake-pdf2md@0.9.2` on crates.io
- [x] **P0.4** `edgeparse-core@0.2.5` on crates.io
- [x] **P0.5** pdf2md cosmetic diffs discarded

---

## Phase 1 — edgeparse-core (verify only)

- [x] **P1.1** EdgeQuake compiles with registry `0.2.5`
- [x] **P1.2** No PR required

---

## Phase 2 — edgequake-llm@0.6.26

- [x] **P2.1–P2.7** PR #79 merged, tagged, published to crates.io
- [x] **P2.8** docs.rs — crate indexed (page may lag)

---

## Phase 3 — edgequake-pdf2md

- [x] **P3A.1–P3A.2** Option A: reuse 0.9.2 from registry
- [x] **P3B** Option B skipped

---

## Phase 4 — EdgeQuake detach from local patches

- [x] **P4.1** `Cargo.toml` — registry pins, no patches
- [x] **P4.2** `Cargo.lock` regenerated from crates.io
- [x] **P4.3** Lock sources verified registry
- [x] **P4.4** `cargo test --workspace --lib --locked` — 860+ pass, 0 fail
- [x] **P4.4b** Test fixes: startup_security, tenant scoping, RRF fusion, auth env
- [x] **P4.8** Committed to edgequake repo

---

## Phase 5 — Post-migration hygiene

- [x] **P5.1** VS Code workspace unchanged (optional local dev)
- [x] **P5.3** SPEC-036 marked COMPLETE
- [x] **P5.4** `cargo clean` on edgequake-llm, edgequake-pdf2md, edgeparse

---

## Execution log

| Date | Step | Result |
|------|------|--------|
| 2026-07-01 | Spec authored | ✅ |
| 2026-07-01 | Security: quinn-proto 0.11.15, anyhow 1.0.103 | ✅ |
| 2026-07-01 | llm 0.6.26 published | ✅ |
| 2026-07-01 | EdgeQuake registry migration | ✅ |
| 2026-07-01 | Test fixes (4 tests) | ✅ |
| 2026-07-01 | Dependent crate cargo clean | ✅ |
| 2026-07-01 | EdgeQuake commit | ✅ |
