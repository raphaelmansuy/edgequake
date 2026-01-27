# OODA 74 - Orient & Decide: Tenant CRUD Tests

## Strategy

Add 2 tests for tenant CRUD operations:

1. "list tenants returns paginated results"
2. "tenant CRUD lifecycle works correctly"

## Implementation

```typescript
test("list tenants returns paginated results", async ({ request }) => {
  const response = await request.get("http://localhost:8080/api/v1/tenants");
  expect(response.ok()).toBe(true);

  const data = await response.json();
  expect(data).toHaveProperty("items");
  expect(data).toHaveProperty("total");
  expect(Array.isArray(data.items)).toBe(true);
});
```
