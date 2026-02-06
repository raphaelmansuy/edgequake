# OODA Iteration 03 - Decide

**Mission**: Improve PDF-to-Markdown Conversion Quality
**Date**: 2026-02-05

---

## Selected Options

**Option A + B: Expand bullet detection AND normalize output**

---

## Implementation Plan

### Step 1: Create comprehensive BULLETS constant

**File**: `edgequake/crates/edgequake-pdf/src/processors/structure_detection.rs`

Create a static string containing all bullet characters (matching PyMuPDF4LLM):

- Common bullets: \*, -, •, ◦, ▪
- Dashes: –, —, ‐, ‑, ‒, ―
- Geometric shapes: 0x25A0-0x2600 range
- Private use area: 0xF0A7, 0xF0B7

### Step 2: Create helper function

**File**: `edgequake/crates/edgequake-pdf/src/processors/structure_detection.rs`

```rust
fn starts_with_bullet(text: &str) -> bool {
    // Check if first char is in BULLETS
    // AND (single char OR followed by space)
}
```

### Step 3: Update ListDetectionProcessor

Replace regex-based detection with character-based detection.

### Step 4: Run tests

```bash
cargo test --lib structure_detection
cargo test --lib
```

---

## Acceptance Criteria

- [x] BULLETS constant with 530+ characters
- [x] starts_with_bullet() helper function
- [x] ListDetectionProcessor uses new detection
- [x] All 498 tests pass
- [x] New tests for Unicode bullet detection

---

## Rollback Plan

If tests fail, revert to regex-based detection.
