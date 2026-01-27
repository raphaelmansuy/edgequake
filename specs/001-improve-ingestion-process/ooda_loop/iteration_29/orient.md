# Iteration 29: Orient

## Gap Analysis

### Mission Requirement (UX Anti-Patterns to Avoid)

> ❌ Spinning loader with no progress indication

### Mission Requirement (UX Patterns to Implement)

> ✅ Specific stage + substage + progress percentage

### Current State vs. Required State

| Component               | Current        | Required                      |
| ----------------------- | -------------- | ----------------------------- |
| BatchProgressCard       | Silent spinner | "Loading batch status..."     |
| PipelineStatusCard      | Silent spinner | "Loading pipeline status..."  |
| QueueMetricsCard        | Silent spinner | "Loading queue metrics..."    |
| ProcessingDocumentsCard | Silent spinner | "Loading documents..."        |
| TaskQueueCard           | Silent spinner | "Loading task queue..."       |
| EmbeddingModelSelector  | Silent spinner | "Loading embedding models..." |
| LlmModelSelector        | Silent spinner | "Loading LLM models..."       |

## Design Pattern

All loading states should follow this pattern:

```tsx
// BEFORE (anti-pattern)
if (isLoading) {
  return (
    <Card>
      <CardContent className="p-6 flex items-center justify-center">
        <Loader2 className="h-6 w-6 animate-spin" />
      </CardContent>
    </Card>
  );
}

// AFTER (proper pattern)
if (isLoading) {
  return (
    <Card>
      <CardContent className="p-6 flex flex-col items-center justify-center gap-2">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
        <p className="text-sm text-muted-foreground">
          Loading pipeline status...
        </p>
      </CardContent>
    </Card>
  );
}
```

## Priority

**HIGH** - This directly addresses Objective D: Safety and Reliability by Design

Users should never see a spinning loader without understanding what's happening.
