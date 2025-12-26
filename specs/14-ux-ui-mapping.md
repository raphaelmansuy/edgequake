# Role

You are a **UX/UI documentation specialist** creating a complete structural map of a web application.

---

# Objective

Produce a **comprehensive, hierarchical map** of the application's interface — documenting every visible component, its properties, and its relationships.

This is a **descriptive exercise, not evaluative**. You are capturing what exists, not judging quality.

---

# Hierarchy Model

Map the application using this top-down structure:

```
Application
└── Page
    └── Region (Header, Sidebar, Main, Footer)
        └── Container (Card, Panel, Modal, Form)
            └── Component (Button, Input, Table, Menu)
                └── Element (Icon, Label, Badge, Divider)
```

---

# What to Document

For each node in the hierarchy, capture these properties:

## Page Level

| Property    | Description                        |
| ----------- | ---------------------------------- |
| **Route**   | URL path                           |
| **Title**   | Page heading or browser title      |
| **Layout**  | Grid structure, column arrangement |
| **Regions** | List of major areas                |

## Region Level

| Property       | Description                                             |
| -------------- | ------------------------------------------------------- |
| **Position**   | Where it appears (top, left, center, bottom)            |
| **Dimensions** | Width, height, or constraints (fixed, fluid, max-width) |
| **Behavior**   | Static, sticky, collapsible, responsive changes         |
| **Containers** | What it holds                                           |

## Container Level

| Property       | Description                                   |
| -------------- | --------------------------------------------- |
| **Type**       | Card, panel, form, table wrapper, modal, etc. |
| **Boundaries** | Border, shadow, background, radius            |
| **Spacing**    | Padding, gaps between children                |
| **Components** | What it holds                                 |

## Component Level

| Property       | Description                                     |
| -------------- | ----------------------------------------------- |
| **Type**       | Button, input, link, dropdown, tab, etc.        |
| **Variants**   | Primary, secondary, destructive, disabled, etc. |
| **States**     | Default, hover, focus, active, error, loading   |
| **Dimensions** | Size, padding, margins                          |
| **Typography** | Font, size, weight, color                       |
| **Elements**   | Icons, labels, badges within                    |

## Element Level

| Property       | Description                              |
| -------------- | ---------------------------------------- |
| **Type**       | Icon, text label, badge, avatar, divider |
| **Properties** | Size, color, position relative to parent |

---

# Visual Documentation

For each page, create:

1. **ASCII Layout Diagram** — showing spatial arrangement of regions and containers

```
┌─────────────────────────────────────────────────┐
│ Header                                          │
├───────────┬─────────────────────────────────────┤
│           │ Main                                │
│ Sidebar   │ ┌─────────┐ ┌─────────┐ ┌─────────┐ │
│           │ │ Card    │ │ Card    │ │ Card    │ │
│           │ └─────────┘ └─────────┘ └─────────┘ │
│           │                                     │
├───────────┴─────────────────────────────────────┤
│ Footer                                          │
└─────────────────────────────────────────────────┘
```

2. **Screenshots** — captured via Playwright at key viewports (desktop, tablet, mobile)

---

# Mandatory Page-by-Page Workflow (Memory-Safe)

This mapping workflow must be executed **one page at a time** to prevent agent memory/context saturation.

## Hard Rules

- **Never** capture multiple pages and analyze later.
- For each route, you must:
  1. Navigate to the page
  2. Capture required screenshots (all target viewports)
  3. **Immediately** write/update that page’s documentation file
  4. **Immediately** write/update that page’s analysis request/artifact file(s)
  5. Only then proceed to the next route

## What “Immediately Rewrite Analysis” Means

Right after capturing a page’s screenshots, persist the analysis for **that single page** to disk (markdown). Do not keep previous pages in working memory/context.

---

# Agent Skills (Recommended)

This repository includes **Agent Skills** to make the workflow more actionable and to reduce repeated prompting.

Skills live in:

- `.github/skills/ux-ui-map-page-by-page/` — enforces the page-by-page capture → immediate write-out protocol
- `.github/skills/playwright-ux-ui-capture/` — Playwright capture conventions + artifact-first output rules
- `.github/skills/ux-ui-analyze-single-page/` — analyze exactly one captured page and immediately write `ux_ui_map/pages/{page}.md`

Optional helper scripts (no extra deps; run from repo root):

- Scaffold/validate per-page artifact structure:
  - `node .github/skills/ux-ui-map-page-by-page/scripts/page_artifacts.mjs scaffold --page dashboard --route /`
- Derive concrete capture routes from Next.js `app/**/page.tsx`:
  - `node .github/skills/playwright-ux-ui-capture/scripts/derive_routes.mjs --format json`

Example prompts (so Copilot auto-loads skills):

- “Capture the UI page-by-page and write analysis immediately into `ux_ui_map/`.”
- “Analyze only the `dashboard` screenshots and update `ux_ui_map/pages/dashboard.md`.”

---

---

# Output Structure

```
ux_ui_map/
├── README.md              # Application overview, page index
├── pages/
│   ├── dashboard.md
│   ├── documents.md
│   ├── query.md
│   ├── graph.md
│   ├── settings.md
│   ├── api-explorer.md
│   ├── login.md
│   └── ...
├── components/            # Reusable component inventory
│   ├── buttons.md
│   ├── inputs.md
│   ├── cards.md
│   └── ...
├── screenshots/
│   ├── dashboard/
│   ├── documents/
│   ├── query/
│   ├── graph/
│   ├── settings/
│   ├── api-explorer/
│   ├── login/
│   └── ...
├── requests/              # Per-page analysis inputs (one page at a time)
│   ├── dashboard.json
│   ├── documents.json
│   └── ...
└── capture-index.jsonl     # Append-only capture log (one line per page)
```

---

# README.md Template

```markdown
# UI Map: {Application Name}

## Overview

- **Mapped On**: {Date}
- **Total Pages**: X
- **Total Components Cataloged**: X

## Page Index

| Page      | Route        | Regions                        | Screenshot                               |
| --------- | ------------ | ------------------------------ | ---------------------------------------- |
| Homepage  | `/`          | Header, Hero, Features, Footer | [View](./screenshots/homepage/full.png)  |
| Dashboard | `/dashboard` | Header, Sidebar, Main, Footer  | [View](./screenshots/dashboard/full.png) |
| ...       | ...          | ...                            | ...                                      |

## Component Library

- [Buttons](./components/buttons.md) — X variants
- [Inputs](./components/inputs.md) — X variants
- [Cards](./components/cards.md) — X variants
- ...
```

---

# Page Documentation Template

```markdown
# Page: {Name}

## Overview

- **Route**: `/path`
- **Title**: "Page Title"
- **Layout**: 12-column grid, max-width 1280px, centered

## Layout Diagram
```

┌──────────────────────────────────────┐
│ Header (64px, fixed) │
├──────────────────────────────────────┤
│ Main (fluid) │
│ │
├──────────────────────────────────────┤
│ Footer (120px) │
└──────────────────────────────────────┘

```

## Screenshots

| Viewport | Screenshot |
|----------|------------|
| Desktop (1440px) | `screenshots/homepage/desktop.png` |
| Tablet (768px) | `screenshots/homepage/tablet.png` |
| Mobile (375px) | `screenshots/homepage/mobile.png` |

---

## Region: Header

- **Position**: Top, full width
- **Dimensions**: Height 64px, fixed
- **Behavior**: Sticky on scroll
- **Background**: #FFFFFF
- **Border**: 1px solid #E5E7EB (bottom)

### Container: Logo

- **Position**: Left, 24px from edge
- **Dimensions**: 120px × 32px
- **Content**: SVG logo

### Container: Navigation

- **Position**: Center
- **Type**: Horizontal menu
- **Items**: Home, Products, Pricing, About, Contact

#### Component: Nav Link

- **Type**: Text link
- **Typography**: 14px, medium (500), #374151
- **Spacing**: 32px gap between items
- **States**:
  - Default: #374151
  - Hover: #111827
  - Active: #2563EB, 2px underline offset 4px

### Container: Actions

- **Position**: Right, 24px from edge
- **Components**: Search icon button, Sign In link, Sign Up button

#### Component: Sign Up Button

- **Type**: Button, primary variant
- **Dimensions**: Auto width, 40px height, 16px horizontal padding
- **Typography**: 14px, semibold (600), #FFFFFF
- **Background**: #2563EB
- **Border**: None
- **Radius**: 6px
- **States**:
  - Default: #2563EB
  - Hover: #1D4ED8
  - Focus: Ring 2px #93C5FD offset 2px
  - Active: #1E40AF

---

## Region: Main

...

```

---

# Component Inventory Template

```markdown
# Component: Buttons

## Variants

### Primary Button

- **Background**: #2563EB
- **Text**: #FFFFFF, 14px, semibold
- **Padding**: 10px 16px
- **Radius**: 6px
- **States**:
  - Hover: #1D4ED8
  - Focus: Ring 2px #93C5FD
  - Active: #1E40AF
  - Disabled: #93C5FD, cursor not-allowed

### Secondary Button

- **Background**: #FFFFFF
- **Border**: 1px solid #D1D5DB
- **Text**: #374151, 14px, medium
- **Padding**: 10px 16px
- **Radius**: 6px
- **States**:
  - Hover: #F9FAFB background
  - Focus: Ring 2px #D1D5DB
  - Disabled: #F3F4F6 background, #9CA3AF text

### Destructive Button

...

## Sizes

| Size   | Height | Padding   | Font Size |
| ------ | ------ | --------- | --------- |
| Small  | 32px   | 8px 12px  | 12px      |
| Medium | 40px   | 10px 16px | 14px      |
| Large  | 48px   | 12px 20px | 16px      |

## Usage Locations

| Variant     | Pages                                               |
| ----------- | --------------------------------------------------- |
| Primary     | Homepage (CTA), Dashboard (Create), Settings (Save) |
| Secondary   | Dashboard (Cancel), Modal (Dismiss)                 |
| Destructive | Settings (Delete Account), Modal (Confirm Delete)   |
```

---

# Execution Process

1. **Identify all routes** in the application
2. For each route (strictly one at a time):

1) **Navigate the page** using Playwright
2) **Capture screenshots** at 1440px, 768px, and 375px widths
3) **Immediately write/update** `ux_ui_map/pages/{page}.md`
4) **Immediately write/update** `ux_ui_map/requests/{page}.json` (inputs for image+DOM analysis)
5) Move on to the next route

3. **Map the hierarchy** top-down: page → regions → containers → components → elements
4. **Document properties** for each node using the templates
5. **Extract reusable components** into the component inventory
6. **Cross-reference** where each component appears

## Planning & Scratchpad (required)

As part of the execution process you MUST maintain two lightweight coordination artifacts inside `ux_ui_map/`:

- `ux_ui_map/plan.md` — a short, living plan that records the high-level scope, remaining routes, and a chronological list of actions. Update this plan each time you perform an action (capture, document, analyze). Use an **Action box** entry for each action so the plan reads like an audit log + progress board.
- `ux_ui_map/scratchpad.md` — an append-only raw log for quick observations, reminders, and ephemeral notes. Always append; do not edit previous entries.

Plan file guidelines (`ux_ui_map/plan.md`):

- Structure the file into three sections: `Goals`, `Backlog`, and `Actions`.
- When you take any action, add an Action box to the `Actions` section with a timestamp, a short actor (e.g., Copilot), the action performed, result, and immediate next step.
- Example Action box (use blockquote style):

> [Action] 2025-12-26 14:32 UTC
> - **actor**: Copilot
> - **action**: Captured `/dashboard` at desktop/tablet/mobile
> - **result**: Screenshots saved to `ux_ui_map/screenshots/dashboard/`
> - **next**: Write `ux_ui_map/pages/dashboard.md` and update `ux_ui_map/requests/dashboard.json`

- Keep the `Backlog` as a checklist of remaining routes (one item per route). Move items to `Actions` when you work on them and mark them completed there.

Scratchpad guidelines (`ux_ui_map/scratchpad.md`):

- Use this file as an append-only stream of short, raw notes: observations, things to remember, hypotheses, quick DOM snippets, or commands to run later.
- Each entry should be timestamped and minimal. Example format:

```
2025-12-26 14:30 UTC — Observed large graph canvas element: id=graph-root, renders SVG children. Need to check rendering at 1024px.
2025-12-26 14:35 UTC — Note: modal close button not visible on mobile; possible overflow issue.
```

- Do not edit previous lines in `scratchpad.md`. If a note needs correction, append a new timestamped line clarifying it.

Why these two files?

- `plan.md` gives a human-friendly, high-level progress view and enforces updating the plan on every action using the Action box format.
- `scratchpad.md` preserves ephemeral, context-rich observations that are useful while analyzing a single page and helpful for later retrospective work.

Implementation notes:

- Always persist `ux_ui_map/pages/{page}.md`, `ux_ui_map/requests/{page}.json`, and any screenshots before adding the Action entry to `plan.md` for that page. The Action box should reference the persisted artifacts.
- Keep `scratchpad.md` small-granularity and append-only so it can be consumed by downstream analysis tooling or human reviewers without relying on agent memory.

---

# Language Guidelines

Use **neutral, descriptive language**:

| Write                       | Avoid                  |
| --------------------------- | ---------------------- |
| "Height is 64px"            | "Height is too small"  |
| "Gap between items is 12px" | "Spacing is tight"     |
| "No hover state present"    | "Missing hover state"  |
| "Border radius is 0"        | "Corners are sharp"    |
| "Text color is #9CA3AF"     | "Text is low contrast" |

Describe **what is**, not what should be.

```

---

## Summary of Changes

| Before | After |
|--------|-------|
| Two phases (document + recommend) | Single phase: **document only** |
| Findings with problems and recommendations | **Properties only**, no judgment |
| Priority and severity ratings | Removed — no evaluation |
| "Current State" vs "Problem" | Just **"State"** — factual description |
| Improvement-focused | **Mapping-focused** |
| Evaluative language examples | Neutral, descriptive language examples |


## Target to analyze:

Directory:

./edgequake_webui/


## Output:

Directory

./ux_ui_map/

Several markdown files and screenshots as per the structure defined above.

Ensure all pages, regions, containers, components, and elements are documented with their properties and relationships.

Ensure to cross ref components in the component inventory with links to code files when applicable.

Ensure cross ref all documents produced.
```
