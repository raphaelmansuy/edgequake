# Iteration 39: Observe + Orient + Decide + Act

## Focus: Issue 9 - Character Handling

### Observation

No existing sanitization module in edgequake-pipeline.
Entity normalization exists (`normalize_entity_name`) but for output, not input.

### Analysis

Need input sanitization before LLM extraction to handle:

- Emojis (can confuse tokenizers, waste tokens)
- Control characters (invisible, break output parsing)
- Zero-width characters (invisible, affect tokenization)
- RTL markers (can flip text direction)
- Unicode normalization (canonical form)

### Decision

Create new `sanitizer.rs` module with:

- Configurable `SanitizeConfig` struct
- `Sanitizer` with `sanitize()` method
- Multiple emoji modes: Preserve, Remove, Replace
- Helper functions for each sanitization type
- 10 unit tests for all features

### Actions Taken

| File           | Change                                                       |
| -------------- | ------------------------------------------------------------ |
| `sanitizer.rs` | Created 450+ lines with full implementation                  |
| `lib.rs`       | Added `pub mod sanitizer`                                    |
| `lib.rs`       | Added exports for `Sanitizer`, `SanitizeConfig`, `EmojiMode` |
| `Cargo.toml`   | Added `unicode-normalization = "0.1"`                        |

### Implementation Summary

```rust
pub struct SanitizeConfig {
    pub normalize_unicode: bool,        // NFC normalization
    pub emoji_mode: EmojiMode,          // Preserve/Remove/Replace
    pub remove_control_chars: bool,     // Remove C0/C1 controls
    pub remove_zero_width: bool,        // Remove ZWS, ZWSP, etc.
    pub remove_directional_markers: bool, // Remove LRM, RLM, etc.
    pub collapse_whitespace: bool,      // Collapse multiple spaces
    pub max_consecutive_newlines: usize, // Limit blank lines
}

pub enum EmojiMode {
    Preserve,
    Remove,
    ReplaceWithPlaceholder,
}
```

### Test Results

```
test sanitizer::tests::test_collapse_whitespace ... ok
test sanitizer::tests::test_control_chars_removed ... ok
test sanitizer::tests::test_default_config ... ok
test sanitizer::tests::test_directional_markers_removed ... ok
test sanitizer::tests::test_emoji_preserve ... ok
test sanitizer::tests::test_emoji_remove ... ok
test sanitizer::tests::test_emoji_replace ... ok
test sanitizer::tests::test_limit_newlines ... ok
test sanitizer::tests::test_unicode_normalization ... ok
test sanitizer::tests::test_zero_width_removed ... ok

10 passed; 0 failed
```

### Issue 9 Status: ✅ COMPLETE
