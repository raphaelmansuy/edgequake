# OODA Iteration 02 - ACT

## Summary

Created the `FailedChunksCard` UI component and integrated it into the `IngestionProgressPanel` to display failed chunks during document processing.

## Changes Made

### 1. New Component: FailedChunksCard

**File**: `edgequake_webui/src/components/documents/failed-chunks-card.tsx`

Created a new component (~240 lines) that displays:

- Summary badge showing success/failure counts and percentages
- Expandable list of individual chunk failures
- Timeout vs error type indicators (with icons)
- Retry button placeholder (when `onRetry` prop provided)
- Individual retry buttons per failed chunk

Features:

- Uses shadcn/ui components (Card, Badge, Collapsible, Tooltip, Button)
- Consistent styling with existing components
- Dark mode support
- Accessibility with proper ARIA labels

### 2. Updated: IngestionProgressPanel

**File**: `edgequake_webui/src/components/documents/ingestion-progress-panel.tsx`

- Added import for `FailedChunksCard`
- Added import for `useChunkProgress` hook
- Integrated chunk progress tracking with `getProgress`, `getFailedChunks`, `hasFailedChunks`
- Conditionally render `FailedChunksCard` after cost display when failed chunks exist

```tsx
{
  /* SPEC-003: Display failed chunks if any */
}
{
  progress?.document_id && hasFailedChunks(progress.document_id) && (
    <FailedChunksCard
      documentId={progress.document_id}
      failedChunks={getFailedChunks(progress.document_id)}
      totalChunks={getProgress(progress.document_id)?.totalChunks ?? 0}
      successfulChunks={
        getProgress(progress.document_id)?.successfulChunks ?? 0
      }
      className="mt-3"
    />
  );
}
```

## UI Design

```
┌─────────────────────────────────────────────────────────────────────────┐
│  ⚠️ Partial Processing                    [8/10 OK (80%)] [2 Failed]   │
├─────────────────────────────────────────────────────────────────────────┤
│  Some chunks failed during extraction but the document was partially   │
│  processed.                                                             │
│                                                                         │
│  ▼ Show 2 failed chunks                              [Retry All]        │
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ [Chunk 3]  🕐 Timeout  2 retries                          [↻]   │   │
│  │ Request timed out after 60 seconds                              │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ [Chunk 7]  ❌ Error   3 retries                            [↻]   │   │
│  │ JSON parsing error: unexpected token at position 1234           │   │
│  └─────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

## Verification

- ✅ TypeScript compilation passed (`pnpm exec tsc --noEmit`)
- ✅ Component follows project UI patterns
- ✅ Uses existing shadcn/ui components
- ✅ Consistent with useChunkProgress hook interface

## Remaining Work

1. **Retry API Endpoint**: The retry buttons are present but need backend endpoint
2. **Database Schema**: Store failed chunks for retry functionality
3. **Prometheus Metrics**: Track chunk success/failure rates
