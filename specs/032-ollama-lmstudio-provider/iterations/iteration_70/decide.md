# OODA 70 - Decide: Add Workspace Settings Tests

## Decision

Add 2 E2E tests for Focus 4 (Workspace Settings):

### Test 1: Workspace settings deeplink loads
```typescript
test("workspace settings deeplink loads", async ({ page, request }) => {
  // Get workspace slug from API
  // Navigate to /w/{slug}/settings
  // Verify settings page renders
});
```

### Test 2: Settings displays workspace model configuration
```typescript
test("settings displays workspace model configuration", async ({ page }) => {
  // Navigate to /settings
  // Look for model configuration section
  // Verify LLM/embedding labels are visible
});
```

## Implementation Location

Add to existing Focus 6 (Deeplink Routes) section or create new Focus 4 section.

Since Focus 6 already tests deeplinks, I'll add the settings deeplink there and create a new Focus 4 section for workspace configuration display.

## Expected Outcome

- 21 total tests after addition
- Coverage for Focus 4 workspace settings
