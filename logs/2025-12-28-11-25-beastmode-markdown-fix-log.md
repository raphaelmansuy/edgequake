# Task Log: Markdown Rendering Bug Fix

**Date**: 2025-12-28
**Mode**: Beastmode
**Task**: Fix markdown rendering issues in Query page

## Actions

- Analyzed screenshot identifying two markdown issues: trailing space before `**` and `**` attached to wrong word
- Investigated server-side `chat.rs` - confirmed NO content modification (raw tokens passed through)
- Investigated client-side `StreamingMarkdownRenderer.tsx` - found `normalizeMarkdownForStreaming()` function
- Tested existing regex patterns - found missing pattern for `word** text` case
- Added new "Pattern 0" regex patterns for all markdown marker types (\*_,_,\__,_,~~)
- Updated documentation in `archive/plan_streaming_improvements/plan.md` and `scratchpad.md`

## Decisions

- Root cause: CLIENT-SIDE - LLM tokenizers attach `**` to previous word during streaming
- Fix location: `normalizeMarkdownForStreaming()` function in StreamingMarkdownRenderer.tsx
- Pattern: `([a-zA-Z0-9])\*\* (\w)` → `$1 **$2` moves marker to correct position
- Applied same fix pattern for all markdown markers for consistency

## Next Steps

- User to verify fix in browser by running the app and testing Query page
- Optional: Add unit tests for the new regex patterns
- Optional: Run E2E tests to verify no regressions

## Lessons/Insights

- LLM tokenizers add leading spaces to word tokens, causing markdown markers to attach to wrong words
- The streaming pipeline has two normalization steps: `normalizeMarkdownForStreaming()` then `addSpacesAroundMarkdown()`
- Both functions work together to fix all marker placement issues
