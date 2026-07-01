# SPEC-035 — API Explorer: Replace Custom Explorer with OpenAPI-Native Integration

**Spec:** `035-api-explorer`  
**Date:** 2026-07-01  
**Method:** Code is law — all claims are cross-referenced against live source files.  
**Status:** `DECISION: REPLACE` — See [007-decision-matrix.md](./007-decision-matrix.md)  
**Author:** Analysis via first-principles + codebase audit  

---

## TL;DR — Executive Decision

> **The custom API Explorer covers 18% of the real API and will drift further on every feature commit. Replace it with `@scalar/api-reference` (React component) consuming the backend's live `/api-docs/openapi.json` spec. Zero maintenance cost. Always in sync. Better UX. Auth-aware. Themeable.**

---

## The Evidence (Code is Law)

| Metric                              | Custom Explorer | Real API      |
| ----------------------------------- | --------------- | ------------- |
| **Endpoints hardcoded**             | 30              | —             |
| **Endpoints documented in OpenAPI** | —               | 169           |
| **Coverage**                        | **17.8%**       | 100%          |
| **Auto-synced with code**           | ❌ Never         | ✅ Always      |
| **Path parameters handled**         | ❌ No            | ✅ Yes         |
| **Request body schema**             | ❌ Static JSON   | ✅ Full schema |
| **Auth token injection**            | ❌ No            | ✅ Yes         |
| **Response schemas shown**          | ❌ No            | ✅ Yes         |
| **Maintenance cost / endpoint**     | O(n) manual     | O(0)          |

**Source files:**
- Custom explorer: [`edgequake_webui/src/components/shared/api-explorer.tsx`](../../edgequake_webui/src/components/shared/api-explorer.tsx)
- OpenAPI spec: [`edgequake/crates/edgequake-api/src/openapi.rs`](../../edgequake/crates/edgequake-api/src/openapi.rs) (169 paths in `paths()`)
- Backend serves: `GET /api-docs/openapi.json` + `GET /swagger-ui/`

---

## Documents in this Spec

| File                                                                 | Lens             | Key Question                       |
| -------------------------------------------------------------------- | ---------------- | ---------------------------------- |
| [001-five-whys.md](./001-five-whys.md)                               | Root Cause       | Why is the explorer broken?        |
| [002-first-principles.md](./002-first-principles.md)                 | First Principles | What are we really solving?        |
| [003-product-owner-lens.md](./003-product-owner-lens.md)             | Product Owner    | What is the business cost?         |
| [004-ux-ui-designer-lens.md](./004-ux-ui-designer-lens.md)           | UX/UI Designer   | What is the best user experience?  |
| [005-fullstack-developer-lens.md](./005-fullstack-developer-lens.md) | Full Stack Dev   | How to implement it correctly?     |
| [006-user-lens.md](./006-user-lens.md)                               | EdgeQuake User   | What does the user actually need?  |
| [007-decision-matrix.md](./007-decision-matrix.md)                   | Decision         | Custom vs Swagger UI vs Hybrid     |
| [008-implementation-plan.md](./008-implementation-plan.md)           | Implementation   | Phased plan with DRY/SOLID         |
| [009-cross-reference-matrix.md](./009-cross-reference-matrix.md)     | Cross-Reference  | Every claim linked to its evidence |

---

## Decision Summary

```
CHOSEN OPTION: B — Embed @scalar/api-reference React component
                     consuming /api-docs/openapi.json dynamically.

REJECTED: A (iframe to /swagger-ui) — visual mismatch, no auth injection
REJECTED: C (enhance custom explorer) — rebuilding what already exists
REJECTED: D (keep custom explorer) — 82% blind spot, O(n) drift
```

**Key rationale:**
1. Backend already generates a perfect, living OpenAPI spec — consume it.
2. `@scalar/api-reference` supports dark mode, theming, auth token injection.
3. Zero maintenance after integration — new endpoints appear automatically.
4. DRY principle: one source of truth (Rust `#[utoipa]` annotations → spec → UI).
5. SOLID: UI component has a single responsibility — render a spec URL.
