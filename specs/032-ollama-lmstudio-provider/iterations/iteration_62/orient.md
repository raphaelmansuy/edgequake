# OODA 62 - Orient: Simplify E2E Tests

## Analysis

### Current Test Complexity

The deeplink test `workspace deeplink by slug resolves correctly` has defensive logic:

1. Waits for multiple possible states (query input, error, create workspace)
2. Falls back to breadcrumb check if TenantGuard shows "Create Workspace"
3. Uses 2-second hard timeout before checking elements

### Why This Was Needed (Pre-OODA 61)

TenantGuard would:

1. Query workspaces independently of deeplink page
2. Complete before deeplink page set workspace context
3. Show "Create Workspace" because workspaces array was empty

### Why It's No Longer Needed (Post-OODA 61)

OODA 61 fixed the race condition by:

1. Removing TenantGuard from deeplink layout
2. Having deeplink pages handle their own loading/error states
3. Adding workspace list fetching to populate store

## Options

### Option 1: Remove Fallback Logic Entirely

- **Pro**: Cleaner test code
- **Pro**: Tests actual expected behavior
- **Con**: May be less resilient to future changes

### Option 2: Simplify But Keep Robust Assertions

- **Pro**: Still catches regressions
- **Pro**: Documents expected behavior
- **Con**: More verbose than needed

### Option 3: Convert to Simpler Wait Pattern

- **Pro**: Uses Playwright best practices
- **Pro**: Reduces flakiness
- **Con**: May need adjustment if timing changes

## Recommendation

**Option 3: Convert to Simpler Wait Pattern**

Replace complex Promise.race with:

```typescript
// Navigate to deeplink
await page.goto(`/w/${workspaceSlug}/query`, { waitUntil: "domcontentloaded" });

// Wait for query interface to appear
await expect(page.locator('textarea[placeholder*="Ask"]')).toBeVisible({
  timeout: 30000,
});
```

This is simpler and tests the actual expected behavior now that the race condition is fixed.
