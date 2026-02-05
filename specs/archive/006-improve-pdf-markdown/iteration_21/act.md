# Act – OODA-21: Added FormulaConfig Builder Tests

## What Changed

Added 3 new tests to `formula/detector.rs`:

1. **`test_formula_config_with_min_density`**: Verify builder sets density and preserves defaults
2. **`test_formula_config_with_min_confidence`**: Verify builder sets confidence and preserves defaults
3. **`test_formula_config_new_equals_default`**: Verify `new()` and `Default` produce equivalent configs

## Code Location

- `edgequake/crates/edgequake-pdf/src/formula/detector.rs`

## Verification

```
cargo test formula_config --lib
# Result: 3 passed

cargo test --lib
# Result: 466 passed (up from 463)
```

## Value Added

- Builder pattern methods now have dedicated tests
- Documents expected behavior for config construction
- Ensures `new()` and `Default::default()` remain equivalent

## Next Iteration

OODA-22: Continue test coverage improvements
