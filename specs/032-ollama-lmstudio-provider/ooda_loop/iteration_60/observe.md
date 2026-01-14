# OODA 60: Observe

## E2E Test Suite Verification

### Problem Discovery
E2E tests for SPEC-032 were failing due to multiple issues:

1. **Port Mismatch**: Playwright config was set to port 3000, but OrbStack was using that port, causing Next.js to fall back to port 3001
2. **Network Idle Timeout**: Using `networkidle` wait state with HMR connections that never close
3. **TenantGuard Race Condition**: Deeplink pages set workspace context, but TenantGuard's own workspace query runs separately

### Test Results Before Fix
- 6/9 tests passing (Models API + Creation tests)
- 3/9 tests failing (all Deeplink tests)

### Root Cause Analysis

**Port Issue:**
```
lsof -i :3000
COMMAND  PID  NAME
OrbStack 34264  raphaelmansuy
```
OrbStack occupies port 3000 on macOS, Next.js falls back to 3001.

**Network Idle Issue:**
HMR WebSocket connections stay open indefinitely, causing `page.waitForLoadState('networkidle')` to timeout after 30s.

**TenantGuard Race:**
```
1. Page navigates to /w/default-workspace/query
2. Deeplink page: fetches tenants → selects first
3. Deeplink page: fetches workspace by slug
4. TenantGuard: fetches workspaces for tenant
5. TenantGuard: checks if workspaces.length === 0 → shows "Create Workspace" UI
```

The race happens because TenantGuard's workspace query can complete before the deeplink page's `selectWorkspace` call updates the store.

### Verification
- `curl http://localhost:3001/w/default-workspace/query` returns valid HTML
- Backend `/api/v1/tenants/{id}/workspaces` returns 2 workspaces
- Routes appear correctly in `next build` output
