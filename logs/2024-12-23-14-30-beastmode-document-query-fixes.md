# Task Log: Document & Query Fixes

**Date:** 2024-12-23 14:30  
**Mode:** Beastmode  
**Commit:** a1ba0c6

## Actions

1. **Backend - DocumentSummary Enhancement**

   - Added `file_name`, `status`, `created_at`, `updated_at`, `entity_count` fields to `DocumentSummary` struct
   - Updated `list_documents` handler to parse complete metadata from KV storage
   - Fixed related unit tests

2. **Frontend - Toast Duration Fix**

   - Added `duration={3000}` and `closeButton` props to global Toaster
   - Added explicit `duration: 3000` (success) and `duration: 5000` (error/warning) to upload toasts

3. **Frontend - Markdown Renderer Error Fix**

   - Added content-based key to MarkdownErrorBoundary for proper reconciliation
   - Key format: `md-{length}-{prefix}` to help React reset error state on content change

4. **Frontend - Query Regenerate Race Condition Fix**

   - Fixed `handleRegenerate` to use `setTimeout(..., 0)` to defer `handleStreamQuery`
   - Ensures state update completes before new message is added

5. **Frontend - Upload UX Enhancement**
   - Added overall progress header with file count (X/Y files complete)
   - Added phase legend showing: Reading → Uploading → Extracting → Done
   - Phase legend with color-coded dots (amber, blue, purple, green)

## Decisions

- Used existing phase tracking system rather than creating new component
- Used content-based key for error boundary instead of random key
- Chose setTimeout(0) for state synchronization instead of useEffect

## Next Steps

- Start backend server to test document title display
- Test upload flow with multiple files
- Verify toast auto-dismiss behavior
- Test query regenerate without errors

## Lessons/Insights

- Zustand store setters don't support functional updates like React useState
- react-markdown v10+ may throw errors during reconciliation that bypass error boundaries
- Content-based keys help React properly reset error state without unnecessary remounts
