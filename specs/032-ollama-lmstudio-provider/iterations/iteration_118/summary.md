# OODA 118: Query Lineage Display Testing

## Observe
- Chat message component has `llmProvider` and `llmModel` props for lineage tracking
- MetadataBar displays provider/model as a badge with tooltip
- Query interface captures provider/model from streaming response
- Backend returns `llm_provider` and `llm_model` in query response

## Orient
- Need E2E tests to verify lineage info appears in UI
- Need to test that different providers show correct lineage
- Need to verify tooltip shows complete info

## Decide
- Add E2E tests for:
  1. Lineage badge appears on assistant messages
  2. Provider name is displayed correctly
  3. Model name is displayed correctly
  4. Tooltip shows full lineage info

## Act
- Add `lineage-display.spec.ts` E2E tests
- Verify lineage appears after query submission
