# Issue #180 — Runtime Config and Authentication Hardening

## Summary

Issue #180 exposed three independent but related production risks:

1. The WebUI depended on `NEXT_PUBLIC_API_URL`, which is baked into the browser bundle at build time in Next.js.
2. Protected dashboard routes could still be opened directly when demo login was disabled.
3. Backend authentication flags and bootstrap API keys were not enforced consistently enough for hardened deployments.

## Reproduction Evidence

### 1. Runtime API URL bug

A prebuilt frontend image continued to use the API base URL from build time. Changing only the container runtime environment did not update the browser bundle.

### 2. Frontend auth bypass

Direct navigation to dashboard pages such as `/documents` and `/graph` succeeded even when the login screen no longer exposed the demo entry point.

### 3. Backend security gap

Unauthenticated or weakly-protected flows could still reach sensitive creation and API-key paths in hardened deployments.

## First-Principles Root Cause Analysis

### Root cause A — Build-time vs runtime configuration

`NEXT_PUBLIC_*` variables are statically inlined by Next.js during the build for client bundles. That means a Docker image compiled against one API URL cannot safely be reused across environments unless the runtime values are injected server-side.

### Root cause B — Missing route-level guard

The dashboard shell enforced workspace selection, but not authenticated session presence. Hiding a login button is not equivalent to access control.

### Root cause C — Split auth logic

The backend already had authentication primitives, but the runtime environment parsing, middleware enforcement, and some endpoint handlers were not using one consistently enforced source of truth.

### Root cause D — Ad-hoc frontend fetches

A handful of settings/provider screens were bypassing the shared API client and therefore skipped the centralized auth-header logic.

## Design Goals

- Keep one shared runtime-config path for the WebUI.
- Keep one shared authenticated API client for frontend requests.
- Keep one shared source of truth for backend auth flags and bootstrap keys.
- Fail closed when auth is enabled.
- Preserve the existing developer/demo mode when auth is intentionally disabled.

## Implemented Fix

### Frontend

- Added a shared runtime-config helper that reads server-injected values.
- Injected runtime config from the root layout so prebuilt images remain portable across environments.
- Added an `AuthGuard` to the dashboard layout.
- Hid the demo path whenever authentication is required.
- Routed settings/provider fetches through the shared authenticated API client.

### Backend

- Extended `AuthConfig::from_env()` to load:
  - `EDGEQUAKE_AUTH_ENABLED`
  - `EDGEQUAKE_MASTER_API_KEY`
  - static API key lists
  - registration toggle
- Added request-auth helpers for consistent handler enforcement.
- Protected API-key and user-management endpoints.
- Prevented public self-registration from assigning elevated roles.
- Applied fail-closed middleware to the versioned API when auth is enabled.

## Validation

### Automated

- Rust auth E2E regression suite passes: `28 passed, 0 failed`.

### Browser E2E

Verified manually with the live app:

- **Without authentication enabled**: Dashboard, Graph, Documents, Pipeline, Query, Workspace, Costs, Knowledge, API Explorer, and Settings all render successfully.
- **With authentication enabled**:
  - direct navigation to protected screens redirects to `/login`
  - the demo login shortcut is hidden
  - authenticated access restores all main screens successfully

## DRY / SOLID Notes

- Runtime config is centralized in one helper instead of repeated environment lookups.
- Route protection is handled by a dedicated guard instead of page-by-page duplication.
- Auth checks are shared by helper functions and middleware instead of copied endpoint logic.
- The frontend now prefers the common API client over ad-hoc `fetch()` calls.

## Operational Recommendation

For production deployments, set:

- `EDGEQUAKE_AUTH_ENABLED=true`
- `NEXT_PUBLIC_AUTH_ENABLED=true`
- `NEXT_PUBLIC_DISABLE_DEMO_LOGIN=true`
- `EDGEQUAKE_MASTER_API_KEY=<strong bootstrap key>`

This produces a fail-closed deployment that still supports secure first-time admin bootstrap.
