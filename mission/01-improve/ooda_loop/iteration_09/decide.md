# Decide — Iteration 09
Date: 2026-04-10

## Scope
1. Add `ResultExt` trait with `.internal_err(context)` method to `error.rs`
2. Add `parse_uuid(s, label)` helper
3. Migrate the recovery handlers (stuck.rs, reprocess.rs) as proof-of-concept

## Will NOT change
- All 57 sites at once (too risky)
- Public API contract
