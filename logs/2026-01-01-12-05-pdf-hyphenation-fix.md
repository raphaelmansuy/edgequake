# Task Log: PDF Hyphenation and Block Merge Fixes

**Date**: 2026-01-01 12:05
**Session**: beastmode

## Actions

1. **Fixed vertical gap calculation bug in BlockMergeProcessor**

   - Changed from `(b.bbox.y1 - a.bbox.y2).abs()` to `(a.bbox.y1 - b.bbox.y2).abs()`
   - In PDF coordinates: y2 = top, y1 = bottom; gap = bottom_of_A - top_of_B
   - Blocks were incorrectly not merging due to 57.8pt calculated gap (actual: 2pt)

2. **Fixed Unicode panic in string slicing**

   - Multiple `&text[..len.min(N)]` patterns caused panics on multi-byte chars (e.g., 'ℓ')
   - Replaced with `.chars().take(N).collect()` for safe Unicode handling
   - Fixed in: `should_merge()`, `merge_page_blocks()`, and hyphen continuation debug

3. **Fixed span/text sync issue in HyphenContinuationProcessor**

   - Added `page.blocks[i].spans.clear()` after joining hyphenated text
   - Spans contained old "modifi-" text, causing MarkdownRenderer to output stale content

4. **Fixed span/text sync issue in BlockMergeProcessor**

   - Added `cur.spans.clear()` after `cur.merge(&block)`
   - Block.merge() extends spans from both blocks, which are out of sync with joined text

5. **Fixed sota_test AI enhancement config**

   - Conditionally enable table/readability enhancement only when OPENAI_API_KEY is set
   - Prevents mock provider from replacing content with "Mock response"

6. **Cleaned up debug logging**
   - Removed verbose block-by-block logging from ProcessorChain.process()
   - Removed BlockMerge INPUT logging
   - Removed processor chain count logging

## Decisions

- Clear spans rather than regenerate them (simpler, MarkdownRenderer falls back to block.text)
- Use `.chars()` for all user-facing string operations to handle Unicode safely
- Keep clippy warnings (30 suggestions) for now - focus on functionality fixes

## Next Steps

- Consider regenerating spans from merged text to preserve bold/italic styling
- Address remaining two-column interleaving for affiliation/footer content
- Reduce clippy warnings with `cargo clippy --fix`

## Lessons/Insights

- PDF coordinates: y2 = top (higher Y), y1 = bottom (lower Y) - counterintuitive!
- Spans and text can get out of sync when processors modify one but not the other
- Unicode safety: never use byte slicing `[..N]` on user content, use `.chars()`
- Mock LLM providers can silently replace content - disable AI enhancement without API key
