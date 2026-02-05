# OODA-50: Test Infrastructure Cleanup

## Date: 2026-02-05 (Planned)

## Observe

Multiple test files with overlapping coverage:

- `quality_evaluation.rs` - deprecated
- `comprehensive_quality.rs` - main quality tests
- `fast_quality.rs` - quick smoke tests

### Issues

- Deprecated tests still exist
- Some tests are ignored
- CI runs redundant tests

## Orient

Technical debt from multiple refactoring iterations.

## Decide

Clean up test infrastructure for single source of truth.

## Act

**Status:** PLANNED

Changes to make:

1. Remove deprecated `quality_evaluation.rs`
2. Consolidate to `fast_quality.rs` + `comprehensive_quality.rs`
3. Update CI to run only active tests
4. Add test README explaining structure
