# UX/UI Full Audit — EdgeQuake Web Platform

**Spec ID:** 030  
**Date:** 2026-06-29  
**Style Target:** Clean · Minimalist · Elegant · Polished  
**References:** WCAG 2.1 AA, Nielsen Heuristics, Google Material Design 3, Refactoring UI (Wathan & Schoger)

---

## Structure

```
specs/030-full-ux-ui-audit/
├── README.md                         ← this file — master index
├── 001-audit-findings/               ← deep assessment (signal-heavy, no bloat)
│   ├── README.md
│   ├── 001-workspace-selector.md     ← tenant/workspace selector audit
│   ├── 002-dashboard-layout.md       ← dashboard layout audit
│   ├── 003-knowledge-graph.md        ← graph screen audit
│   ├── 004-documents.md              ← documents page audit
│   ├── 005-navigation-system.md      ← sidebar / nav audit
│   ├── 006-design-tokens.md          ← design system audit
│   └── 007-accessibility.md          ← WCAG / keyboard audit
├── 002-improvement-plans/            ← actionable plan per area
│   ├── README.md
│   ├── 001-workspace-fuzzy-search.md ← Command-palette selector
│   ├── 002-deeplinks.md              ← URL state management
│   ├── 003-dashboard-redesign.md     ← dashboard layout plan
│   └── 004-graph-improvements.md    ← graph screen plan
├── 003-implementations/              ← code change reference
│   └── README.md
└── e2e/
    ├── audit.spec.ts                 ← Playwright E2E audit spec
    └── screenshots/                  ← captured baseline + after
```

---

## Priority Matrix

| Area                       | Severity | Effort | Priority |
|----------------------------|----------|--------|----------|
| Workspace fuzzy search     | HIGH     | M      | P0       |
| Deeplinks for all states   | HIGH     | S      | P0       |
| Dashboard layout           | MED      | M      | P1       |
| Graph label readability    | HIGH     | S      | P0       |
| Graph toolbar bloat        | MED      | S      | P1       |
| Quick Actions visual noise | LOW      | S      | P2       |
| Sidebar discoverability    | MED      | S      | P1       |
| Documents pagination       | MED      | M      | P1       |
| Accessibility WCAG AA      | HIGH     | M      | P0       |

---

## Principles Applied

- **First Principles**: Every element must earn its place. Remove or hide until needed.
- **Progressive Disclosure**: Show only what is needed at each decision level.
- **Minimum Viable Interaction**: Fewest steps to task completion.
- **Consistent Affordances**: Same interaction pattern for same action type.

---

## External References

- [WCAG 2.1 AA Quick Reference](https://www.w3.org/WAI/WCAG21/quickref/)
- [Refactoring UI — Designing in Color](https://www.refactoringui.com/previews/building-your-color-palette)
- [Nielsen Norman Group — 10 Usability Heuristics](https://www.nngroup.com/articles/ten-usability-heuristics/)
- [Radix UI Accessible Patterns](https://www.radix-ui.com/primitives/docs/overview/accessibility)
- [cmdk — Command Menu Patterns](https://cmdk.paco.me/)
- [Every Layout — Consistent Spacing](https://every-layout.dev/)
