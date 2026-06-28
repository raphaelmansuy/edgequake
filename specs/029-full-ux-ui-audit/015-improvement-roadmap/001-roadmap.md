# 001 — UX/UI Improvement Roadmap

**Philosophy:** Incremental, testable improvements. No big-bang redesigns.  
**Last updated:** 2026-06-28

---

## Implementation Status

```
✅ DONE    — Implemented, verified with screenshot
🔄 PARTIAL — Partially implemented
⬜ TODO    — Not yet started
```

---

## Phase 1 — Critical Accessibility ✅ COMPLETE

| ID | Issue | Status | Screenshot |
|----|-------|--------|-----------|
| A11Y-04 | `div[role="button"]` → `<button>` in folder-sidebar.tsx | ✅ | b-phase1 |
| A11Y-02 | Login error: `role="alert"` + `aria-describedby` | ✅ | - |
| A11Y-05 | Sidebar `<aside aria-label="...">` | ✅ pre-existing | - |
| A11Y-06 | `<h1>` on dashboard pages | ✅ pre-existing | - |
| CC-05 | `--ring` darkened to 0.4 for focus ring contrast | ✅ | - |
| A11Y-03 | `--input` border darkened to 0.78 (3:1 WCAG) | ✅ | - |
| KN-01 | Skip link programmatic focus fix | ✅ | - |

---

## Phase 2 — Navigation Overhaul ✅ COMPLETE

| ID | Issue | Status | Screenshot |
|----|-------|--------|-----------|
| NAV-01 | Grouped sidebar (primary/knowledge/system) | ✅ | 02-nav-groups-fixed.png |
| NAV-02 | Breadcrumb hidden at depth ≤ 1 | ✅ | 03-documents-no-breadcrumb.png |
| NAV-03 | Knowledge Graph → Graph, Knowledge remains | ✅ | - |
| NAV-04 | Breadcrumb owns container (no empty space) | ✅ | - |
| NAV-05 | Touch targets: `min-h-[44px]` | ✅ | - |
| NAV-08 | `DollarSign` → `BarChart2` for Costs | ✅ | - |

---

## Phase 3 — Color System ✅ COMPLETE

| ID | Issue | Status |
|----|-------|--------|
| CC-01 | `muted-foreground` 0.556 → 0.48 | ✅ |
| CC-03 | Status colors: 12 variants → 4 semantic groups (amber/blue/purple/green/red/orange) | ✅ |
| MI-06 | Status badge spinner overload fixed (AI-only spin, standard pulse, queued slow-pulse) | ✅ |
| TY-03 | `tabular-nums` on stats cards (pre-existing) | ✅ |

---

## Phase 4 — Query Interface ✅ COMPLETE

| ID | Issue | Status | Screenshot |
|----|-------|--------|-----------|
| QI-01 | Provider + filter moved into settings sheet | ✅ | 05-settings-sheet-with-provider-filter.png |
| QI-02 | Mode labels: Local→Focused, Global→Broad, Hybrid→Smart, Simple→Direct | ✅ | b1-02-query-renamed-modes.png |
| QI-02 | "Recommended" dot on Smart/Hybrid mode | ✅ | b1-02-query-renamed-modes.png |

---

## Phase 5 — Documents Interface 🔄 PARTIAL

| ID | Issue | Status |
|----|-------|--------|
| DI-05 | Pagination "1–20 of 47" format | ✅ | b2-02-documents-pagination-range.png |
| LS-01 | ARIA on skeleton loading (`aria-busy`, `aria-label`) | ✅ |
| DI-01 | Toolbar consolidation | ⬜ |
| DI-02 | Selection bar replaces toolbar | ⬜ |
| DI-03 | Column visibility toggle | ⬜ |
| PP-01 | Optimistic mutations | ⬜ |

---

## Phase 6 — Loading & Empty States 🔄 PARTIAL

| ID | Issue | Status |
|----|-------|--------|
| IH-03 | System status card collapses when healthy → compact "All systems operational" | ✅ | b1-01-dashboard-system-status-collapsed.png |
| LS-03 | Stats cards show zeroHint when value=0 | ✅ |
| ES-04 | Graph empty state | ⬜ |
| PD-04 | First-time welcome state on dashboard | ⬜ |

---

## Phase 7 — Progressive Disclosure 🔄 PARTIAL

| ID | Issue | Status |
|----|-------|--------|
| PD-03 | PDF backend selector already in dropzone (pre-existing) | ✅ |
| PD-01 | Settings page grouped left-nav | ⬜ |
| PD-02 | Query settings basic/advanced toggle | ⬜ |
| PD-06 | Bulk reprocess default + advanced | ⬜ |

---

## Phase 8 — Design Token Consolidation ✅ COMPLETE

| ID | Issue | Status |
|----|-------|--------|
| All tokens | Shadow/transition/z-index tokens (pre-existing in design-tokens.css) | ✅ |
| MI-01 | Button active/press state `scale(0.97)` | ✅ |
| MI-07 | Focus ring fade-in transition 80ms | ✅ |

---

## Phase 9 — Micro-interactions 🔄 PARTIAL

| ID | Issue | Status |
|----|-------|--------|
| MI-03 | Active/press state on buttons | ✅ |
| MI-07 | Focus ring fade-in | ✅ |
| MI-06 | Status badge spinner overload (done in Phase 3) | ✅ |
| MI-02 | Card hover shadow escalation | ⬜ |
| MI-04 | Batch bar entry animation | ⬜ |
| MI-08 | Sidebar collapse animation | ⬜ |

---

## Phase 10 — Error Experience 🔄 PARTIAL

| ID | Issue | Status |
|----|-------|--------|
| ES-01 | Backend banner: fixed overlay (no CLS) | ✅ |
| ES-02 | Backend banner: user-friendly language | ✅ |
| KN-01 | Skip link programmatic focus | ✅ |
| ES-05 | Document error inline message | ⬜ |
| ES-06 | Humanize backend error messages | ⬜ |
| ES-08 | Undo after delete | ⬜ |

---

## Phase 11 — Keyboard & Performance ✅ COMPLETE

| ID | Issue | Status |
|----|-------|--------|
| PP-02 | Deduplicate health check polling (React Query shared cache) | ✅ |
| PP-03 | Graph loading overlay (already has rotating tips) | ✅ pre-existing |
| KN-03 | Arrow key table navigation | ⬜ |
| KN-06 | Keyboard shortcuts surface | ⬜ |

---

## Scroll Fixes ✅ COMPLETE

| Page | Fix | Status |
|------|-----|--------|
| Knowledge page | Wrapped in `<ScrollArea className="h-full">` | ✅ |
| Knowledge detail page | Wrapped in `<ScrollArea className="h-full">` | ✅ |
| Pipeline monitor | `h-[calc(100vh-...)]` → `h-full` | ✅ |

---

## New Work Items (2026-06-28 Sprint 2)

### P0 — Right Panel Redesign (RP-01 to RP-09)

| ID | Issue | Files | Effort |
|----|-------|-------|--------|
| RP-01 | Remove ID subtitle from panel header | right-panel.tsx, document-preview-right-panel.tsx | XS |
| RP-02 | Fix value truncation — all fields clipped at panel edge | document-preview-panel.tsx | S |
| RP-03 | Fix cost color bug — $0.158 shows as $0 in orange | document-preview-panel.tsx | XS |
| RP-04 | Lowercase section labels (remove CAPS bureaucracy) | document-preview-panel.tsx | XS |
| RP-05 | Remove duplicate title (header + content h3) | document-preview-panel.tsx | S |
| RP-06 | Remove separator overuse | document-preview-panel.tsx | XS |
| RP-07 | Actions pinned to bottom (always visible) | document-preview-panel.tsx | S |
| RP-08 | Cost row: compact single-line summary | document-preview-panel.tsx | S |
| RP-09 | Tooltip for full model name | document-preview-panel.tsx | XS |

### P0 — Virtual Scrolling (VS-01 to VS-03)

| ID | Issue | Files | Effort |
|----|-------|-------|--------|
| VS-01 | Replace pagination with virtual scroll | document-table-section.tsx, use-document-queries.ts | M |
| VS-02 | Remove pagination controls bar | pagination-controls.tsx, document-table-section.tsx | XS |
| VS-03 | Large-page-size fetch (500 docs at once) | use-document-queries.ts, document-manager.tsx | S |

### P1 — System Status Widget (SS-01)

| ID | Issue | Files | Effort |
|----|-------|-------|--------|
| SS-01 | Healthy badge with animated pulse indicator | system-status.tsx | XS |

### P1 — High Impact
1. **ES-04**: Graph empty state when 0 nodes
2. **MI-04**: Batch actions bar entry animation
3. **DI-01/DI-02**: Document toolbar + selection bar redesign
4. **PD-04**: First-time welcome state on dashboard
5. **MI-02**: Card hover — shadow escalation only (remove translate)

### P2 — Polish
6. **KN-03**: Arrow key navigation in document table
7. **ES-05/ES-06**: Inline error messages + humanized error copy
8. **PD-01**: Settings page grouped navigation
9. **MI-08**: Sidebar collapse smooth animation
10. **ES-08**: Undo after delete

---

## Measurement Criteria (vs. baseline)

```
Metric                      Before    Current
────────────────────────────────────────────────────────
Sidebar nav items           10        7 (4+2+4 grouped)
Status color variants       12        6 semantic groups
Query toolbar controls      5         3 (2 in sheet)
Breadcrumb noise at depth 1 Always    Never
System status card          Full card Compact 1-line
Pagination format           "Page 1 of 3" "1-20 of 47"
Mode labels                 Technical User-friendly
WCAG focus ring contrast    ~2.7:1    ~5:1
Input border contrast        ~1.4:1   ~3.1:1
Muted text contrast          ~4.2:1   ~5.7:1
```

```
P0 — Blocks users (accessibility/functional failures)
P1 — High impact on daily workflow
P2 — Meaningful polish, noticeable improvement
P3 — Refinement, nice-to-have
```

**Effort:**
```
XS — < 1 hour  (CSS tweak, token change)
S  — 1-4 hours (single component change)
M  — 4-8 hours (feature or multi-component)
L  — 1-2 days  (systemic change)
XL — > 2 days  (architectural change)
```

---

## Phase 1 — Critical Accessibility (Week 1)

These issues block screen reader users and fail WCAG 2.1 AA.

| ID      | Issue                                                   | Files              | Effort | PR Size |
| ------- | ------------------------------------------------------- | ------------------ | ------ | ------- |
| A11Y-04 | `div[role="button"]` → `<button>` in folder-sidebar.tsx | folder-sidebar.tsx | XS     | Tiny    |
| A11Y-02 | Login error: add `role="alert"` + `aria-describedby`    | login/page.tsx     | XS     | Tiny    |
| A11Y-05 | Sidebar `<div>` → `<aside aria-label="...">`            | sidebar.tsx        | XS     | Tiny    |
| A11Y-06 | Add `<h1 className="sr-only">` to all dashboard pages   | 5 page files       | S      | Small   |
| CC-05   | Darken `--ring` token for 3:1 focus ring contrast       | globals.css        | XS     | Tiny    |
| A11Y-03 | Darken `--input` border to 3:1 contrast                 | globals.css        | XS     | Tiny    |

**Total Phase 1 effort:** ~4 hours  
**Impact:** WCAG 2.1 AA compliance on critical paths

---

## Phase 2 — Navigation Overhaul (Week 2)

Reduce cognitive load in the primary navigation.

| ID     | Issue                                                | Files                   | Effort |
| ------ | ---------------------------------------------------- | ----------------------- | ------ |
| NAV-01 | Group sidebar items (primary / knowledge / system)   | sidebar.tsx             | S      |
| NAV-02 | Remove duplicated workspace selector from sidebar    | sidebar.tsx, header.tsx | S      |
| NAV-03 | Rename "Knowledge" and "Knowledge Graph" items       | sidebar.tsx, i18n files | XS     |
| NAV-04 | Hide breadcrumb at depth ≤ 1                         | dynamic-breadcrumb.tsx  | XS     |
| NAV-05 | Touch targets: 40px → 44px                           | sidebar.tsx             | XS     |
| NAV-08 | Replace `DollarSign` icon with `BarChart2` for Costs | sidebar.tsx             | XS     |

**Total Phase 2 effort:** ~1 day  
**Impact:** Cleaner navigation, better cognitive hierarchy, mobile-friendlier

---

## Phase 3 — Color System Cleanup (Week 2-3)

Consolidate status colors and fix muted foreground contrast.

| ID    | Issue                                                 | Files                             | Effort |
| ----- | ----------------------------------------------------- | --------------------------------- | ------ |
| CC-01 | Reduce status colors from 12 to 4 semantic groups     | status-badge.tsx                  | M      |
| CC-03 | Status color overload — merge similar shades          | status-badge.tsx                  | S      |
| CC-04 | Replace hard-coded color classes with semantic tokens | stats-card.tsx, quick-actions.tsx | S      |
| TY-03 | Add `tabular-nums` to all numeric displays            | stats-card.tsx, cost-cell.tsx     | XS     |
| CC-01 | `muted-foreground` contrast: 0.556 → 0.48             | globals.css                       | XS     |

**Total Phase 3 effort:** ~1 day  
**Impact:** Accessible color system, cleaner visual hierarchy

---

## Phase 4 — Query Interface Polish (Week 3)

Reduce toolbar density, improve mode selector discoverability.

| ID    | Issue                                                                        | Files                                         | Effort |
| ----- | ---------------------------------------------------------------------------- | --------------------------------------------- | ------ |
| QI-01 | Move provider + document filter into settings sheet                          | query-interface.tsx, query-settings-sheet.tsx | M      |
| QI-02 | Rename mode labels (Local/Global/Hybrid/Simple → Focused/Broad/Smart/Direct) | query-mode-selector.tsx, i18n                 | XS     |
| QI-02 | Add "recommended" indicator to Hybrid/Smart mode                             | query-mode-selector.tsx                       | XS     |
| QI-03 | Standardize chat message max-width using CSS token                           | query-interface.tsx                           | XS     |
| QI-04 | Move image previews above textarea                                           | query-interface.tsx                           | S      |
| MI-05 | Add streaming cursor blink                                                   | chat-message.tsx                              | XS     |

**Total Phase 4 effort:** ~1 day  
**Impact:** Cleaner chat interface, better mode discoverability

---

## Phase 5 — Documents Interface Polish (Week 4)

Improve the document management workflow.

| ID    | Issue                                             | Files                                             | Effort |
| ----- | ------------------------------------------------- | ------------------------------------------------- | ------ |
| DI-01 | Consolidate document toolbar (primary + overflow) | document-header.tsx, document-toolbar-section.tsx | M      |
| DI-02 | Selection bar replaces toolbar (not supplements)  | batch-actions-bar.tsx, document-manager.tsx       | M      |
| DI-03 | Column visibility toggle (show/hide Chunks, Cost) | document-table-section.tsx                        | M      |
| DI-04 | Two-tier status display (macro + detail)          | status-badge.tsx                                  | M      |
| DI-05 | Improve pagination "1–20 of 47 documents" format  | pagination-controls.tsx                           | XS     |
| DI-07 | Upload progress panel (bottom-right fixed)        | pdf-upload-progress.tsx                           | L      |
| PP-01 | Optimistic delete/reprocess mutations             | use-document-mutations.ts                         | M      |

**Total Phase 5 effort:** ~2 days  
**Impact:** Significantly more usable document management workflow

---

## Phase 6 — Loading & Empty States (Week 4-5)

Consistent, high-quality zero/loading states.

| ID    | Issue                                           | Files                              | Effort |
| ----- | ----------------------------------------------- | ---------------------------------- | ------ |
| LS-01 | Add ARIA attributes to skeleton loading         | document-table-states.tsx          | XS     |
| LS-03 | Dashboard stats with onboarding CTA when 0 docs | stats-card.tsx, dashboard/page.tsx | S      |
| ES-04 | Graph empty state when 0 nodes                  | graph-viewer.tsx                   | S      |
| PD-04 | First-time welcome state on dashboard           | dashboard/page.tsx                 | M      |
| LS-01 | Shimmer animation instead of pulse              | globals.css                        | S      |
| IH-03 | Collapse system-status card when healthy        | system-status.tsx                  | XS     |

**Total Phase 6 effort:** ~1 day  
**Impact:** Better first-time experience, less confusing 0-state screens

---

## Phase 7 — Progressive Disclosure (Week 5)

Reduce settings complexity.

| ID    | Issue                                        | Files                                             | Effort |
| ----- | -------------------------------------------- | ------------------------------------------------- | ------ |
| PD-01 | Settings page: grouped left-nav categories   | settings/page.tsx                                 | L      |
| PD-02 | Query settings: show basic/advanced toggle   | query-settings-sheet.tsx                          | S      |
| PD-03 | Move PDF backend selector to upload advanced | document-header.tsx, pdf-parser-backend-field.tsx | S      |
| PD-06 | Bulk reprocess: default + advanced option    | bulk-reprocess-dialog.tsx                         | S      |

**Total Phase 7 effort:** ~1.5 days  
**Impact:** Dramatically cleaner settings UX

---

## Phase 8 — Design Token Consolidation (Week 6)

Establish a complete, consistent token system.

| ID    | Issue                                           | Files                   | Effort |
| ----- | ----------------------------------------------- | ----------------------- | ------ |
| TK-01 | Add shadow tokens                               | design-tokens.css       | XS     |
| TK-01 | Add transition duration tokens                  | design-tokens.css       | XS     |
| TK-01 | Add z-index scale tokens                        | design-tokens.css       | XS     |
| TK-01 | Consume `--chat-message-max-width` token in JSX | query-interface.tsx     | XS     |
| TY-01 | Full typography scale as tokens                 | design-tokens.css       | S      |
| MI-01 | Standardize transition durations                | globals.css, components | M      |

**Total Phase 8 effort:** ~1 day  
**Impact:** Consistent, maintainable design system

---

## Phase 9 — Micro-interactions (Week 6-7)

Polish hover, press, and motion states.

| ID    | Issue                                         | Files                     | Effort |
| ----- | --------------------------------------------- | ------------------------- | ------ |
| MI-02 | Card hover: shadow escalation vs. translate   | stats-card.tsx            | XS     |
| MI-03 | Active/press state on buttons                 | globals.css or button.tsx | XS     |
| MI-04 | Batch action bar entry animation              | batch-actions-bar.tsx     | XS     |
| MI-06 | Fix status badge spinning (too many spinners) | status-badge.tsx          | S      |
| MI-07 | Focus ring fade-in transition                 | globals.css               | XS     |
| MI-08 | Sidebar collapse animation                    | sidebar.tsx               | S      |

**Total Phase 9 effort:** ~4 hours  
**Impact:** Noticeably more polished interaction feel

---

## Phase 10 — Error Experience (Week 7)

Improve error messages and recovery paths.

| ID    | Issue                                      | Files                                             | Effort |
| ----- | ------------------------------------------ | ------------------------------------------------- | ------ |
| ES-01 | Backend banner: prevent CLS                | backend-status-banner.tsx                         | S      |
| ES-02 | Backend banner: user-friendly language     | backend-status-banner.tsx                         | XS     |
| ES-05 | Document error: inline truncated message   | document-table-row.tsx, error-message-popover.tsx | S      |
| ES-06 | Humanize backend error messages            | lib/utils/document-status.ts                      | M      |
| ES-08 | Undo after delete (toast with undo action) | use-document-mutations.ts, sonner config          | M      |
| KN-01 | Fix skip link focus management             | skip-link.tsx                                     | XS     |

**Total Phase 10 effort:** ~1 day  
**Impact:** Higher user trust, clearer error recovery

---

## Phase 11 — Keyboard & Performance (Week 8)

Final polish on keyboard navigation and performance.

| ID    | Issue                                    | Files                                 | Effort |
| ----- | ---------------------------------------- | ------------------------------------- | ------ |
| KN-03 | Arrow key navigation in document table   | document-table-row.tsx                | S      |
| KN-06 | Surface keyboard shortcuts in app UI     | keyboard-shortcuts-help.tsx           | S      |
| PP-02 | Deduplicate backend health check polling | header.tsx, backend-status-banner.tsx | S      |
| PP-03 | Phase-aware loading text for graph       | graph-loading-overlay.tsx             | XS     |
| LS-06 | Mobile query toolbar overflow fix        | query-interface.tsx                   | S      |

**Total Phase 11 effort:** ~4 hours

---

## Issue Cross-Reference Index

| Issue ID | Description                             | Audit File                 | Phase |
| -------- | --------------------------------------- | -------------------------- | ----- |
| A11Y-01  | Status badge color-only differentiation | 002-accessibility          | 3     |
| A11Y-02  | Login error aria                        | 002-accessibility          | 1     |
| A11Y-03  | Input border contrast                   | 002-accessibility          | 1     |
| A11Y-04  | div[role=button]                        | 002-accessibility          | 1     |
| A11Y-05  | Sidebar aside landmark                  | 002-accessibility          | 1     |
| A11Y-06  | Page h1 headings                        | 002-accessibility          | 1     |
| CC-01    | muted-foreground contrast               | 006-contrast-color         | 3     |
| CC-02    | Input border WCAG 1.4.11                | 006-contrast-color         | 1     |
| CC-03    | Status color overload (12 variants)     | 006-contrast-color         | 3     |
| CC-04    | Hard-coded color classes                | 006-contrast-color         | 8     |
| CC-05    | Focus ring contrast                     | 006-contrast-color         | 1     |
| DI-01    | Toolbar density                         | 008-documents              | 5     |
| DI-02    | Dual toolbar problem                    | 008-documents              | 5     |
| DI-03    | Column width distribution               | 008-documents              | 5     |
| DI-04    | 15 status states                        | 008-documents              | 3     |
| DI-05    | Pagination total count format           | 008-documents              | 5     |
| DI-06    | Double-click discoverability            | 008-documents              | 3     |
| DI-07    | Upload progress inline vs. panel        | 008-documents              | 5     |
| DI-08    | Search debounce indicator               | 008-documents              | 5     |
| ES-01    | Banner CLS                              | 010-error-surfacing        | 10    |
| ES-02    | Technical error language                | 010-error-surfacing        | 10    |
| ES-03    | Login error missing ARIA                | 010-error-surfacing        | 1     |
| ES-04    | Generic error messages                  | 010-error-surfacing        | 10    |
| ES-05    | Error discoverability                   | 010-error-surfacing        | 10    |
| ES-06    | No recovery guidance                    | 010-error-surfacing        | 10    |
| ES-07    | Toast accessibility                     | 010-error-surfacing        | 1     |
| ES-08    | Undo after delete                       | 010-error-surfacing        | 10    |
| IH-01    | Stats cards without narrative           | 003-information-hierarchy  | 6     |
| IH-02    | CTA flat hierarchy                      | 003-information-hierarchy  | 6     |
| IH-03    | System status card                      | 003-information-hierarchy  | 6     |
| IH-04    | Recent activity stub                    | 003-information-hierarchy  | 6     |
| IH-05    | Table column weight                     | 003-information-hierarchy  | 5     |
| IH-06    | Toolbar density                         | 003-information-hierarchy  | 4     |
| IH-07    | Status badge info density               | 003-information-hierarchy  | 3     |
| IH-08    | Query header overloaded                 | 003-information-hierarchy  | 4     |
| IH-09    | Mode selector visible always            | 003-information-hierarchy  | 4     |
| KN-01    | Skip link focus                         | 013-keyboard               | 10    |
| KN-02    | 10+ tab stops                           | 013-keyboard               | 2     |
| KN-03    | No arrow key table nav                  | 013-keyboard               | 11    |
| KN-04    | Dialog focus trap audit                 | 013-keyboard               | 1     |
| KN-05    | AlertDialog focus on cancel             | 013-keyboard               | 1     |
| KN-06    | Keyboard shortcuts hidden               | 013-keyboard               | 11    |
| KN-07    | Document keyboard selection             | 013-keyboard               | 5     |
| KN-08    | Missing focus-visible                   | 013-keyboard               | 1     |
| LS-01    | ARIA on skeleton loading                | 004-loading-empty          | 6     |
| LS-02    | Dashboard stats 0-state                 | 004-loading-empty          | 6     |
| LS-03    | Graph empty state                       | 004-loading-empty          | 6     |
| MI-01    | Inconsistent transition duration        | 009-micro-interactions     | 8     |
| MI-02    | Card hover overused                     | 009-micro-interactions     | 9     |
| MI-03    | No active/press state                   | 009-micro-interactions     | 9     |
| MI-04    | Batch bar entry animation               | 009-micro-interactions     | 9     |
| MI-05    | Chat streaming cursor                   | 009-micro-interactions     | 4     |
| MI-06    | Too many spinners                       | 009-micro-interactions     | 3     |
| MI-07    | Focus ring animation                    | 009-micro-interactions     | 9     |
| MI-08    | Sidebar collapse animation              | 009-micro-interactions     | 9     |
| NAV-01   | 10 flat nav items                       | 001-navigation             | 2     |
| NAV-02   | Workspace selector duplicated           | 001-navigation             | 2     |
| NAV-03   | Knowledge/Knowledge Graph confusion     | 001-navigation             | 2     |
| NAV-04   | Breadcrumb at depth 1                   | 001-navigation             | 2     |
| NAV-05   | Touch target 40px                       | 001-navigation             | 2     |
| NAV-08   | DollarSign icon                         | 001-navigation             | 2     |
| PD-01    | Settings all options flat               | 011-progressive-disclosure | 7     |
| PD-02    | Query settings basic/advanced           | 011-progressive-disclosure | 7     |
| PD-03    | PDF backend in toolbar                  | 011-progressive-disclosure | 7     |
| PD-04    | No first-time experience                | 011-progressive-disclosure | 6     |
| PD-06    | Bulk reprocess complexity               | 011-progressive-disclosure | 7     |
| PD-07    | Graph controls visibility               | 011-progressive-disclosure | 7     |
| PP-01    | No optimistic updates                   | 012-performance            | 5     |
| PP-02    | Duplicate health polling                | 012-performance            | 11    |
| PP-03    | Graph loading text static               | 012-performance            | 11    |
| QI-01    | Query header 5 controls                 | 007-query                  | 4     |
| QI-02    | Mode selector labels                    | 007-query                  | 4     |
| QI-03    | Chat message width                      | 007-query                  | 4     |
| QI-04    | Image attachment UX                     | 007-query                  | 4     |
| QI-05    | History panel no empty state            | 007-query                  | 6     |
| QI-07    | Stop button transition                  | 007-query                  | 9     |
| TK-01    | Missing design tokens                   | 005-typography             | 8     |
| TY-01    | Inconsistent type scale                 | 005-typography             | 8     |
| TY-02    | No letter-spacing for labels            | 005-typography             | 8     |
| TY-03    | No tabular-nums on numbers              | 005-typography             | 3     |

---

## Effort Summary by Phase

```
Phase  Description                Effort   Issues
─────────────────────────────────────────────────────────
1      Critical Accessibility     4 hours  6 issues
2      Navigation Overhaul        1 day    6 issues
3      Color System               1 day    5 issues
4      Query Interface            1 day    6 issues
5      Documents Interface        2 days   7 issues
6      Loading & Empty States     1 day    6 issues
7      Progressive Disclosure     1.5 day  6 issues
8      Design Token System        1 day    6 issues
9      Micro-interactions         4 hours  6 issues
10     Error Experience           1 day    6 issues
11     Keyboard & Performance     4 hours  5 issues
─────────────────────────────────────────────────────────
TOTAL                             ~11 days 65 issues
```

---

## Quick Wins (Do in 1 Hour Total)

These changes have disproportionate impact relative to effort:

```bash
# 1. Fix <aside> landmark in sidebar (XS effort, major a11y win)
# 2. Add h1.sr-only to all dashboard pages (XS × 5 files)
# 3. Darken --muted-foreground to 0.48 (one CSS line)
# 4. Darken --input to 0.78 (one CSS line)
# 5. Touch targets 40px → 44px (one CSS line)
# 6. Add tabular-nums to all numeric values (grep + replace)
# 7. Breadcrumb: depth ≤ 1 returns null (3 lines of code)
# 8. Add role="alert" to login error (5 lines)
```

These 8 changes take ~1 hour and close multiple P0/P1 audit findings.

---

## Success Metrics

After completing all phases:

```
Metric                      Before    Target
────────────────────────────────────────────────────────
Lighthouse Accessibility    ~70       ≥ 95
Lighthouse Performance      ~80       ≥ 90
WCAG 2.1 AA violations      12+       0
Sidebar nav items           10        5-7 (grouped)
Query toolbar controls      5         3 (2 in sheet)
Status color variants       12        4 semantic groups
Design token coverage       ~40%      ~85%
```
