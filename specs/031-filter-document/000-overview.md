# SPEC-031: Document Scope Filter for Query Pipeline

> **Status**: Draft  
> **Priority**: High  
> **Complexity**: Medium  
> **Supersedes**: SPEC-005 (Date & Pattern Filters) — extends, does not replace  
> **Cross-refs**: SPEC-005, SPEC-006, SPEC-022, SPEC-028, SPEC-032

---

## 1. Executive Summary

EdgeQuake currently supports *implicit* document scoping via date ranges and name-pattern matching (SPEC-005). Users cannot *explicitly* select a discrete set of documents to restrict query scope.

This spec introduces **explicit document scope selection** — the ability to hand-pick specific documents (by ID or by searching by name) and materialize that selection as **dismissible pills** in the query interface. The default scope is the full workspace; narrowing is purely additive and opt-in.

---

## 2. Problem Statement

| Dimension     | Current State                                     | Desired State                                        |
| ------------- | ------------------------------------------------- | ---------------------------------------------------- |
| Scope control | Pattern-text + date range only                    | Explicit doc selection + existing filters            |
| Selection UX  | Hidden inside settings sheet                      | Visible pills in query bar                           |
| API contract  | `document_filter.document_pattern` → fuzzy match  | `document_filter.document_ids[]` → exact IDs         |
| Search        | `GET /documents?document_pattern=` (full listing) | `GET /documents/search?q=` (type-ahead, lightweight) |
| Default state | All workspace (implicit)                          | All workspace (explicit, represented as "no pills")  |

---

## 3. Scope

### In-Scope

- `DocumentFilter` struct extended with `document_ids: Option<Vec<String>>`
- New `GET /api/v1/documents/search` endpoint (type-ahead, lightweight)
- `QueryScopeBar` UI component: pill list + trigger button
- `DocumentPickerPopover` UI component: search + checkbox list
- Hook: `useDocumentScope` — manages selected doc state
- MCP tool `query` — exposes `document_ids` param
- Backward-compatible: all existing SPEC-005 filter fields retained

### Out-of-Scope

- Full document management in the query page (use Documents page)
- Saved/named scope presets (future spec)
- Cross-workspace scope (single workspace only)
- Retroactive migration of existing SPEC-005 filter state

---

## 4. Document Index

| File                                                     | Lens                 | Content                                       |
| -------------------------------------------------------- | -------------------- | --------------------------------------------- |
| [001-problem-analysis.md](001-problem-analysis.md)       | Engineering          | Current state deep-dive, gap analysis         |
| [002-ux-ui-design.md](002-ux-ui-design.md)               | UX/UI Designer       | Interaction flows, ASCII layouts, pill design |
| [003-api-backend-spec.md](003-api-backend-spec.md)       | System + AI Engineer | API contract, pipeline changes, storage       |
| [004-frontend-spec.md](004-frontend-spec.md)             | Full Stack           | Components, hooks, types, state management    |
| [005-mcp-integration.md](005-mcp-integration.md)         | MCP / Platform       | MCP tool changes, surface contract            |
| [006-edge-cases.md](006-edge-cases.md)                   | Engineering          | Edge cases, mitigations, invariants           |
| [007-implementation-plan.md](007-implementation-plan.md) | PM / Engineering     | Phased tasks, acceptance criteria             |

---

## 5. Key Design Decisions

### D1 — Explicit IDs Beat Patterns for Determinism

When a user manually selects a document, the intent is unambiguous. Storing the resolved `document_ids[]` at selection time ensures the query pipeline uses exactly the selected documents regardless of title changes.

### D2 — Default = Full Workspace, Zero Overhead

An empty `document_ids` array (or `null`) means no filtering. The query pipeline short-circuits immediately — no KV scan, no filtering logic — preserving today's baseline performance.

### D3 — Pills Are Primary; Settings Sheet Is Secondary

Visible pills in the query input bar are the primary affordance. The advanced pattern/date filters remain in the settings sheet. This separation follows progressive disclosure and matches how users think ("I want these 3 docs" vs "I want docs from January").

### D4 — Lightweight Search Endpoint, Not Full List

The document picker needs type-ahead with < 200ms perceived latency. Reusing `GET /documents?document_pattern=` loads full metadata per document. A dedicated `GET /documents/search?q=&page_size=20` endpoint returns minimal projections (`id`, `title`, `status`) only — no chunk counts, no cost data.

### D5 — IDs and Patterns Are OR-Unioned, Not Stacked

If both `document_ids` and `document_pattern` are set in `DocumentFilter`, the resolver unions both sets. This preserves backward compatibility and prevents confusion: any document matching either criterion is included.

---

## 6. Success Criteria

| Criterion                                | Metric                                                          |
| ---------------------------------------- | --------------------------------------------------------------- |
| Type-ahead search latency                | p99 < 200ms (KV scan on 1,000 docs)                             |
| Query overhead when no scope filter      | 0ms (short-circuit)                                             |
| Filter state survives page reload        | Persisted in `localStorage` via `useQuerySettings`              |
| ARIA accessibility                       | All interactive elements have labels; pills have remove buttons |
| No breaking change to SPEC-005 consumers | Existing `document_filter` JSON still accepted unchanged        |
