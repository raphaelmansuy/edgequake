# OODA 73 - Decide: Add Provider Health Check Test

## Decision

Add 1 E2E test for provider health check:

```typescript
test("provider health check returns enabled providers", async ({ request }) => {
  const response = await request.get("http://localhost:8080/api/v1/models/health");
  expect(response.ok()).toBe(true);

  const providers = await response.json();
  expect(Array.isArray(providers)).toBe(true);
  expect(providers.length).toBeGreaterThan(0);

  // Verify provider structure
  for (const provider of providers) {
    expect(provider).toHaveProperty("name");
    expect(provider).toHaveProperty("enabled");
    expect(provider).toHaveProperty("priority");
  }

  // At least one enabled provider
  const enabledProviders = providers.filter((p: any) => p.enabled);
  expect(enabledProviders.length).toBeGreaterThan(0);
});
```

## Expected Outcome

- 27 total tests after addition
- Provider health check validated
