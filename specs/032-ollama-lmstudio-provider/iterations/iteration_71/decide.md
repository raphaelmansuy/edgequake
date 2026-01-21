# OODA 71 - Decide: Add Rebuild Embeddings API Test

## Decision

Add 1 E2E test for Focus 5 (Rebuild functionality):

### Test: Rebuild embeddings API requires force flag

```typescript
test("rebuild embeddings API requires force flag for unchanged config", async ({
  request,
}) => {
  // Get existing workspace
  // POST to rebuild-embeddings without changes and force:false
  // Expect 400 error with message about config unchanged
  // POST with force:true
  // Expect 200 success
});
```

## Rationale

Testing the rebuild API validates:

1. Endpoint exists and is accessible
2. Request validation works (force flag requirement)
3. API contract is correct

This completes Focus 5 coverage:

- ✅ Settings shows rebuild option (OODA 70)
- ✅ Rebuild API works correctly (OODA 71)

## Expected Outcome

- 23 total tests after addition
- Focus 5 fully covered
