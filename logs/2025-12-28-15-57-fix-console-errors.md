# Task Log: Fix Console Errors

**Date:** 2025-12-28 15:57
**Mode:** Beastmode

## Actions

- Fixed empty image `src` attribute in `MarkdownInlineTokens.tsx` by adding null/empty check for `imgToken.href` before rendering `<img>` element
- Changed WebSocket error logging in `progress-websocket.ts` from `console.error` to `console.warn` with development mode check to reduce noise
- Verified TypeScript compilation passes with no errors

## Decisions

- Used a placeholder `<span>` with grey background and icon emoji (🖼️) for images with empty/missing href
- Made WebSocket connection errors less noisy since they're expected when backend isn't running during development
- The ContentRenderer fix is covered by the MarkdownInlineTokens fix since it's the same underlying component

## Files Changed

1. `edgequake_webui/src/components/markdown/MarkdownInlineTokens.tsx`
   - Added check: `if (!imgToken.href)` to return placeholder instead of empty `<img src="">`
2. `edgequake_webui/src/lib/websocket/progress-websocket.ts`
   - Changed `handleError()` to use `console.warn` with development-only logging

## Next Steps

- Refresh browser and verify no more console errors appear
- Test document upload with progress tracking
- Run E2E tests to validate fixes

## Lessons/Insights

- Empty string in img src causes browser warning - always validate before rendering
- WebSocket errors when backend is down are expected during development - use warn level not error
