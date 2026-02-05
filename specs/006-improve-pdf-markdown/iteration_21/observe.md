# Observe – OODA-21: FormulaConfig Builder Method Tests

## Current State

- `formula/detector.rs` has 21 tests (good coverage overall)
- FormulaConfig builder methods NOT tested directly:
  - `with_min_density()` - 0 direct tests
  - `with_min_confidence()` - 0 direct tests

## Gap Analysis

The builder pattern methods are useful utilities but have no dedicated tests. While they're simple, testing them:
1. Documents expected behavior
2. Catches regressions if internals change
3. Increases coverage metrics

## Test Plan

Add tests for:
1. `FormulaConfig::with_min_density()` 
2. `FormulaConfig::with_min_confidence()`
3. `FormulaConfig::new()` vs `Default::default()` equivalence
