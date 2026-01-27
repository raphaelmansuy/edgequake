# Act - Iteration 11: Stage Progress Tooltip

## Actions Taken

### 1. Added Processing Stage Configuration

Added `PROCESSING_STAGES` constant with:

- Stage keys (chunking, extracting, embedding, indexing)
- Stage labels for display
- Stage descriptions for tooltip

### 2. Added Stage Progress Helper

`getStageProgress(status)` function returns:

- Current stage number (1-4)
- Total stages (4)
- Stage description

### 3. Enhanced StatusBadge with Rich Tooltip

For processing states, badge now shows tooltip with:

- Current stage label and step counter (e.g., "Step 2/4")
- Stage description (e.g., "Running LLM entity extraction")
- Visual progress bar (4 segments)
- Stage names under progress bar
- Current stage highlighted

### 4. Added Props

- `disableTooltip`: Option to disable tooltip (for nested tooltips)
- Maintained backward compatibility with existing props

## Files Changed

- `edgequake_webui/src/components/documents/status-badge.tsx`
  - Added Tooltip imports
  - Added PROCESSING_STAGES config
  - Added getStageProgress helper
  - Added useMemo import
  - Enhanced StatusBadge with TooltipProvider

## Verification

- ✅ TypeScript compilation passes

## User Experience

Before: Status badge only shows current stage name
After: Hover reveals:

```
┌─────────────────────────────────┐
│  Extracting         Step 2/4   │
│  Running LLM entity extraction │
│  ████ ████ ░░░░ ░░░░           │
│  Chunk Extract Embed Index     │
└─────────────────────────────────┘
```

## Next Steps

- Continue with Iteration 12
- Focus on bulk operation improvements
