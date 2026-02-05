# Decide – OODA-21: Add FormulaConfig Builder Tests

## Decision

Add 3 tests for FormulaConfig builder methods to ensure they work correctly.

## Tests to Add

```rust
#[test]
fn test_formula_config_with_min_density() {
    let config = FormulaConfig::new().with_min_density(0.25);
    assert_eq!(config.min_math_density, 0.25);
    // Other fields should remain at defaults
    assert_eq!(config.min_confidence, 0.5);
}

#[test]
fn test_formula_config_with_min_confidence() {
    let config = FormulaConfig::new().with_min_confidence(0.8);
    assert_eq!(config.min_confidence, 0.8);
    // Other fields should remain at defaults
    assert_eq!(config.min_math_density, 0.15);
}

#[test]
fn test_formula_config_new_equals_default() {
    let from_new = FormulaConfig::new();
    let from_default = FormulaConfig::default();
    // WHY: new() and Default should produce equivalent configs
    assert_eq!(from_new.min_math_density, from_default.min_math_density);
    assert_eq!(from_new.min_confidence, from_default.min_confidence);
    assert_eq!(from_new.superscript_threshold, from_default.superscript_threshold);
    assert_eq!(from_new.subscript_threshold, from_default.subscript_threshold);
    assert_eq!(from_new.detect_inline, from_default.detect_inline);
    assert_eq!(from_new.detect_display, from_default.detect_display);
}
```

## Location

Add to `src/formula/detector.rs` test module after existing tests.
