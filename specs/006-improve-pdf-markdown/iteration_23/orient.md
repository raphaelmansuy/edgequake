# Orient – OODA-23: Convert TODOs to KNOWN LIMITATION Comments

## Strategy

Replace generic TODO comments with specific KNOWN LIMITATION comments that:

1. Document WHY the limitation exists
2. Link to relevant issues/discussions if any
3. Suggest future implementation approaches
4. Note workarounds if available

## Template

```rust
// KNOWN LIMITATION: Feature X not implemented
// WHY: Requires Y which is complex because Z
// WORKAROUND: Use alternative approach A
// FUTURE: Could implement using technique B
```

## Benefits

1. More informative than "TODO"
2. Sets realistic expectations
3. Documents technical constraints
4. Helps future contributors understand context
