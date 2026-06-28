# 001 — Typography & Design Tokens Audit

**First Principle: Clarity** — Typography communicates structure before content is read.

---

## Typography Audit

### Current Font Stack

```
Sans: Geist Sans (next/font/local)
Mono: Geist Mono (code blocks, metrics)
Fallback: system-ui, sans-serif
```

Geist Sans is a strong choice — modern, highly legible, designed for data-dense UIs (Vercel).

### Type Scale Audit

The `design-tokens.css` defines text size tokens but the actual usage across components is inconsistent:

```
Defined tokens (design-tokens.css):
--text-xs: 0.75rem (12px)
--text-sm: 0.875rem (14px)
--text-base: 1rem (16px)
--text-lg: 1.125rem (18px)
--text-xl: 1.25rem (20px)
--text-2xl: 1.5rem (24px)
--text-3xl: 1.875rem (30px)
```

**Issue: Tokens defined but not consumed consistently**

Components use raw Tailwind classes (`text-sm`, `text-base`, `text-lg`) instead of the CSS custom properties. This means the tokens don't provide actual centralized control.

```
Evidence:
sidebar.tsx:    text-lg font-bold      (logo)
sidebar.tsx:    text-sm font-medium    (nav items)
header.tsx:     text-base font-semibold (mobile title)
query-interface: text-base sm:text-lg font-semibold (page title)
stats-card.tsx: text-3xl font-bold     (metric value)
              : text-sm                (card title)
```

### Page Title Hierarchy Issues

| Page | H1 Present? | Style | Notes |
|------|------------|-------|-------|
| Dashboard | ❌ No h1 | `StatsCard` titles are `text-sm` | No page-level heading |
| Documents | ❌ No h1 in `DocumentManager` | `text-xl font-semibold` in header | Header `<h1>` inside component |
| Query | ✅ `<h1>` inside header | `text-base sm:text-lg font-semibold` | Correct but small |
| Graph | ❌ No h1 | Dynamic title via `<title>` only | Missing visual h1 |
| Settings | ❌ No h1 | Card titles are effective h2/h3 | Needs page-level h1 |
| Login | ✅ CardTitle = h3 | But page has no h1 | CardTitle != page h1 |

### Type Rhythm Issues

**TY-01 · Inconsistent Line Heights**

```
Query input: leading-relaxed (1.625)
Chat messages: leading-relaxed (1.625)  ← good match
Stats card description: default (1.5)
Sidebar labels: default (1.5)
```

No body copy line-height token exists. `1.5` is the default which is acceptable but `1.6` is more legible for longer prose.

**TY-02 · Missing Letter-Spacing for Small Caps / Labels**

Group labels in the sidebar (proposed), table column headers, and section labels should use `tracking-wider` for better legibility at small sizes. Current `tracking-tight` on the logo is correct.

**TY-03 · Number Rendering**

Stats cards display large numbers (`text-3xl font-bold`). With Geist Sans, this looks clean. But numbers should use `tabular-nums` (font-variant-numeric: tabular-nums) to prevent layout shift when values change.

```typescript
// stats-card.tsx
// CURRENT:
<span className="text-3xl font-bold">{value}</span>

// FIX:
<span className="text-3xl font-bold tabular-nums">{value.toLocaleString()}</span>
```

---

## Design Token System Audit

### Token Categories Defined

```
✅ Spacing scale (--space-0 to --space-32)
✅ Chat message system tokens
✅ Code block system tokens
✅ Semantic color tokens (via shadcn/ui)
✅ Border radius tokens
⚠️ Typography scale partially defined
❌ Shadow scale not defined as tokens
❌ Transition duration tokens missing
❌ Z-index scale not defined as tokens
❌ Icon size tokens missing
❌ Component-specific density tokens (compact/default/spacious) missing
```

### Token Coverage Gap Analysis

**Missing: Shadow Tokens**

Currently shadows are hardcoded:
```typescript
// stats-card.tsx
className="shadow-sm hover:shadow-md"

// query-interface.tsx
'shadow-[0_2px_8px_rgba(0,0,0,0.08)]'
'dark:shadow-[0_2px_8px_rgba(0,0,0,0.2)]'
```

**Fix:** Add shadow tokens to `design-tokens.css`:

```css
:root {
  --shadow-xs: 0 1px 2px oklch(0 0 0 / 0.04);
  --shadow-sm: 0 1px 3px oklch(0 0 0 / 0.08), 0 1px 2px oklch(0 0 0 / 0.04);
  --shadow-md: 0 4px 6px oklch(0 0 0 / 0.07), 0 2px 4px oklch(0 0 0 / 0.05);
  --shadow-lg: 0 10px 15px oklch(0 0 0 / 0.10), 0 4px 6px oklch(0 0 0 / 0.05);

  --shadow-xs-dark: 0 1px 2px oklch(0 0 0 / 0.2);
  --shadow-sm-dark: 0 1px 3px oklch(0 0 0 / 0.3), 0 1px 2px oklch(0 0 0 / 0.2);
  --shadow-md-dark: 0 4px 6px oklch(0 0 0 / 0.35);
}
```

**Missing: Transition Tokens**

```css
:root {
  --transition-fast: 100ms ease;
  --transition-base: 150ms ease;
  --transition-slow: 250ms ease;
  --transition-spring: cubic-bezier(0.34, 1.56, 0.64, 1);
}
```

**Missing: Z-Index Scale**

Currently z-index values are scattered as magic numbers (`z-50`, `z-9999` in tour-provider.tsx):

```css
:root {
  --z-base: 0;
  --z-elevated: 10;
  --z-dropdown: 100;
  --z-sticky: 200;
  --z-overlay: 300;
  --z-modal: 400;
  --z-toast: 500;
  --z-tour: 9999; /* special case - must be above everything */
}
```

### Token Naming Convention Issues

**TK-01 · Chat vs. non-chat token naming inconsistency**

Tokens like `--chat-message-max-width: 800px` are defined in `design-tokens.css`. But the max-width for the query message container in `query-interface.tsx` uses `max-w-4xl lg:max-w-5xl` (Tailwind) rather than the `--chat-message-max-width` token.

This means the token and the actual value are desynced.

**Fix:** Consume the token:

```typescript
// query-interface.tsx — replace Tailwind class with token
<div 
  className="mx-auto px-4 sm:px-6 py-6"
  style={{ maxWidth: 'var(--chat-message-max-width)' }}
>
```

---

## Recommended Typography Scale

```
Token                 Size    Weight   Usage
─────────────────────────────────────────────────────
--type-display        2rem    700      Hero sections only
--type-h1             1.5rem  600      Page titles (sr-only or visible)
--type-h2             1.25rem 600      Section headers
--type-h3             1rem    600      Card titles, subsection headers
--type-body-lg        1rem    400      Lead/intro text
--type-body           0.875rem 400     Default body
--type-body-sm        0.8125rem 400    Secondary text, table cells
--type-label          0.75rem  500     Labels, badges, captions
--type-micro          0.6875rem 400    Timestamps, metadata
--type-mono           0.875rem  400    Code, IDs, technical values
```

---

## External References

- [Type Scale Generator — typescale.com](https://typescale.com/)
- [Geist Font — Vercel](https://vercel.com/font)
- [Modular Type Scale — A List Apart](https://alistapart.com/article/more-meaningful-typography/)
- [Design Tokens — Theo Tokens](https://github.com/tokens-studio/figma-plugin)
- [W3C Design Tokens Spec](https://design-tokens.github.io/community-group/format/)
- [CSS Custom Properties Best Practices](https://www.smashingmagazine.com/2017/04/start-using-css-custom-properties/)
