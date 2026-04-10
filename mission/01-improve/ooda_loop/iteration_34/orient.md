# OODA-34 — Orient

## Analysis

Both parsers are pure functions with well-defined input/output contracts and multiple edge cases.
They are the most impactful untested files in the pipeline crate — directly responsible for extracting
entities and relationships from LLM output.

## First Principles

1. **Parse robustness**: LLM output is inherently noisy — parsers must be resilient
2. **Business rule enforcement**: BR0006 (no self-ref), BR0004 (keyword limit 5)
3. **Normalization**: Entity names must be normalized before comparison
4. **Metadata tracking**: Parse errors must be counted for observability

## Risk Assessment

- Low risk: all functions are pure, no side effects
- Tests can run in isolation without any external dependencies
- Module visibility: both modules are `pub use`-d from `mod.rs` — accessible from test modules
