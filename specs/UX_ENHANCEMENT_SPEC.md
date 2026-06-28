# UX Enhancement Specification — EdgeQuake WebUI

**Covers**: SPEC-0001 (#133), SPEC-0002 (#131), SPEC-0003 (#128), SPEC-0004 (#91)
**Status**: Proposed | **Created**: 2026-04-02

---

## Design Principles

Every addition follows the existing EdgeQuake design language:

- **Compact density** — `text-xs`/`text-[11px]`, `p-3`, `scale-75` switches
- **Neutral palette** — blues/slates for semantics, never alarming reds
- **Progressive disclosure** — collapsed by default, expand on demand
- **DRY** — reuse existing components (`DocumentDropzone`, `StatusBadge`, `EmptyState`, etc.)
- **Zero visual bloat** — inline badges, dots, and micro-text over bulky cards

---

## Architecture Overview

```
app/(dashboard)/layout.tsx
  +-- Sidebar (10 nav items — 1 new: Knowledge)
  +-- Header + TenantSelector
  +-- DynamicBreadcrumb
  +-- TenantGuard
       +-- /settings   <-- SPEC-0001: admin section (role-gated)
       +-- /knowledge  <-- SPEC-0002: dedicated knowledge injection page (NEW)
       +-- /query      <-- SPEC-0003: explain toggle + inline trace
       +-- /graph      <-- SPEC-0004: edge label config fix
```

---

## Page Impact Matrix

| Page         | SPEC-0001     | SPEC-0002               | SPEC-0003                  | SPEC-0004      |
| ------------ | ------------- | ----------------------- | -------------------------- | -------------- |
| `/settings`  | Admin section |                         |                            |                |
| `/knowledge` |               | Dedicated page (NEW)    |                            |                |
| `/query`     |               |                         | Explain toggle + trace     |                |
| `/graph`     |               |                         |                            | Edge label fix |
| citations    |               |                         | Provenance on entity click |                |
| sidebar      |               | New nav item (BookOpen) |                            |                |

---

## Cross-Feature Interaction

```
SPEC-0002 (Injection)
   |
   | entities appear in graph w/ source_type="injection"
   |
   +---> SPEC-0004 (Edge Labels)        SPEC-0003 (Explainability)
         |                                  |
         | labeled edges on                 | provenance shows
         | injection entities               | injection origin
         v                                  v
      graph-renderer.tsx               source-citations.tsx
         |                                  |
         +---- entity click ------> entity-provenance-dialog

SPEC-0001 (Tenant Quotas) — independent admin flow
   |
   +-- workspace count visible on /workspace
```

---

## Implementation Priority

| Spec | Effort | Backend Dep  | Risk | Order |
| ---- | ------ | ------------ | ---- | ----- |
| 0004 | Small  | serde rename | Low  | 1st   |
| 0001 | Medium | 2 endpoints  | Low  | 2nd   |
| 0002 | Medium | 3 endpoints  | Med  | 3rd   |
| 0003 | Large  | explain API  | High | 4th   |

---

## Per-Spec UX Specs

Detailed UX/UI specifications live alongside each spec's ADR:

- [`specs/0001_tenant_workspace_limits_issue_133/005_ux_spec.md`](0001_tenant_workspace_limits_issue_133/005_ux_spec.md)
- [`specs/0002_knowledge_injection_issue_131/005_ux_spec.md`](0002_knowledge_injection_issue_131/005_ux_spec.md)
- [`specs/0003_explainability_issue_128/005_ux_spec.md`](0003_explainability_issue_128/005_ux_spec.md)
- [`specs/0004_graph_edge_labels_issue_91/005_ux_spec.md`](0004_graph_edge_labels_issue_91/005_ux_spec.md)
