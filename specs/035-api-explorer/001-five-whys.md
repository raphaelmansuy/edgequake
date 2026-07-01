# SPEC-035 — 5 WHYs: Root Cause Analysis of the API Explorer Problem

**Lens:** Root Cause Analysis  
**Method:** 5 WHYs — iterative causal chain to structural root  
**Evidence:** All claims verified against live source code  

---

## The Symptom

A developer opens the EdgeQuake API Explorer at `http://localhost:3000/api-explorer`.  
They see 30 endpoints. The real API has 169.  
They click on `GET /documents/{id}` — there is no path parameter input. The request fails silently with a literal `{id}` in the URL.  
They want to test `POST /api/v1/tenants/{tenant_id}/workspaces` — it is not listed at all.  
They want to set an Authorization header — there is no way to do this.  

The tool that is meant to help them explore the API actively misleads them about what the API can do.

---

## The 5 WHYs Chain

### WHY 1 — Why does the API Explorer show only 30 of 169 endpoints?

**Because the endpoint list is hardcoded.**

```typescript
// edgequake_webui/src/components/shared/api-explorer.tsx, line ~48
const endpoints: Endpoint[] = [
  { method: 'GET', path: '/health', description: 'Check API health status', category: 'Health' },
  { method: 'POST', path: '/auth/login', ... },
  // ... 28 more hardcoded entries
  // The remaining 139 endpoints don't exist in this file.
];
```

The component has no connection to the OpenAPI specification. Every endpoint must be added manually.

---

### WHY 2 — Why is the list hardcoded instead of dynamically fetched from `/api-docs/openapi.json`?

**Because the developer who built the component chose to build a custom UI rather than consume the existing OpenAPI spec.**

The backend already serves:
- `GET /swagger-ui/` — full Swagger UI
- `GET /api-docs/openapi.json` — machine-readable OpenAPI 3.0 spec

These were available and operational **before** the custom explorer was built.  
See: [`edgequake/crates/edgequake-api/src/server.rs#L126`](../../edgequake/crates/edgequake-api/src/server.rs)

```rust
// server.rs — Swagger UI already running
if self.config.enable_swagger {
    app = app.merge(
        SwaggerUi::new("/swagger-ui")
            .url("/api-docs/openapi.json", ApiDoc::openapi())
            ...
    );
}
```

---

### WHY 3 — Why was a custom component built instead of embedding the existing Swagger UI?

**Because of a legitimate UX/design goal: visual consistency with the application theme.**

The reasoning is sound in isolation:
- The native Swagger UI has its own distinct visual style (blue/green brand)
- The EdgeQuake frontend uses a dark-mode-first design system
- Embedding an iframe to `/swagger-ui` would look jarring inside the dashboard

The intent was good. The execution missed a critical constraint: **a custom component has to be maintained to stay in sync with the API it documents.**

---

### WHY 4 — Why wasn't a "design-system-aware OpenAPI renderer" used instead of a fully custom component?

**Because the developer was not aware of (or did not evaluate) React-native OpenAPI UI libraries** (`@scalar/api-reference`, `swagger-ui-react`, `@stoplight/elements`) **that support full theming and consume the spec dynamically.**

These libraries:
- Accept a spec URL (`/api-docs/openapi.json`)
- Render all endpoints automatically from the spec
- Support CSS variable theming to match any design system
- Support auth token injection (Bearer token from the app's auth store)

No evaluation of these alternatives is recorded in the codebase, commit history, or documentation.

---

### WHY 5 — Why was there no evaluation framework to prevent this class of decision?

**Because there was no "build vs consume" checklist for UI features that duplicate existing infrastructure.**

The structural root cause is an absent design decision record. The team:
1. Had a living OpenAPI spec (infrastructure)
2. Built a UI that duplicates that spec's content (duplication)
3. Without a process to ask: "Does a library/service already exist that solves this?"

This is a **DRY violation at the architecture level**: the OpenAPI spec is the single source of truth for the API contract, but the custom explorer created a second source of truth (the hardcoded array) — one that immediately started drifting.

---

## Root Cause Statement

> The API Explorer is broken because a custom static component was built to duplicate information that is already maintained programmatically in the OpenAPI spec, with no mechanism to detect or prevent the divergence — and no evaluation of existing libraries that would have solved the problem correctly.

---

## Causal Chain Summary

```
SYMPTOM:  Explorer shows 18% of actual endpoints
    ↑
WHY 1:  Endpoint list is hardcoded in api-explorer.tsx
    ↑
WHY 2:  Component was built without consuming /api-docs/openapi.json
    ↑
WHY 3:  Custom UI was chosen for visual consistency (valid goal)
    ↑
WHY 4:  Themeable OpenAPI React libraries were not evaluated
    ↑
ROOT:   No "build vs consume" decision process for infra-duplicating UI features
```

---

## Structural Failures Identified

| Failure                     | Impact                             | DRY/SOLID Violation          |
| --------------------------- | ---------------------------------- | ---------------------------- |
| Hardcoded endpoint list     | 82% blind spot                     | DRY — second source of truth |
| No path parameter handling  | Silent request failures            | —                            |
| No auth token injection     | All protected endpoints untestable | —                            |
| No response schema display  | User cannot understand API         | —                            |
| No body schema validation   | Invalid requests sent silently     | —                            |
| Manual maintenance required | O(n) cost per endpoint             | DRY — must update two places |

---

## Prevention Principle (Post-Mortem)

**Rule:** Any UI feature that presents information already maintained in a machine-readable artifact (OpenAPI spec, database schema, config file) MUST first evaluate available consumer libraries before building a custom renderer.

**Checklist before building a custom data-display component:**
- [ ] Does a machine-readable source of truth already exist for this data?
- [ ] Is there a library/component that consumes it and renders it?  
- [ ] If yes: evaluate theming/integration first. Only build custom if clear gaps.
- [ ] Document the decision in a spec file.
