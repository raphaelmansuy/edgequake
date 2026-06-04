# P3 — Chunker test extraction (PIPE-DRY-009 / PIPE-SOLID-S-003)

**Status:** ✅ Proven  
**Date:** 2026-06-04

## Claim

Production `chunker/mod.rs` no longer embeds ~400 LOC of tests. Chunker integration tests live in `tests/chunker_tests.rs` as top-level `#[test]` functions.

## Before / after

| File | Before | After |
|------|--------|-------|
| `src/chunker/mod.rs` | ~644 LOC (tests inline) | ~159 LOC (production only) |
| `tests/chunker_tests.rs` | — | 487 LOC, 29 tests |

## Commands

```bash
cargo test -p edgequake-pipeline --test chunker_tests              # 29/29
cargo test -p edgequake-pipeline spec017_chunker_strategy_changes_output  # contract
wc -l edgequake/crates/edgequake-pipeline/src/chunker/mod.rs
```

## Evidence

- `test_chunker_strategy_changes_output` — sentence vs token strategies differ.
- `test_sentence_boundary_*`, `test_paragraph_boundary_*` — strategy tests in integration crate.
- `text_utils` helpers (`estimate_tokens`, `calculate_line_numbers`, char boundaries) exported for test reuse.
