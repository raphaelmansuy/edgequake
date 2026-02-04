# OODA-09 Decide: Add Missing Font Patterns

## Decision

### Fix 1: Add "ital" pattern for italic detection

**Files to modify:**

1. `font_handling.rs` (lopdf backend)
2. `pymupdf_structs.rs` (pdfium backend)

**Change:**

```rust
// Before
lower.contains("italic") || lower.contains("oblique")

// After
lower.contains("italic")
    || lower.contains("oblique")
    || lower.contains("ital")  // Catches ReguItal, MediItal
```

### Fix 2: Re-enable "medi" pattern for bold detection

**Files to modify:**

1. `font_handling.rs` (lopdf backend)
2. `pymupdf_structs.rs` (pdfium backend - already has it)

**Change in font_handling.rs:**

```rust
// Before: "medi" was intentionally disabled
let is_bold = lower.contains("bold")
    || lower.contains("black")
    || lower.contains("heavy")
    || lower.contains("sfbx")
    || lower.contains("cmbx")
    || lower.contains("-bold");

// After: Re-enable "medi"
let is_bold = lower.contains("bold")
    || lower.contains("black")
    || lower.contains("heavy")
    || lower.contains("sfbx")
    || lower.contains("cmbx")
    || lower.contains("medi")  // Re-enabled for abstract bold
    || lower.contains("-bold");
```

## Expected Impact

- Format score should improve significantly
- v2_2512 italic should go from 0% to ~30%
- Abstract text should now render as **bold**
