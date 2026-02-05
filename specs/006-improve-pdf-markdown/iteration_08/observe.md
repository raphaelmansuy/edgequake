# OODA-08: Observe - Monospace Style Transition Test

## Current State

OODA-07 added a test for bold→italic style transitions. We now have:

- `test_span_rejects_different_style` - Unit test for Span.can_append() rejecting monospace mismatch
- `test_mixed_style_chars_to_spans` - Integration test for bold→italic transitions

## Gap Identified

Missing: Integration test for **monospace** style transitions in `chars_to_spans()`.

- We test bold/italic at integration level
- We test monospace at unit level only
- Need integration test for normal→monospace→normal flow

## Evidence

Current test in pymupdf_grouper.rs tests bold/italic:

```rust
// "AB" (bold) + "cd" (italic)
let chars = vec![
    make_styled_char('A', 60.0, 100.0, 12.0, true, false, false),
    make_styled_char('B', 67.2, 100.0, 12.0, true, false, false),
    make_styled_char('c', 74.4, 100.0, 12.0, false, true, false),
    make_styled_char('d', 81.6, 100.0, 12.0, false, true, false),
];
```

Missing test for:

```rust
// "Hi" (normal) + "code" (monospace) + "!" (normal)
```

## Data Needed

- Location of test file: `layout/pymupdf_grouper.rs`
- Helper function: `make_styled_char` (already exists from OODA-07)
- Expected spans: 3 (normal, monospace, normal)
