# OODA-29 Act: LLM Enhance Module Documentation

## Changes Made

### 1. Added WHY Documentation

**File**: `src/processors/llm_enhance.rs`

Added 2 WHY comments explaining design decisions:

#### LlmEnhanceProcessor Struct (line ~50)

```rust
/// **WHY use LLM for post-processing?**
/// - Tables as raw text → proper markdown or HTML
/// - Math rendered as Unicode → LaTeX equations
/// - OCR text with errors → cleaned text
/// - improve_text=false by default (too aggressive, can modify correct text)
```

#### text_needs_improvement Function (line ~440)

```rust
/// WHY these thresholds?
/// - special_char_ratio > 0.3: Heavy symbols suggest OCR noise or encoding issues
/// - digit_ratio > 0.4: Mostly numbers often indicates mangled formulas
/// - alpha_ratio < 0.5: Text should be mostly letters for normal prose
```

### 2. Added Unit Tests

**File**: `src/processors/llm_enhance.rs`

Added 2 new tests:

1. **test_processor_with_image_ocr_enabled**: Validates the `with_image_ocr_enabled()` builder method creates a properly configured processor with default OCR config.

2. **test_processor_with_custom_image_ocr**: Validates the `with_image_ocr()` builder method accepts custom ImageOcrConfig with specific model settings.

## Verification

```bash
# Tests pass
cargo test --lib -- llm_enhance
# All 7 llm_enhance tests pass (5 existing + 2 new)

# Full suite
cargo test --lib
# 486 tests pass (was 484)
```

## Metrics

| Metric                     | Before | After | Delta |
| -------------------------- | ------ | ----- | ----- |
| WHY Comments (llm_enhance) | 0      | 2     | +2    |
| Tests (llm_enhance)        | 5      | 7     | +2    |
| Total Lib Tests            | 484    | 486   | +2    |
| Clippy Warnings            | 0      | 0     | ±0    |

## Commit Message

```
OODA-29: Add WHY docs and builder tests to LLM enhance module

- Add WHY explaining post-processing strategy (tables→markdown, etc)
- Add WHY explaining text_needs_improvement thresholds
- Add test_processor_with_image_ocr_enabled
- Add test_processor_with_custom_image_ocr
- Tests: 484 → 486 (+2)
```
