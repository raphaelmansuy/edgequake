# IT11 Decide: Add Test Coverage and Clean Debug Logging

## Decision

1. Add unit test for `is_table_reference` detection logic
2. Clean up verbose debug logging from IT10 investigation
3. Add documentation for the algorithm

## Rationale

```
┌─────────────────────────────────────────────────────────────┐
│                    DECISION MATRIX                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Priority 1: Test Coverage                                  │
│  - is_table_reference logic has no test                     │
│  - Risk: Regression could break Table 4 reconstruction      │
│  - Effort: LOW (simple unit test)                           │
│                                                             │
│  Priority 2: Debug Logging Cleanup                          │
│  - IT10 left verbose tracing::debug! statements             │
│  - Risk: LOG noise in production                            │
│  - Effort: LOW (remove 5-10 lines)                          │
│                                                             │
│  Priority 3: Documentation                                  │
│  - Algorithm is complex, needs clear WHY comments           │
│  - Risk: Future maintainer confusion                        │
│  - Effort: LOW (add comments)                               │
│                                                             │
│  NOT IN SCOPE: Complex table parsing (Tables 1, 2, 5)       │
│  - Requires spatial analysis overhaul                       │
│  - Should be separate iteration with dedicated design       │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Implementation Plan

### 1. Add Test for is_table_reference

```rust
#[test]
fn test_is_table_reference_vs_caption() {
    // Test cases:
    // - "Table 4: Statistics" → is_reference=false (caption)
    // - "Table 4 presents statistical" → is_reference=true (prose)
    // - "Table 4." → is_reference=false (caption)
    // - "Table 4 shows the results" → is_reference=true (prose)
}
```

### 2. Clean Debug Logging

Remove from `scan_for_table`:

- `scan_for_table: starting at idx=...` debug line

### 3. Add Algorithm Documentation

Document the `is_table_reference` detection in WHY comment format.

## Expected Outcome

- Test coverage for IT10 changes
- Cleaner log output
- Better maintainability
