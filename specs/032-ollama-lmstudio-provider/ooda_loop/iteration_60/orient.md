# OODA 60: Orient

## Technical Analysis

### Port Configuration
The Playwright config defaults to port 3000, but macOS environments often have services using this port:
- OrbStack uses 3000 for its web interface
- Some local proxies use 3000
- Docker Desktop may also use 3000

Next.js automatically selects the next available port (3001) when 3000 is unavailable.

### Solution Approaches

**Option A: Force Port 3001**
- Pros: Consistent, works with OrbStack
- Cons: May conflict with other services on 3001

**Option B: Dynamic Port Detection**
- Pros: Always finds correct port
- Cons: More complex, slower startup

**Option C: Environment Variable Override**
- Pros: Flexible, CI-friendly
- Cons: Requires explicit configuration

**Chosen: Option A + C** - Default to 3001 with `PLAYWRIGHT_BASE_URL` override

### Wait State Strategy

HMR connections keep network busy. Options:
1. Use `domcontentloaded` instead of `networkidle`
2. Add explicit timeouts
3. Wait for specific elements

**Chosen**: Use `domcontentloaded` + element-specific waits

### TenantGuard Integration

The deeplink pages need to work alongside TenantGuard without conflicts:
1. Accept "Create Workspace" UI as valid state (route resolved correctly)
2. Check breadcrumb for workspace slug as proof of route resolution
3. Increase timeout for complex async flows

## Implementation Plan
1. Update `playwright.config.ts` with port 3001 and environment override
2. Update E2E tests to use `domcontentloaded` wait state
3. Add robust assertions that handle TenantGuard timing
4. Increase global test timeout to 60s
