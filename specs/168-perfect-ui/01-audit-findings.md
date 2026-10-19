# 01 — Audit findings (round-00 baseline)

Method: every route × {light, dark} × {375, 768, 1440} captured against a rich,
hermetic mock workspace (41-node graph, 12 documents of mixed status, cost history,
conversations). Evidence: `e2e/screenshots/round-00-baseline/`. CLS numbers:
`e2e/reports/cls-round-00-baseline.json`.

Severity: **P0** broken / blocks use · **P1** clunky / clearly below best-in-class ·
**P2** polish.

## Shell & navigation

| ID | Sev | Finding |
| -- | --- | ------- |
| S-1 | P0 | Sidebar stays 256px wide at 768px. Pages get ~510px; the graph canvas collapses to ~250px beside the entity panel. |
| S-2 | P1 | Brand appears twice (logo row **and** footer block). Footer wastes 56px of rail on every page. |
| S-3 | P1 | Active nav item is a solid primary pill (near-black in light mode). Visually heavier than page content; fights with primary CTAs. |
| S-4 | P1 | Nav rows are 44px on desktop. 10 items + 3 group labels need scrolling at 800px tall viewports. Touch target size only matters on touch. |
| S-5 | P0 | Persisted collapsed sidebar is applied after hydration → whole page shifts horizontally (CLS). |
| S-6 | P1 | Header has no search / command trigger. ⌘K exists but is undiscoverable. |
| S-7 | P1 | Header: tenant selector, API badge, language, theme, user all compete at 768px. |
| S-8 | P1 | Page headers are inconsistent (some sticky bars, some inline titles, some `max-w-7xl`, some full-bleed). |

## Keyboard

| ID | Sev | Finding |
| -- | --- | ------- |
| K-1 | P0 | Go-to shortcuts are `⌘G / ⌘D / ⌘⇧Q / ⌘,` — all browser-reserved or conflicting (⌘D bookmark, ⌘G find-next, ⌘, preferences). They fail in real browsers. |
| K-2 | P0 | Only 4 of 10 destinations are reachable by keyboard shortcut. |
| K-3 | P0 | ⌘K on the graph page opens the global palette **and** the graph search (double handler). |
| K-4 | P1 | Command palette lists 7 pages; it cannot search entities, documents or conversations and shows no shortcut hints. |
| K-5 | P1 | Help dialog filters the catalog by the text "go to" — it lists shortcuts that do not exist or are broken. |
| K-6 | P1 | No chord system, no `[` sidebar toggle, no `/` focus-search outside the graph. |
| K-7 | P2 | Nav links do not advertise their shortcut. |

## Screens

| ID | Sev | Screen | Finding |
| -- | --- | ------ | ------- |
| Q-1 | P0 | Query | Hero is `justify-center h-full`: at 800px viewport the top (icon + title) is clipped. Same bug class at 375×812. |
| D-1 | P0 | Documents | Intake zone at default layout clips the "Choose files" button; Runs header label overlaps. |
| D-2 | P1 | Documents | Status column renders icon-only pills — state unreadable without hover. |
| D-3 | P1 | Documents | "Retry Failed" is a solid red button: loudest element on the page for a secondary action. |
| D-4 | P2 | Documents | "Rows per page" select truncates its value (`10(`). |
| G-1 | P0 | Graph | At 768px the canvas is unusable (S-1) and overlays (neighbourhood card, tour "?" button, minimap, bookmarks) pile on the remaining area. |
| G-2 | P1 | Graph | Duplicate zoom controls (toolbar + right rail). Ego-depth card is shown without a selection. Hard-coded English strings. |
| C-1 | P1 | Costs | Period select wraps; empty-state Cost Trend chart renders a blank axis; empty "Resets in". |
| P-1 | P1 | Pipeline | Status tiles in dark mode are low-contrast. |
| H-1 | P1 | Dashboard | 375px: stat cards stack to one column, pushing real content below the fold; some cards missing. |
| W-1 | P2 | Workspace | Config cards have ragged heights; "Auto → omit" helper text is dense. |
| A-1 | P2 | API Explorer | Third-party renderer; acceptable. Ensure the surrounding frame stays consistent. |
| L-1 | P2 | Login | Fine. Add autofocus on username. |

## Principles chosen from the audit (drives 02)

1. One quiet shell: rail collapses itself below 1024px, never covers content.
2. Keyboard-first: every destination one chord; palette is the universal search.
3. Zero movement: layout-affecting preferences are applied before first paint.
4. Chrome appears only when it has something to say (selection-scoped cards).
5. Safe centering everywhere — content may never be clipped by its own container.
