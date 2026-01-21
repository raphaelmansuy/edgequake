# OODA 72 - Decide: Add API Error Handling Tests

## Decision

Add new test section for API error handling:

### Test 1: Invalid tenant ID returns 404

```typescript
test("invalid tenant ID returns 404", async ({ request }) => {
  const response = await request.get(
    "http://localhost:8080/api/v1/tenants/00000000-0000-0000-0000-000000000000"
  );
  expect(response.status()).toBe(404);
});
```

### Test 2: Invalid workspace ID returns 404

```typescript
test("invalid workspace ID returns 404", async ({ request }) => {
  // Get valid tenant first
  // Then request invalid workspace
  expect(response.status()).toBe(404);
});
```

## Expected Outcome

- 26 total tests after addition
- Error handling paths validated
- More robust API contract verification
