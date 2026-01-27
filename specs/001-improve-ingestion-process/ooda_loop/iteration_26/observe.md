# Iteration 26 – OBSERVE

## Mission Context

**Objective C**: Rebuild Operations Visibility

- Requirement: Show clear phase statistics
- Spec shows: "Cleared: 1,234 entities | 3,456 relationships"

## Current State

### Data Available

**Backend RebuildKnowledgeGraphResponse**:

- `nodes_cleared`: Number of entities cleared
- `edges_cleared`: Number of relationships cleared
- `vectors_cleared`: Number of vectors cleared

**Backend RebuildEmbeddingsResponse**:

- `vectors_cleared`: Number of vectors cleared

These are returned in the initial API response when rebuild is triggered.

### Frontend State

**RebuildEmbeddingsButton**:

- Receives rebuild response with clear stats
- Opens PipelineStatusDialog to show progress
- But doesn't pass clear stats to the dialog

**PipelineStatusDialog**:

- Polls `getEnhancedPipelineStatus` for progress
- This endpoint doesn't include clear stats
- No way to display what was cleared

### Gap Analysis

The clear stats exist but are lost:

1. Rebuild API returns them
2. Button shows a toast with info
3. Dialog opens but has no access to these stats
4. User loses context about what was cleared

## Observation

The simplest fix is frontend-side state passing:

1. Store clear stats when rebuild response arrives
2. Pass them as props to PipelineStatusDialog
3. Display them in a "Clear Summary" section

This avoids backend changes and keeps the solution simple.
