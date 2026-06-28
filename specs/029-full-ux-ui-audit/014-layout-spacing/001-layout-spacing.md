# 001 — Layout & Spacing Audit

**First Principle: Economy** — White space is not empty space; it is structure.

---

## Layout Architecture

### Application Shell

```
┌──────────────────────────────────────────────────────────────────┐
│ HEADER (h-12 = 48px)                           shrink-0         │
├──────────────────────────────────────────────────────────────────┤
│ BREADCRUMB (py-2 = border-b bg-muted/20)        shrink-0        │
├──────────────────────────────────────────────────────────────────┤
│ MAIN (flex-1 min-h-0 overflow-hidden)                           │
│                                                                  │
│  Each page controls its own scroll                               │
└──────────────────────────────────────────────────────────────────┘

Sidebar (h-screen, fixed width or collapsed):
┌────────────┐
│ Logo (h-12)│  48px — matches header height
│ Nav items  │  flex-1
│ [collapse] │  shrink-0
│ Version    │  shrink-0
└────────────┘
```

**Positive:** Header and logo area share the same `h-12` height — creates visual alignment at the top of the layout. This is correct.

---

## Spacing Inconsistencies

### LS-01 · Header Padding Inconsistency

```typescript
// header.tsx: px-3 (12px)
className="flex h-12 items-center ... px-3"

// breadcrumb: px-4 (16px)
className="border-b px-4 py-2"

// dashboard page cards: p-4 (16px)
// sidebar: px-2 (8px) for nav items, px-4 for logo
```

The horizontal padding is **not consistent** across the layout shell:
- Header: 12px
- Breadcrumb: 16px
- Content: 16-24px varies per page

**Fix:** Use a consistent horizontal gutter via CSS variable:

```css
/* design-tokens.css */
:root {
  --layout-gutter: 1rem;       /* 16px — standard */
  --layout-gutter-sm: 0.75rem; /* 12px — compact, header */
  --layout-gutter-lg: 1.5rem;  /* 24px — spacious, page content */
}
```

### LS-02 · Card Spacing Inconsistency

Different cards use different internal padding:

```typescript
// stats-card.tsx: p-4
<CardContent className="p-4">

// quick-actions.tsx: pb-3 / pt-0 / p-4 (link)
<CardHeader className="pb-3">
<CardContent className="pt-0">
  <Link className="... p-4 ...">

// settings: implied defaults
```

**Fix:** Standardize card padding:

```css
:root {
  --card-padding-sm: var(--space-3);  /* 12px — compact cards */
  --card-padding: var(--space-4);     /* 16px — default cards */
  --card-padding-lg: var(--space-6);  /* 24px — spacious cards */
}
```

### LS-03 · Dashboard Grid: Hardcoded Breakpoints

```typescript
// dashboard/page.tsx (implied from stats layout)
// Stats: grid with 4 columns on large screens
<div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
```

This works but mixes layout concerns with component code. The grid configuration should be extractable.

### LS-04 · List/Table Item Count

The pagination allows 10/20/50/100 items per page. These are all powers of 10 but:

- **10 items** is the minimum — this may feel cramped for power users who regularly work with 50+ documents
- The **default** should be **20** (a good balance for most screens)
- **50** and **100** are useful for bulk operations

Current default is likely 20 (from `useDocumentPreferences`).

**Combo/Select item count in dropdowns:**

Various selectors in the app (document filter, workspace selector) should paginate if they exceed 10-15 items. Verify `QueryDocumentFilter` handles large document lists (>50 documents).

---

## Responsive Behavior Audit

### Breakpoints

```
Tailwind v4 default breakpoints:
sm: 640px
md: 768px  
lg: 1024px
xl: 1280px
2xl: 1536px
```

### Mobile (< 640px)

**LS-05 · Mobile Sidebar: Drawer**

On mobile, the sidebar becomes a drawer (via Radix `Sheet`). The mobile sidebar trigger is a `Menu` icon in the header.

Issues:
- The `MobileSidebar` trigger is in the header at position left (correct)
- After navigating, the sheet should auto-close — verify `onItemClick` fires

**LS-06 · Query Interface: Toolbar Overflow on Mobile**

The query header has 5 controls which will overflow on mobile. Verify the `sm:gap-3` and hidden states for the subtitle text are working as intended.

On small viewports (375px), the header may show:
- [☰] Query [New] [Provider▼] [Mode▼] [Filter▼] [⚙]

At 375px wide, these 6 controls will not fit. The provider selector and filter should be moved to the settings sheet on mobile.

### Tablet (768-1024px)

**LS-07 · Content Width on Tablet**

At 768-1024px, the sidebar takes up 240px of the 768-1024px viewport. This leaves 528-784px for content. The `max-w-4xl` (896px) on query messages won't activate at 768px — content will stretch to fill the available space.

This is generally fine, but the chat input area should have a maximum useful width (around 700px) rather than stretching across the full content area.

---

## Spacing Rhythm

### Base Unit

The design token system uses a 4px base unit (`--space-1: 0.25rem`). This is correct and consistent with most design systems.

**Verify the 8px grid:**

All spacing should be multiples of 4px (preferably 8px):
- `p-2` (8px) ✓
- `p-3` (12px) ✓
- `p-4` (16px) ✓
- `p-5` (20px) ✓
- `p-6` (24px) ✓

**Violation check:** Any `p-2.5` (10px), `p-3.5` (14px) etc. should be reviewed:

```typescript
// sidebar.tsx: gap-2.5 (10px) — between logo icon and text
// query-interface: sm:gap-3 (12px) — acceptable

// These 0.5 increments are acceptable for fine-tuning icon+text spacing
// but should not appear in page-level layout
```

---

## Layout Quality Checklist

| Item                          | Status                 | Fix Priority |
| ----------------------------- | ---------------------- | ------------ |
| Consistent header padding     | ⚠️ 12px vs 16px         | P2           |
| Consistent card padding       | ⚠️ Varies               | P2           |
| 8px base grid compliance      | ✅ Mostly compliant     | —            |
| Responsive sidebar → drawer   | ✅ Implemented          | —            |
| Chat input max-width          | ⚠️ Stretches full width | P2           |
| Query toolbar mobile overflow | ⚠️ Risk at 375px        | P1           |
| Table pagination defaults     | ✅ 20 items (assumed)   | —            |
| Combo/select list pagination  | ❓ Unknown              | Audit needed |

---

## External References

- [8pt Grid System — spec.fm](https://spec.fm/specifics/8-pt-grid)
- [Responsive Design Patterns — NNGroup](https://www.nngroup.com/articles/responsive-web-design-definition/)
- [Spacing Systems — Refactoring UI](https://www.refactoringui.com/)
- [CSS Custom Properties for Layouts — Smashing Magazine](https://www.smashingmagazine.com/2022/05/css-custom-properties-layouts/)
