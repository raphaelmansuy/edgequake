# Iteration 29: Observe

## Mission Reference

Re-read mission spec: `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-improve-ingestion-process.md`

## Objective Focus

**Objective D: Safety and Reliability by Design** - Loading State Clarity

## Observations

### UX Anti-Pattern Found

From the mission spec:

> ❌ Spinning loader with no progress indication

### Loading Spinners WITHOUT Context (Anti-Pattern)

Found 6 loading spinners that just show a spinner without any context:

| File                           | Line | Current Pattern        | Issue                             |
| ------------------------------ | ---- | ---------------------- | --------------------------------- |
| `batch-progress-card.tsx`      | 91   | `<Loader2 ... />` only | No text explaining what's loading |
| `pipeline-monitor.tsx`         | 292  | `<Loader2 ... />` only | No text explaining what's loading |
| `pipeline-monitor.tsx`         | 529  | `<Loader2 ... />` only | No text explaining what's loading |
| `pipeline-monitor.tsx`         | 648  | `<Loader2 ... />` only | No text explaining what's loading |
| `pipeline-monitor.tsx`         | 752  | `<Loader2 ... />` only | No text explaining what's loading |
| `embedding-model-selector.tsx` | 85   | `<Loader2 ... />` only | No text explaining what's loading |
| `llm-model-selector.tsx`       | 115  | `<Loader2 ... />` only | No text explaining what's loading |

### Loading Spinners WITH Context (Good Pattern)

Found good examples:

| File                           | Line | Pattern                                    |
| ------------------------------ | ---- | ------------------------------------------ |
| `ingestion-progress-panel.tsx` | 145  | `<RefreshCw .../>` + "Loading progress..." |

### Component-Specific Context Needed

Each component should provide meaningful context:

1. **batch-progress-card.tsx** - "Loading batch status..."
2. **pipeline-monitor.tsx (PipelineStatusCard)** - "Loading pipeline status..."
3. **pipeline-monitor.tsx (QueueMetricsCard)** - "Loading queue metrics..."
4. **pipeline-monitor.tsx (ProcessingDocumentsCard)** - "Loading documents..."
5. **pipeline-monitor.tsx (TaskQueueCard)** - "Loading task queue..."
6. **embedding-model-selector.tsx** - "Loading embedding models..."
7. **llm-model-selector.tsx** - "Loading LLM models..."

## Files to Modify

1. `edgequake_webui/src/components/documents/batch-progress-card.tsx`
2. `edgequake_webui/src/components/pipeline/pipeline-monitor.tsx`
3. `edgequake_webui/src/components/workspace/embedding-model-selector.tsx`
4. `edgequake_webui/src/components/workspace/llm-model-selector.tsx`
