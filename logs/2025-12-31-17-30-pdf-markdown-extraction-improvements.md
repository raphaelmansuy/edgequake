# Task Log: PDF to Markdown Extraction Improvements

**Date**: 2025-12-31 17:30
**Mode**: beastmode

## Actions

- Updated `convert_one_tool.rs` example to use OpenAI provider when `OPENAI_API_KEY` is set
- Improved image description fallback with dimensions and type inference (Figure/Chart, Banner/Header, Icon/Symbol)
- Added `heuristic_table_parse` function for column-separated text to Markdown table conversion
- Added mock response detection to generate meaningful fallback descriptions
- Fixed 3 clippy warnings: `manual_range_contains`, `needless_range_loop`, `single_char_add_str`
- Configured example to process first 6 pages for faster demo

## Decisions

- Use intrinsic `PdfImage.width()/.height()` methods instead of decoding image bytes
- Infer image type from dimensions and aspect ratio when AI is unavailable
- Detect "Mock response" from LLM and fall back to heuristic descriptions
- Limit example to 6 pages to keep API costs and time reasonable

## Next Steps

- Consider adding vision-capable LLM for actual image content analysis
- Improve two-column PDF layout handling
- Add support for table caption extraction from surrounding context

## Lessons/Insights

- Mock LLM detection enables graceful fallback behavior without code path changes
- Image dimensions provide useful context even without vision AI
- Column-separated text can be heuristically parsed into Markdown tables

## Files Modified

- `edgequake/crates/edgequake-pdf/src/extractor.rs` - Enhanced image and table processing
- `edgequake/crates/edgequake-pdf/examples/convert_one_tool.rs` - Added OpenAI support and config

## Test Results

- 15 tests passing (4 unit + 10 integration + 1 doctest)
- Zero clippy warnings for edgequake-pdf crate
- Successful extraction with both OpenAI and mock providers
