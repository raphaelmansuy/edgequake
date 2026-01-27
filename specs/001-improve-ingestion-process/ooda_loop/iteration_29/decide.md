# Iteration 29: Decide

## Decision

Add contextual loading messages to all 7 spinner locations identified:

### Changes to Make

1. **batch-progress-card.tsx:91** - Add "Loading batch status..."
2. **pipeline-monitor.tsx:292** - Add "Loading pipeline status..."
3. **pipeline-monitor.tsx:529** - Add "Loading queue metrics..."
4. **pipeline-monitor.tsx:648** - Add "Loading documents..."
5. **pipeline-monitor.tsx:752** - Add "Loading task queue..."
6. **embedding-model-selector.tsx:85** - Add "Loading embedding models..."
7. **llm-model-selector.tsx:115** - Add "Loading LLM models..."

### Implementation Pattern

Change from:

```tsx
<Loader2 className="h-6 w-6 animate-spin" />
```

To:

```tsx
<div className="flex flex-col items-center justify-center gap-2">
  <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
  <p className="text-sm text-muted-foreground">Loading X...</p>
</div>
```

### Files to Modify (4 total)

- `edgequake_webui/src/components/documents/batch-progress-card.tsx`
- `edgequake_webui/src/components/pipeline/pipeline-monitor.tsx`
- `edgequake_webui/src/components/workspace/embedding-model-selector.tsx`
- `edgequake_webui/src/components/workspace/llm-model-selector.tsx`

## Rationale

This change:

1. Eliminates the "spinning loader without context" anti-pattern
2. Tells users exactly what's being loaded
3. Reduces user anxiety during waits
4. Follows the mission's UX requirements
