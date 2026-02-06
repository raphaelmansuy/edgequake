# OODA IT37 — Act

## Changes Made

### 1. Enhanced GarbledTextFilterProcessor
**File:** `src/processors/text_cleanup.rs:625-700`

Added two new detection checks to `is_garbled()`:

**Long-word detection (line ~667):**
```rust
// Any word > 40 chars (not a URL/path) → garbled
let has_long_word = words.iter().any(|w| {
    w.len() > 40 && !w.contains("://") && !w.contains('/')
});
```

**Low-space-ratio detection (line ~690):**
```rust
// Text > 80 chars with < 5% spaces → garbled
let space_ratio = space_count as f32 / trimmed.len() as f32;
if space_ratio < 0.05 { return true; }
```

### 2. Header Number-Title Spacing
**File:** `src/processors/structure_detection.rs:450-490`

Added `normalize_section_number_spacing()` helper:
- Finds leading digit/dot sequence
- If next char is uppercase, inserts space
- "1INTRODUCTION" → "1 INTRODUCTION"

Applied in first pass of `process()` (line ~198) BEFORE level check:
```rust
for block in &mut page.blocks {
    if raw_text.starts_with(|c: char| c.is_ascii_digit()) {
        let normalized = Self::normalize_section_number_spacing(&raw_text);
        if normalized != raw_text { block.text = normalized; }
    }
}
```

### 3. Renderer Fallback
**File:** `src/renderers/markdown.rs:282-298`

Modified `render_header()` to detect when block.text differs from spans:
```rust
let span_raw: String = block.spans.iter().map(|s| s.text.as_str()).collect();
let text = if !block.spans.is_empty() && span_raw.trim() == block.text.trim() {
    self.render_spans_styled(&block.spans, true, false)
} else {
    self.clean_text(&block.text)
};
```

### 4. Tests Added
- `test_normalize_section_number_spacing_basic` — "1INTRODUCTION" → "1 INTRODUCTION"
- `test_normalize_section_number_spacing_with_dot` — "3.2DUAL" → "3.2 DUAL"
- `test_normalize_section_number_spacing_already_spaced` — no change
- `test_normalize_section_number_spacing_no_match` — normal text unchanged
- `test_garbled_long_word_detection` — >40 char words detected
- `test_garbled_long_word_url_exception` — URLs not flagged
- `test_garbled_low_space_ratio` — <5% spaces detected
- `test_garbled_short_text_not_flagged` — short text not affected

## Results

### Before IT37
```
#### 1INTRODUCTION
#### 2RETRIEVAL-AUGMENTED GENERATION
Page 3: 5 garbled blocks, 60421 bytes total
```

### After IT37
```
#### 1. INTRODUCTION
#### 2. RETRIEVAL-AUGMENTED GENERATION
Page 3: garbled blocks removed, 58993 bytes total (-1428 bytes of noise)
```

### Test Results
- **457 tests pass** (449 existing + 8 new), 0 failures
- **0 clippy warnings** in edgequake-pdf
- **Elitizon:** 84 blocks, 5332 bytes — no regression
- **LightRAG:** Headers properly spaced, page 3 significantly cleaner
