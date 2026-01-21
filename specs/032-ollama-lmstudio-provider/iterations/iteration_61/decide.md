# OODA 61 - Decide: Remove TenantGuard from Deeplink Routes

## Decision

**Selected Option: Option 4 - Bypass TenantGuard in Deeplink Routes**

## Rationale

1. **Architectural Clarity**: Deeplink routes are self-contained - they resolve their own workspace context from the URL slug
2. **No Race Condition**: By removing TenantGuard, we eliminate the race between TenantGuard's workspace query and the page's workspace resolution
3. **Proper Loading States**: Each deeplink page already handles loading/error states explicitly
4. **No Redundancy**: TenantGuard was providing no additional value since pages already validate workspace existence

## Implementation Scope

### Files Modified

1. **`/w/[slug]/layout.tsx`**

   - Remove TenantGuard import
   - Render children directly without wrapper
   - Add documentation explaining architectural decision

2. **`/w/[slug]/query/page.tsx`**

   - Remove TenantGuard import
   - Add `getWorkspaces` import
   - Fetch and populate workspace list in store (prevents any downstream TenantGuard issues)
   - Return `<QueryInterface />` directly

3. **`/w/[slug]/settings/page.tsx`**
   - Same pattern as query page
   - Add workspace list fetching
   - Remove TenantGuard wrapper

## Expected Outcomes

- ✅ E2E tests pass without needing breadcrumb fallback assertions
- ✅ No "Create Workspace" flash on deeplink navigation
- ✅ Proper loading → content transition
- ✅ 404 handling for invalid slugs

## Risk Assessment

- **Risk**: Other components relying on TenantGuard for workspace context
- **Mitigation**: Deeplink pages now explicitly call `setWorkspaces()` to populate the store
- **Testing**: E2E tests will validate behavior
