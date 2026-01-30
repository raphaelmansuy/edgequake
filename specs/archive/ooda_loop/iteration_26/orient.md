# Iteration 26 – ORIENT

## Approach

Frontend-side state passing for clear stats.

## Implementation Plan

### 1. Extend PipelineStatusDialog Props

Add optional props for clear phase statistics:

```tsx
interface PipelineStatusDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title?: string;
  subtitle?: string;
  // OODA-26: Clear phase statistics
  clearStats?: {
    nodesCleared?: number;
    edgesCleared?: number;
    vectorsCleared?: number;
  };
}
```

### 2. Add Clear Summary Section

Display a summary card when clearStats are provided:

```
┌──────────────────────────────────────┐
│ ✓ Clear Phase Complete              │
│                                      │
│ Entities: 1,234  Relationships: 3,456│
│ Vectors: 45,678                      │
└──────────────────────────────────────┘
```

### 3. Update RebuildEmbeddingsButton

Store rebuild response and pass clear stats:

```tsx
const [clearStats, setClearStats] = useState<ClearStats | null>(null);

// In rebuildMutation.onSuccess:
setClearStats({
  vectorsCleared: response.vectors_cleared,
});

// Pass to dialog:
<PipelineStatusDialog
  open={isPipelineOpen}
  onOpenChange={setIsPipelineOpen}
  clearStats={clearStats}
/>;
```

### 4. Update RebuildKnowledgeGraphButton

Similar pattern for KG rebuild with nodes/edges stats.

## Design Decisions

1. **Optional Props**: Clear stats are optional, dialog works without them
2. **Conditional Rendering**: Only show clear summary when stats provided
3. **Formatted Numbers**: Use toLocaleString() for readability
4. **Visual Distinction**: Green background for "cleared" summary
