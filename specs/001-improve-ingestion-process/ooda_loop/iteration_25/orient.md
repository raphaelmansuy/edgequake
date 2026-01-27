# Iteration 25 – ORIENT

## Strategic Assessment

The frontend can detect rebuild operations by parsing job_name:

- `rebuild_kg_*` → Knowledge Graph rebuild (3 phases)
- `rebuild_embed_*` → Embeddings rebuild (2 phases)

We can estimate current phase based on document progress:

- Phase 1 (Clear): When processed_documents == 0 and is_busy
- Phase 2 (Re-extract): When processed_documents > 0 and < total_documents
- Phase 3 (Complete): When processed_documents == total_documents

## Approach

Add a **RebuildPhaseIndicator** component that:

1. Detects rebuild type from job_name
2. Shows 3-phase stepper for KG rebuild
3. Shows 2-phase stepper for embed rebuild
4. Highlights current phase
5. Shows summary when complete

## Component Design

```tsx
┌────────────────────────────────────────────────────────────┐
│ REBUILDING KNOWLEDGE GRAPH                                 │
│                                                            │
│ ○───────●───────○                                         │
│ Clear   Extract  Embed                                     │
│ ✓       Active   Pending                                   │
│                                                            │
│ Phase 2: Re-extracting                                    │
│ Documents: 8/25 (32%) | ETA: ~10m                         │
└────────────────────────────────────────────────────────────┘
```

## Integration Points

1. PipelineStatusDialog: Add RebuildPhaseIndicator before progress bar
2. Conditional rendering: Only show for rebuild operations
3. Phase detection: Parse job_name + use progress ratios

## Risks

- Phase inference is heuristic (no explicit backend state)
- Clear phase is instant, may not be visible
- Solution: Show "Clear ✓" immediately when rebuild starts
