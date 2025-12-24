# Task Log: Query Interface UX Fixes

**Date:** 2024-12-24 02:45 UTC
**Mode:** beastmode

## Actions

1. **Fixed markdown rendering issue** - Added `normalizeMarkdown` function in `markdown-renderer.tsx` to fix broken markdown syntax from streaming tokenization. The function fixes spacing issues around `**`, `*`, backticks, and `~~` markers.

2. **Fixed sticky input layout** - Changed parent container from `relative` to `overflow-hidden` and removed unnecessary `sticky bottom-0` from input div. The flexbox layout with `flex-shrink-0` correctly keeps input at bottom.

3. **Verified New Conversation button** - Button already present with `Plus` icon and proper clear conversation logic.

4. **Verified smart auto-scroll** - Implementation in place with:
   - `shouldAutoScroll` state to track user intent
   - `scrollAnchorRef` for smooth scroll target
   - Scroll detection to disable auto-scroll when user scrolls up
   - Auto-re-enable scroll when streaming starts

## Decisions

- Used regex-based markdown normalization instead of modifying SSE parsing to avoid breaking other consumers
- Removed `sticky bottom-0` in favor of pure flexbox layout which is more reliable
- Added `overflow-hidden` to parent to ensure proper scroll containment

## Next Steps

1. Manual testing in browser to verify:
   - Markdown renders correctly (no visible `**` asterisks)
   - Input stays fixed at bottom during scroll
   - New Conversation button clears chat
   - Auto-scroll works during streaming but allows user to scroll up

## Lessons/Insights

- Streaming tokens often include whitespace that breaks markdown syntax like `** text **` instead of `**text**`
- Pure flexbox layout (`flex-col` + `flex-1` for scrollable + `flex-shrink-0` for fixed) is more reliable than `sticky` positioning
