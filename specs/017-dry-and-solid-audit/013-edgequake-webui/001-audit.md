# edgequake-webui — DRY & SOLID Audit

**Path:** `edgequake_webui/`  
**Files:** ~361 TS/TSX | API client monolith ~2,035 LOC  
**Role:** Next.js 15 + React 19 client for EdgeQuake REST/WebSocket API

---

## Executive Summary

Frontend follows reasonable patterns (central `apiClient`, React Query, Zustand stores). Main debt: **monolithic API module** (`lib/api/edgequake.ts`), **duplicate status badge components**, **partial API client bypass** (raw `fetch` in same file), and **QueryMode type drift** from backend (missing `mix`, `bypass`). No critical production bugs identified; mostly **P1-P2 maintainability**.

---

## DRY Violations

| ID | P | Violation | Evidence | Remediation |
|----|---|-----------|----------|-------------|
| UI-DRY-001 | **P1** | Monolithic API client ~2,035 LOC | `lib/api/edgequake.ts` — all REST endpoints in one file | Split by domain: `documents.ts`, `graph.ts`, `query.ts` (partial split exists in `chat.ts`, `models.ts`) |
| UI-DRY-002 | **P1** | Dual status badge components | `status-badge.tsx` (304 LOC) vs `enhanced-status-badge.tsx` (173 LOC) | Merge or compose: base badge + enhancement layer |
| UI-DRY-003 | **P2** | Raw `fetch` bypasses `apiClient` | `edgequake.ts:58, 68` uses direct fetch; rest uses `client.ts` auth/retry | Route all through `apiClient` or `streamClient` |
| UI-DRY-004 | **P2** | QueryMode duplicated across stores | `use-query-store.ts`, `use-conversation-store.ts`, `use-settings-store.ts` | Single source in `types/index.ts` (exists) — ensure stores import, don't re-declare |
| UI-DRY-005 | **P2** | QueryMode type incomplete vs backend | `types/index.ts:419` — `"local" \| "global" \| "hybrid" \| "naive"` only | Add `mix`, `bypass`; sync with OpenAPI/codegen |
| UI-DRY-006 | **P2** | Types monolith | `types/index.ts` >1,100 LOC | Split by domain matching API modules |
| UI-DRY-007 | **P3** | Parallel query UI state | `use-query-store.ts` + `use-query-ui-store.ts` | Document boundary or merge if overlap |

---

## SOLID Violations

| ID | P | Principle | Violation | Evidence |
|----|---|-----------|-----------|----------|
| UI-SOLID-S-001 | **P1** | SRP | `edgequake.ts` god module | Documents, graph, query, auth, pipeline in one file |
| UI-SOLID-S-002 | **P2** | SRP | Large page components | Document manager, query interface mix fetch + render + state |
| UI-SOLID-O-001 | **P2** | OCP | New endpoint → edit 2,035 LOC file | Split API modules enable isolated extension |
| UI-SOLID-I-001 | **P2** | ISP | Components import full stores | Prefer selector hooks / narrow subscriptions |
| UI-SOLID-D-001 | **P2** | DIP | Some components call API directly | Prefer hooks layer (`use-document-mutations.ts` pattern) |

---

## Alignment with Backend Audit

| Backend issue | Frontend mirror |
|---------------|-----------------|
| Four `QueryMode` enums | UI type missing `mix`, `bypass` (UI-DRY-005) |
| Query vs chat duplication | Separate `chat.ts` + query in `edgequake.ts` — acceptable if shared types |
| Provider catalog hardcoded | Check `use-providers.ts` vs `/models` API |

---

## Remediation Plan

### P1

1. Split `edgequake.ts` into domain modules mirroring backend handlers
2. Consolidate status badges into one component with variants

### P2

3. Eliminate raw `fetch` in API layer
4. Extend `QueryMode` type; consider OpenAPI codegen (`openapi-typescript`)
5. Split `types/index.ts` by domain
6. Enforce hooks-as-boundary for data fetching

### P3

7. Review query store split; document store responsibilities

---

## Verification

```bash
cd edgequake_webui && bun test
cd edgequake_webui && bun run build
# After split: no file in lib/api/ > 500 LOC
```

---

## Positive Patterns

- Central `apiClient` with auth refresh (`client.ts`)
- React Query for server state (`query-provider.tsx`)
- Feature ID traceability in JSDoc (`@implements FEATxxxx`)
- Dedicated hooks for complex flows (`use-document-mutations.ts`, `use-query-page-state.ts`)
