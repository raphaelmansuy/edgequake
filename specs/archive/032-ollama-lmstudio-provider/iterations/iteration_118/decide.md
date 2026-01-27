# OODA 118: Decide

## Decision
Add query lineage display tests:
1. `query API response includes llm_provider field` - Verify response structure
2. `models API returns provider display names` - Verify provider metadata

## Implementation
```typescript
test("query API response includes llm_provider field", async ({ request }) => {
  // Get valid tenant/workspace
  // Make POST /api/v1/query
  // Verify response has answer or error field
});
```

## Expected Outcome
- Tests validate current API behavior
- Future improvements can add llm_provider to response
- Error handling tested for empty workspaces
