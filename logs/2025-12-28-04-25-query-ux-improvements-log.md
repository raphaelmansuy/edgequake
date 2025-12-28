# Task Log: Query Page UX Improvements

**Date:** 2025-12-28 04:25
**Mode:** beastmode

## Actions

- Fixed autoscroll: Changed scroll anchor from `h-6` (24px) to `h-32` (128px) to clear input area
- Created `NonStreamingLoadingIndicator` component with 3-phase animation (Searching → Analyzing → Generating)
- Added display condition for non-streaming loading indicator at line 973
- Added `hasIncompleteHR()` and `extractContentBeforeIncompleteHR()` functions to streaming-utils.ts
- Updated `analyzeStreamingContent()` to detect incomplete HR patterns
- Changed HR rendering from `border-border` to gradient fade (`from-transparent via-border to-transparent`)
- Updated streaming cursor from `bg-zinc-400 animate-cursor-blink` to `bg-primary/70 animate-pulse` (theme-aware)

## Decisions

- Used `h-32` (128px) for scroll anchor to ensure clearance above input area (~80-100px visible + padding)
- Three loading phases with 2-second intervals for visual progression
- Gradient HR with `via-border` looks intentional, not like rendering artifact
- Theme-aware cursor with `/70` opacity is more subtle than solid color

## Modified Files

1. `edgequake_webui/src/components/query/query-interface.tsx` - Autoscroll fix, NonStreamingLoadingIndicator
2. `edgequake_webui/src/components/query/markdown/utils/streaming-utils.ts` - HR detection functions
3. `edgequake_webui/src/components/query/markdown/MarkdownTokens.tsx` - Gradient HR rendering
4. `edgequake_webui/src/components/query/markdown/StreamingMarkdownRenderer.tsx` - Theme-aware cursor

## Next Steps

- Manual visual testing of autoscroll behavior
- Test non-streaming mode loading indicator appearance
- Test streaming mode for HR artifacts during generation

## Lessons/Insights

- Scroll anchor needs extra height to account for fixed input area
- Multi-phase loading animations provide better perceived performance
- Theme-aware styling (using CSS variables like `bg-primary`) adapts to light/dark mode
