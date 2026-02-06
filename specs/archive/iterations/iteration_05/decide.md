# OODA-05: Decide - Font Style Detection Improvements

## Decision

Implement **Option A + improved header thresholds**:

1. **Add font name patterns** for bold detection:
   - "medi" (Medium)
   - "semi" (SemiBold)
   - "demi" (DemiBold)

2. **Adjust header level thresholds**:
   - Current: ratio >= 1.5 → H3
   - New: ratio >= 1.4 → H2 (captures the v2 PDF title at ratio 1.445)

## Implementation Plan

### Step 1: Update `is_bold()` in `pymupdf_structs.rs`

```rust
pub fn is_bold(&self) -> bool {
    self.font_name
        .as_ref()
        .map(|n| {
            let lower = n.to_lowercase();
            lower.contains("bold")
                || lower.contains("black")
                || lower.contains("heavy")
                || lower.contains("medi")  // Medium (NimbusRomNo9L-Medi)
                || lower.contains("semi")  // SemiBold
                || lower.contains("demi")  // DemiBold
        })
        .unwrap_or(false)
}
```

### Step 2: Adjust header thresholds in `pymupdf_grouper.rs`

```rust
let level = if ratio >= 1.8 {
    1
} else if ratio >= 1.4 {
    2
} else if ratio >= 1.3 {
    3
} else if ratio >= 1.2 {
    4
} else {
    5
};
```

## Expected Outcomes

| Metric    | Before | Expected | Δ     |
| --------- | ------ | -------- | ----- |
| Format    | 0.343  | 0.40+    | +0.06 |
| Structure | 0.350  | 0.40+    | +0.05 |
| Quality   | 0.675  | 0.70+    | +0.03 |

## Validation

1. Run `eval_comprehensive.py` after changes
2. Check v2_2512.25072v1 specifically for header level and bold detection
3. Ensure no regression on other files

---

**Timestamp**: 2025-01-27
