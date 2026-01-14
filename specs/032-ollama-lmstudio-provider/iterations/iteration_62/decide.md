# OODA 62 - Decide: Simplify E2E Tests

## Decision

Simplified the deeplink E2E tests by:
1. Removing complex Promise.race patterns
2. Using direct `expect().toBeVisible()` assertions
3. Removing fallback logic for TenantGuard race condition (fixed in OODA 61)

## Rationale

### Before (OODA 60)
```typescript
// Complex Promise.race to handle multiple possible states
const foundElement = await Promise.race([
  queryTextarea.first().waitFor({ state: 'visible', timeout: 30000 }).then(() => 'query'),
  errorState.waitFor({ state: 'visible', timeout: 30000 }).then(() => 'error'),
  createWorkspaceButton.waitFor({ state: 'visible', timeout: 30000 }).then(() => 'create'),
]).catch(() => 'none');

// Fallback logic for TenantGuard timing issue
if (foundElement === 'create') {
  const breadcrumb = page.locator(`[href*="${workspaceSlug}"]`);
  const hasBreadcrumb = await breadcrumb.count() > 0;
  expect(hasBreadcrumb).toBe(true);
  return;
}
```

### After (OODA 62)
```typescript
// Simple direct assertion - TenantGuard race condition fixed in OODA 61
const queryTextarea = page.locator('textarea[placeholder*="Ask"], input[placeholder*="Ask"], [aria-label*="query"]');
await expect(queryTextarea.first()).toBeVisible({ timeout: 30000 });
```

## Benefits

1. **Code Clarity**: Tests now document expected behavior, not workarounds
2. **Faster Debugging**: Simpler assertions mean clearer failure messages
3. **Regression Detection**: Will catch any regressions in deeplink behavior
4. **Performance**: Removed 2-second `waitForTimeout` delays

## Test Execution

- All 9 tests pass consistently
- Run time reduced from ~11s to ~3.3s
