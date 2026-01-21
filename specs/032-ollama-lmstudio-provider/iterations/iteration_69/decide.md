# OODA 69 - Decide: Add Provider Selector UI Tests

## Decision

Add two E2E tests for Focus 3 (Query interface provider selection):

### Test 1: Query page has provider model selector

```typescript
test("query page has provider model selector", async ({ page }) => {
  // Navigate to query page
  // Look for provider selector button/trigger
  // Verify it's visible with accessible attributes
});
```

### Test 2: Provider selector shows available providers

```typescript
test("provider selector shows available providers", async ({ page }) => {
  // Navigate to query page
  // Click to open provider selector
  // Verify providers from API appear in dropdown
  // Verify at least OpenAI/Ollama are listed
});
```

## Implementation Notes

1. Use `data-testid` or role-based locators for reliability
2. Handle HMR connections by using `domcontentloaded` wait
3. Test against `/query` main route (not deeplink) for simplicity
4. Compare dropdown content against known providers from API

## Expected Outcome

- Coverage for Focus 3 provider selection UI
- Validates ProviderModelSelector integration
- 19 total tests after addition
