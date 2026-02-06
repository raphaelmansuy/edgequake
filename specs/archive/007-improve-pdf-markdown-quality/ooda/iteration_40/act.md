# IT40 — Act: Implement Font-Aware Word Boundary Detection

## Changes Made

### 1. Modified `src/layout/pymupdf_structs.rs`

Updated `Span::can_append()` to use font-aware space threshold:

**Before (OODA-IT32)**:
```rust
let space_threshold = self.font_size * 0.33;
```

**After (OODA-IT40)**:
```rust
let space_threshold = if self.font_is_monospace.unwrap_or(false) {
    // Monospace: wide inter-char spacing requires higher threshold
    self.font_size * 0.33
} else {
    // Proportional: tight kerning allows lower threshold for better word detection
    self.font_size * 0.22
};
```

## Validation Results

### Test Suite
- **462 tests passed**, 0 failed
- **0 clippy warnings** in edgequake-pdf

### Elitizon Output (Before → After)

| Text | Before (IT39) | After (IT40) |
|------|---------------|--------------|
| Executive summary | "Executivesummary" | "Executive summary" ✅ |
| AI Agent Design & Building | "AIAgentDesign &Building" | "AI Agent Design & Building" ✅ |
| Context Graph & Powerful | "ContextGraph &Powerful" | "Context Graph & Powerful" ✅ |
| Delivery approach | "Deliveryapproach" | "Delivery approach" ✅ |
| Engagement models | "Engagementmodels" | "Engagement models" ✅ |
| Next step | "Nextstep" | "Next step" ✅ |

### File Size Changes

| Document | IT39 | IT40 | Delta |
|----------|------|------|-------|
| Elitizon | 5,268 bytes | 5,338 bytes | +70 bytes (spaces added) |
| LightRAG | 57,262 bytes | 57,292 bytes | +30 bytes (minor) |

The increase in file size is expected — spaces are now being correctly inserted where they were previously missing.

### LightRAG Quality

No regressions observed. The LightRAG PDF uses explicit space characters in the PDF stream, so the threshold change doesn't affect it significantly.

## Commit Ready

All tests pass, code is clean, and the fix resolves the missing-space issue in proportional font documents.
