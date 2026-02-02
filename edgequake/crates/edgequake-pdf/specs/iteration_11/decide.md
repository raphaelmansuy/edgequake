# OODA-11 DECIDE: Add Academic Reference Detection

## Decision

Add academic reference pattern `[N]` detection to `ListDetectionProcessor` in `structure_detection.rs`.

## Implementation Plan

### Step 1: Modify ListDetectionProcessor

Location: `src/processors/structure_detection.rs` lines 320-323

**Current code:**

```rust
let bullet_regex = Regex::new(r"^[-–—*•◦▪]\s+").unwrap();
let number_regex = Regex::new(r"^\d+[\.)]\s+").unwrap();
```

**New code:**

```rust
let bullet_regex = Regex::new(r"^[-–—*•◦▪]\s+").unwrap();
let number_regex = Regex::new(r"^\d+[\.)]\s+").unwrap();
// WHY: Academic papers use [N] for references, arXiv papers commonly have 20-50+
let ref_regex = Regex::new(r"^\[\d{1,3}\]\s*").unwrap();
```

### Step 2: Update Detection Logic

**Current code (line 346):**

```rust
if bullet_regex.is_match(text) || number_regex.is_match(text) {
```

**New code:**

```rust
if bullet_regex.is_match(text) || number_regex.is_match(text) || ref_regex.is_match(text) {
```

### Step 3: Verify with Test

Run comprehensive quality test and compare v2 PDF scores.

## Expected Impact

- v2 PDF structural fidelity: 47.2% → ~75%
- Overall structural average: 69.0% → ~78%
- Overall quality: 76.2% → ~82%

## Risk Assessment

- **Low risk**: The `[N]` pattern is unambiguous in PDF text
- **Edge case**: Inline citations like "as shown in [1]" - BUT these appear mid-sentence, not at line start
- The regex requires `^` (line start) so inline citations won't be affected

## Alternative Considered

Could also use a more lenient regex like `\[\d+\]` without requiring line start, but:

1. Would match inline citations incorrectly
2. Would match markdown image syntax `![alt]`
3. Would be more error-prone

The `^` anchor is intentional and correct.

## Proceed to ACT Phase
