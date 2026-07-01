# SPEC-035 — Full Stack Developer Lens

**Lens:** Full Stack Developer — Implementation  
**Key Questions:**  
- What is the technically correct architecture?  
- How do we maintain DRY and SOLID principles?  
- What does the actual code look like?  

---

## Current Implementation Audit

### Component Architecture (Code is Law)

```
edgequake_webui/src/
├── app/(dashboard)/api-explorer/page.tsx     ← 5 lines: just renders ApiExplorer
└── components/shared/api-explorer.tsx        ← 400 lines: all logic hardcoded
```

**Violations identified:**

| Violation                                                            | Location                 | Principle                     |
| -------------------------------------------------------------------- | ------------------------ | ----------------------------- |
| Hardcoded endpoint list (30 entries)                                 | `api-explorer.tsx:48–95` | DRY                           |
| No single source of truth for endpoint data                          | `api-explorer.tsx`       | DRY                           |
| Component does 5 things: list + select + request + execute + display | `api-explorer.tsx`       | SRP (Single Responsibility)   |
| Static body examples will drift from schema                          | `api-explorer.tsx:49–95` | DRY                           |
| Path parameters not handled                                          | `api-explorer.tsx`       | Correctness                   |
| Auth not integrated with auth store                                  | `api-explorer.tsx`       | Coupling (should use Zustand) |

---

## Proposed Architecture — DRY/SOLID

### Data Flow

```
Rust code
  └── #[utoipa::path] annotations
        └── ApiDoc::openapi()           [edgequake-api/src/openapi.rs]
              └── /api-docs/openapi.json [served at runtime by axum]
                    └── @scalar/api-reference React component
                          ├── Auth token  ← Zustand auth store
                          ├── Base URL    ← workspace context
                          └── Renders full interactive explorer
```

**Single source of truth:** Rust `#[utoipa::path]` annotations.  
**Zero duplication:** No endpoint list in frontend code.  
**Zero maintenance per endpoint:** Adding a handler in Rust + annotation → appears automatically in UI.

---

### SOLID Application

| Principle                     | How it applies                                                                                     |
| ----------------------------- | -------------------------------------------------------------------------------------------------- |
| **S** — Single Responsibility | `ApiExplorerPage` renders the explorer. Auth injection hook has one job. Theme config has one job. |
| **O** — Open/Closed           | New endpoints don't require frontend code changes — open for extension, closed for modification    |
| **L** — Liskov                | Not applicable (no inheritance hierarchy)                                                          |
| **I** — Interface Segregation | Separate `useApiExplorerConfig()` hook isolates auth+URL concerns                                  |
| **D** — Dependency Inversion  | Component depends on spec URL abstraction, not on concrete endpoint list                           |

---

## Implementation Plan

### Prerequisites: Research Latest `@scalar/api-reference` API

Before implementation, verify current API surface:
- Package: `@scalar/api-reference`
- Version: latest stable (`^1.x`)
- React integration: `@scalar/api-reference` exports `ApiReferenceReact` component

### Phase 1: Install Library

```bash
# In edgequake_webui/
bun add @scalar/api-reference
```

Expected impact on bundle: ~200KB (lazy-loaded, so no impact on initial page load).

### Phase 2: Create Config Hook

**New file:** `edgequake_webui/src/hooks/use-api-explorer-config.ts`

```typescript
/**
 * @module useApiExplorerConfig
 * @description Computes the Scalar API Reference configuration from app context.
 *
 * @implements FEAT-035-01 - Auth token injection
 * @implements FEAT-035-02 - Workspace base URL injection
 * @enforces DRY - single source for explorer config
 * @enforces SRP - only responsible for computing explorer config
 */
import { useAuthStore } from '@/stores/auth-store';
import { useWorkspaceContext } from '@/hooks/use-workspace-context';
import { useMemo } from 'react';

export interface ApiExplorerConfig {
  specUrl: string;
  baseServerUrl: string;
  bearerToken: string | null;
}

export function useApiExplorerConfig(): ApiExplorerConfig {
  const { token } = useAuthStore();
  const { apiBaseUrl } = useWorkspaceContext();

  return useMemo(() => ({
    specUrl: `${apiBaseUrl}/api-docs/openapi.json`,
    baseServerUrl: apiBaseUrl,
    bearerToken: token ?? null,
  }), [token, apiBaseUrl]);
}
```

### Phase 3: Create Theme Configuration

**New file:** `edgequake_webui/src/lib/api-explorer-theme.ts`

```typescript
/**
 * @module apiExplorerTheme
 * @description Maps EdgeQuake design tokens to Scalar API Reference CSS variables.
 *
 * @enforces DRY - one place to update if design tokens change
 * @enforces SRP - only handles theme mapping
 */

export const SCALAR_DARK_THEME = `
  :root {
    --scalar-background-1: hsl(222.2 84% 4.9%);
    --scalar-background-2: hsl(217.2 32.6% 17.5%);
    --scalar-background-3: hsl(215 27.9% 16.9%);
    --scalar-background-accent: hsl(217.2 91.2% 59.8% / 0.1);
    
    --scalar-color-1: hsl(210 40% 98%);
    --scalar-color-2: hsl(214.3 31.8% 91.4%);
    --scalar-color-3: hsl(215 20.2% 65.1%);
    
    --scalar-color-accent: hsl(217.2 91.2% 59.8%);
    
    --scalar-border-color: hsl(217.2 32.6% 17.5%);
    
    --scalar-color-green: hsl(142 71% 45%);
    --scalar-color-red: hsl(0 84% 60%);
    --scalar-color-yellow: hsl(47.9 95.8% 53.1%);
    --scalar-color-blue: hsl(217.2 91.2% 59.8%);
    --scalar-color-orange: hsl(24.6 95% 53.1%);
    
    --scalar-sidebar-background-1: hsl(222.2 84% 4.9%);
    --scalar-sidebar-color-1: hsl(210 40% 98%);
    --scalar-sidebar-color-active: hsl(217.2 91.2% 59.8%);
  }
`;
```

### Phase 4: Implement the New Page Component

**Modified file:** `edgequake_webui/src/app/(dashboard)/api-explorer/page.tsx`

```tsx
/**
 * @module ApiExplorerPage
 * @description API Explorer page — renders Scalar API Reference from live spec.
 *
 * @implements UC0901 - Developer tests API endpoints
 * @implements FEAT0639 - Interactive API testing
 * @implements FEAT0640 - Request/response visualization
 * @implements FEAT-035 - OpenAPI-native API Explorer
 *
 * @enforces DRY - no hardcoded endpoint list; spec is the source of truth
 * @enforces SRP - page renders explorer; config is in useApiExplorerConfig
 * @enforces OCP - new endpoints appear automatically
 */
'use client';

import dynamic from 'next/dynamic';
import { useApiExplorerConfig } from '@/hooks/use-api-explorer-config';
import { SCALAR_DARK_THEME } from '@/lib/api-explorer-theme';

// Lazy load to avoid SSR issues and reduce initial bundle
const ApiReferenceReact = dynamic(
  () => import('@scalar/api-reference').then(m => ({ default: m.ApiReferenceReact })),
  {
    ssr: false,
    loading: () => (
      <div className="flex h-full items-center justify-center">
        <div className="text-muted-foreground text-sm">Loading API Explorer…</div>
      </div>
    ),
  }
);

export default function ApiExplorerPage() {
  const { specUrl, bearerToken } = useApiExplorerConfig();

  return (
    <div className="h-full w-full overflow-hidden">
      <ApiReferenceReact
        configuration={{
          spec: { url: specUrl },
          theme: 'none',           // disable default theme; use our CSS
          customCss: SCALAR_DARK_THEME,
          authentication: bearerToken
            ? {
                preferredSecurityScheme: 'bearerAuth',
                http: { bearer: { token: bearerToken } },
              }
            : undefined,
          layout: 'sidebar',
          hideDownloadButton: false,
          hideTestRequestButton: false,
          defaultHttpClient: {
            targetKey: 'javascript',
            clientKey: 'fetch',
          },
        }}
      />
    </div>
  );
}
```

### Phase 5: Remove Old Component

After the new implementation is verified:

```bash
# Remove the legacy component
rm edgequake_webui/src/components/shared/api-explorer.tsx
```

Update any remaining imports (currently only `api-explorer/page.tsx` imports it).

---

## Auth Store Integration

Check existing auth store structure:

```typescript
// Expected interface in stores/auth-store (verify against actual file)
interface AuthStore {
  token: string | null;
  user: User | null;
  setToken: (token: string) => void;
  clearAuth: () => void;
}
```

The `useApiExplorerConfig` hook depends on `useAuthStore()`. If the auth store exposes `token` under a different key, update the hook accordingly.

**Action:** Check `edgequake_webui/src/stores/` for the actual auth store shape before implementing.

---

## Workspace Context Integration

```typescript
// Expected interface in hooks/use-workspace-context (verify against actual file)
interface WorkspaceContext {
  apiBaseUrl: string;    // e.g., "http://localhost:8080"
  workspaceId: string;
  tenantId: string;
}
```

**Action:** Check `edgequake_webui/src/hooks/` for the workspace context hook.

---

## Backend CORS Verification

The Scalar component makes fetch requests directly from the browser to `http://localhost:8080`.  
The backend already has CORS enabled (see `server.rs:L96–L120`). Verify for production:

```rust
// server.rs — CORS already configured
CorsLayer::new()
    .allow_origin(Any)    // ← In dev
    .allow_methods(Any)
    .allow_headers(Any)
```

For production, `EDGEQUAKE_CORS_ORIGINS` should include the frontend origin.

---

## Testing Requirements

### Unit Tests

**New file:** `edgequake_webui/src/hooks/__tests__/use-api-explorer-config.test.ts`

```typescript
import { renderHook } from '@testing-library/react';
import { useApiExplorerConfig } from '../use-api-explorer-config';

describe('useApiExplorerConfig', () => {
  it('returns spec URL based on apiBaseUrl', () => {
    // Mock auth store and workspace context
    // Assert specUrl = "${apiBaseUrl}/api-docs/openapi.json"
  });

  it('includes bearer token when authenticated', () => {
    // Mock auth store with token
    // Assert bearerToken is set
  });

  it('returns null bearerToken when not authenticated', () => {
    // Mock auth store with no token
    // Assert bearerToken is null
  });

  it('spec URL changes when workspace context changes', () => {
    // Assert memoization works correctly
  });
});
```

### E2E Tests

**New file:** `edgequake_webui/e2e/api-explorer.spec.ts`

```typescript
import { test, expect } from '@playwright/test';

test.describe('API Explorer', () => {
  test('loads without errors', async ({ page }) => {
    await page.goto('/api-explorer');
    // Should not show blank page or loading spinner indefinitely
    await expect(page.locator('text=EdgeQuake API')).toBeVisible({ timeout: 10000 });
  });

  test('shows health endpoint', async ({ page }) => {
    await page.goto('/api-explorer');
    await expect(page.locator('text=/health')).toBeVisible({ timeout: 10000 });
  });

  test('shows more than 30 endpoints', async ({ page }) => {
    await page.goto('/api-explorer');
    // Count visible endpoint items — must be > 30
    const endpoints = await page.locator('[data-scalar-type="operation"]').count();
    expect(endpoints).toBeGreaterThan(30);
  });

  test('auth token is pre-populated when logged in', async ({ page }) => {
    // Login first, then navigate to explorer
    await page.goto('/api-explorer');
    // Verify auth field is not empty
  });
});
```

---

## Migration Checklist

```
- [ ] Install @scalar/api-reference
- [ ] Verify @scalar/api-reference API surface (check current docs)
- [ ] Check auth store shape in stores/
- [ ] Check workspace context hook in hooks/
- [ ] Create src/hooks/use-api-explorer-config.ts
- [ ] Create src/lib/api-explorer-theme.ts
- [ ] Update src/app/(dashboard)/api-explorer/page.tsx
- [ ] Verify CORS config allows explorer origin
- [ ] Run bun run build (no type errors)
- [ ] Manual test: /health endpoint returns 200
- [ ] Manual test: auth token is pre-populated
- [ ] Manual test: dark mode matches app
- [ ] Manual test: path parameters work
- [ ] Write unit tests for useApiExplorerConfig
- [ ] Write E2E test
- [ ] Remove old api-explorer.tsx component
- [ ] Update CHANGELOG.md
```

---

## DRY Verification

After implementation, the word "endpoint" should NOT appear in any frontend file as a hardcoded API path. The only reference to the API structure should be:

```typescript
const specUrl = `${apiBaseUrl}/api-docs/openapi.json`;
//               ↑ This line is the entire frontend knowledge of the API structure
```

**Before:** 30 hardcoded endpoint entries × avg 4 lines = 120 lines of duplicated API knowledge  
**After:** 1 URL pointing to the spec = 0 duplication  
**DRY ratio improvement:** 120× reduction in duplicated API knowledge
