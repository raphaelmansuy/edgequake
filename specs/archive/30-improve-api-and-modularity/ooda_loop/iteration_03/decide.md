# OODA Iteration 03 - Decide

**Date**: 2026-01-07
**Focus**: Documentation fix implementation plan

## Decision

Fix all 5 rustdoc warnings with minimal invasive changes.

## Implementation Plan

### Step 1: Fix edgequake-llm URL warnings

- [gemini.rs:203](../../edgequake/crates/edgequake-llm/src/providers/gemini.rs#L203): Wrap URL in angle brackets
- [azure_openai.rs:6](../../edgequake/crates/edgequake-llm/src/providers/azure_openai.rs#L6): Use backticks for inline code
- [azure_openai.rs:148](../../edgequake/crates/edgequake-llm/src/providers/azure_openai.rs#L148): Use backticks for inline code

### Step 2: Fix edgequake-core HTML tag warnings

- [relationship.rs:19](../../edgequake/crates/edgequake-core/src/types/relationship.rs#L19): Use backticks for `<SEP>`
- [relationship.rs:41](../../edgequake/crates/edgequake-core/src/types/relationship.rs#L41): Use backticks for `<SEP>`

### Step 3: Verify

1. Run `cargo doc --workspace --no-deps`
2. Confirm 0 warnings from edgequake crates
3. Run `cargo test --workspace`
4. Confirm non-regression

## Success Criteria

- 0 doc warnings in edgequake crates
- All tests pass
