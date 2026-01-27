# Iteration 26 – DECIDE

## Decision

Implement frontend-side clear stats passing.

## Files to Modify

1. `pipeline-status-dialog.tsx`: Add clearStats prop and ClearSummarySection
2. `rebuild-embeddings-button.tsx`: Store and pass vectorsCleared
3. `rebuild-knowledge-graph-button.tsx`: Store and pass nodes/edges/vectors cleared

## Component Design

### ClearSummarySection

```tsx
function ClearSummarySection({
  nodesCleared,
  edgesCleared,
  vectorsCleared,
}: {
  nodesCleared?: number;
  edgesCleared?: number;
  vectorsCleared?: number;
}) {
  // Only show if any stats are provided
  if (!nodesCleared && !edgesCleared && !vectorsCleared) {
    return null;
  }

  return (
    <div className="p-3 bg-green-50 dark:bg-green-950/30 rounded-lg border border-green-200 dark:border-green-800">
      <div className="flex items-center gap-2 mb-2">
        <Check className="h-4 w-4 text-green-600" />
        <span className="text-sm font-medium text-green-700 dark:text-green-400">
          Clear Phase Complete
        </span>
      </div>
      <div className="grid grid-cols-3 gap-2 text-sm">
        {nodesCleared !== undefined && (
          <div className="text-center">
            <p className="text-xs text-muted-foreground">Entities</p>
            <p className="font-bold text-green-600">
              {nodesCleared.toLocaleString()}
            </p>
          </div>
        )}
        {edgesCleared !== undefined && (
          <div className="text-center">
            <p className="text-xs text-muted-foreground">Relations</p>
            <p className="font-bold text-green-600">
              {edgesCleared.toLocaleString()}
            </p>
          </div>
        )}
        {vectorsCleared !== undefined && (
          <div className="text-center">
            <p className="text-xs text-muted-foreground">Vectors</p>
            <p className="font-bold text-green-600">
              {vectorsCleared.toLocaleString()}
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
```

## Success Criteria

- [ ] PipelineStatusDialog accepts clearStats prop
- [ ] ClearSummarySection renders when stats provided
- [ ] Numbers formatted with thousands separator
- [ ] Green styling indicates success
- [ ] RebuildEmbeddingsButton passes vectors_cleared
- [ ] RebuildKnowledgeGraphButton passes all stats
