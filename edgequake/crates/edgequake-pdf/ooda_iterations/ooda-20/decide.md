# OODA-20: Footnote Marker Cleanup - DECIDE

## Decision

Implement `strip_footnote_markers()` in TextCleanupProcessor.

## Implementation Plan

1. Add function to strip leading footnote markers: ⋆, †, ‡, §, ¶
2. Call function in `process_block()` for block.text and span.text
3. Do NOT strip asterisks used for markdown lists

## Code Location

- File: `src/processors/text_cleanup.rs`
- Method: New `strip_footnote_markers(&self, text: &str) -> String`
- Call sites: `process_block()` after other cleanups

## Expected Outcome

- Cleaner markdown output
- Minimal quality score change (symbols are tiny fraction of text)
- Better readability for humans

## Test Verification

- Build: `cargo build -p edgequake-pdf --release`
- Test: `cargo test -p edgequake-pdf --test comprehensive_quality --features comprehensive-tests --release`
