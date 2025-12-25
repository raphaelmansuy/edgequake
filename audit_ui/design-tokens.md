# EdgeQuake Design Tokens

**Version:** 1.0.0  
**Last Updated:** December 25, 2025

This document defines the design token system for EdgeQuake WebUI, ensuring consistency across all components and screens.

---

## Typography Scale

| Token         | Size | Weight | Line Height | Usage                        |
| ------------- | ---- | ------ | ----------- | ---------------------------- |
| `--text-xs`   | 12px | 400    | 1.5         | Captions, labels, badges     |
| `--text-sm`   | 14px | 400    | 1.5         | Secondary text, descriptions |
| `--text-base` | 16px | 400    | 1.5         | Body text, form inputs       |
| `--text-lg`   | 18px | 500    | 1.4         | Subheadings, card titles     |
| `--text-xl`   | 20px | 600    | 1.3         | Section headings             |
| `--text-2xl`  | 24px | 600    | 1.2         | Page headings                |
| `--text-3xl`  | 30px | 700    | 1.1         | Hero headings                |

### Current Implementation

```css
/* From typography audit */
body {
  font-size: 16px;
  line-height: 24px; /* 1.5 ratio - ✅ Good */
  font-weight: 400;
}

h1 {
  font-size: 24px;
  line-height: 32px; /* 1.33 ratio - ✅ Good */
  font-weight: 700;
}
```

### Recommendations

- Enforce single H1 per page (currently 2 on dashboard)
- Add H2 styling for mobile branding text
- Use `text-muted-foreground` consistently for secondary text

---

## Spacing Scale

| Token         | Value | Tailwind | Usage                            |
| ------------- | ----- | -------- | -------------------------------- |
| `--space-0.5` | 2px   | `p-0.5`  | Icon margins                     |
| `--space-1`   | 4px   | `p-1`    | Tight grouping, badge padding    |
| `--space-2`   | 8px   | `p-2`    | Button padding, related elements |
| `--space-3`   | 12px  | `p-3`    | Default gap, form spacing        |
| `--space-4`   | 16px  | `p-4`    | Card padding, section padding    |
| `--space-5`   | 20px  | `p-5`    | Medium sections                  |
| `--space-6`   | 24px  | `p-6`    | Card padding (large)             |
| `--space-8`   | 32px  | `p-8`    | Section margins                  |
| `--space-10`  | 40px  | `p-10`   | Large sections                   |
| `--space-12`  | 48px  | `p-12`   | Page margins                     |
| `--space-16`  | 64px  | `p-16`   | Major sections                   |

### Current Usage

- Card padding: `p-4` to `p-6` (16-24px) ✅
- Section gaps: `space-y-6` to `space-y-8` (24-32px) ✅
- Main content padding: `p-6` (24px) ✅

### Recommendations

- Standardize page padding to `p-page` custom class
- Use `gap` utilities instead of margins for grids

---

## Panel Dimensions

| Token                  | Value         | Notes                   |
| ---------------------- | ------------- | ----------------------- |
| `--sidebar-width`      | 256px (16rem) | Desktop expanded        |
| `--sidebar-collapsed`  | 64px (4rem)   | Icon-only mode          |
| `--panel-right-width`  | 400px         | Query context panel     |
| `--panel-right-narrow` | 320px         | Alternative narrow mode |
| `--panel-min-width`    | 200px         | Resize constraint       |
| `--panel-max-width`    | 480px         | Resize constraint       |
| `--header-height`      | 64px (4rem)   | Fixed header            |
| `--breadcrumb-height`  | 48px (3rem)   | Breadcrumb bar          |

### Current Implementation

```tsx
// sidebar.tsx
className={cn(
  "hidden border-r bg-card md:block transition-all duration-300",
  sidebarCollapsed ? "w-16" : "w-64"  // 64px / 256px
)}

// right-panel.tsx
const panelWidth = width === 'narrow' ? 'w-80' : 'w-[400px]'; // 320px / 400px
```

### Recommendations

- Add localStorage persistence for sidebar state
- Implement keyboard shortcut for collapse toggle
- Add resize handle for right panel

---

## Color Tokens

### Light Theme

```css
:root {
  --background: oklch(1 0 0); /* White */
  --foreground: oklch(0.145 0 0); /* Near black */
  --primary: oklch(0.205 0 0); /* Dark gray */
  --primary-foreground: oklch(0.985 0 0); /* Near white */
  --muted: oklch(0.97 0 0); /* Light gray */
  --muted-foreground: oklch(0.556 0 0); /* Medium gray */
  --border: oklch(0.922 0 0); /* Light border */
  --destructive: oklch(0.577 0.245 27.325); /* Red */
}
```

### Dark Theme

```css
.dark {
  --background: oklch(0.145 0 0); /* Near black */
  --foreground: oklch(0.985 0 0); /* Near white */
  --primary: oklch(0.922 0 0); /* Light gray */
  --card: oklch(0.205 0 0); /* Dark card */
  --muted: oklch(0.269 0 0); /* Dark muted */
}
```

### Status Colors

| Status         | Color             | Usage                  |
| -------------- | ----------------- | ---------------------- |
| Connected      | `text-green-500`  | API status indicator   |
| Disconnected   | `text-red-500`    | API offline indicator  |
| Checking       | `text-yellow-500` | Loading/checking state |
| Primary Action | `bg-primary`      | Main buttons           |
| Destructive    | `bg-destructive`  | Delete actions         |

### Entity Type Colors (Recommended)

| Entity Type  | Suggested Color     |
| ------------ | ------------------- |
| Person       | `#8B5CF6` (Violet)  |
| Organization | `#06B6D4` (Cyan)    |
| Location     | `#10B981` (Emerald) |
| Concept      | `#F59E0B` (Amber)   |
| Document     | `#6366F1` (Indigo)  |

---

## Border Radius

| Token           | Value  | Tailwind       | Usage                 |
| --------------- | ------ | -------------- | --------------------- |
| `--radius-sm`   | 6px    | `rounded-md`   | Badges, small buttons |
| `--radius-md`   | 8px    | `rounded-lg`   | Buttons, inputs       |
| `--radius-lg`   | 10px   | `rounded-xl`   | Cards, modals         |
| `--radius-xl`   | 14px   | `rounded-2xl`  | Large cards, panels   |
| `--radius-2xl`  | 18px   | `rounded-3xl`  | Hero elements         |
| `--radius-full` | 9999px | `rounded-full` | Avatars, pills        |

### Current Implementation

```css
:root {
  --radius: 0.625rem; /* 10px base */
}
```

---

## Animation Tokens

| Token               | Value                          | Usage                            |
| ------------------- | ------------------------------ | -------------------------------- |
| `--duration-fast`   | 150ms                          | Micro-interactions, hover states |
| `--duration-normal` | 250ms                          | UI state changes, panel toggle   |
| `--duration-slow`   | 400ms                          | Page transitions, modals         |
| `--ease-out`        | `cubic-bezier(0, 0, 0.2, 1)`   | Entrances, expand                |
| `--ease-in`         | `cubic-bezier(0.4, 0, 1, 1)`   | Exits, collapse                  |
| `--ease-in-out`     | `cubic-bezier(0.4, 0, 0.2, 1)` | Morphs, resizing                 |

### Current Implementation

```tsx
// sidebar.tsx
"transition-all duration-300"; // 300ms - slightly slow, recommend 250ms

// right-panel.tsx
"transition-all duration-300 ease-in-out"; // Good
```

### Recommendations

```css
/* Add shimmer animation for loading states */
@keyframes shimmer {
  from {
    transform: translateX(-100%);
  }
  to {
    transform: translateX(100%);
  }
}

.animate-shimmer {
  animation: shimmer 1.5s infinite;
}
```

---

## Shadow Tokens

| Token         | Value                               | Usage            |
| ------------- | ----------------------------------- | ---------------- |
| `--shadow-sm` | `0 1px 2px 0 rgb(0 0 0 / 0.05)`     | Subtle elevation |
| `--shadow-md` | `0 4px 6px -1px rgb(0 0 0 / 0.1)`   | Cards, dropdowns |
| `--shadow-lg` | `0 10px 15px -3px rgb(0 0 0 / 0.1)` | Modals, popovers |
| `--shadow-xl` | `0 20px 25px -5px rgb(0 0 0 / 0.1)` | Large overlays   |

### Recommendations

- Add `shadow-sm` to cards on hover
- Use `shadow-lg` for modals and dialogs
- Reduce shadow intensity in dark mode

---

## Z-Index Scale

| Token          | Value | Usage                    |
| -------------- | ----- | ------------------------ |
| `--z-base`     | 0     | Base content             |
| `--z-dropdown` | 10    | Dropdowns, popovers      |
| `--z-sticky`   | 20    | Sticky headers, sidebars |
| `--z-fixed`    | 30    | Fixed elements           |
| `--z-modal`    | 40    | Modal overlays           |
| `--z-toast`    | 50    | Notifications, toasts    |
| `--z-tooltip`  | 60    | Tooltips                 |

---

## Component Tokens

### Button

```css
.btn {
  --btn-height: 40px;
  --btn-height-sm: 32px;
  --btn-height-lg: 48px;
  --btn-padding-x: 16px;
  --btn-border-radius: var(--radius-md);
  --btn-font-weight: 500;
}
```

### Input

```css
.input {
  --input-height: 40px;
  --input-padding-x: 12px;
  --input-border-radius: var(--radius-md);
  --input-border-width: 1px;
}
```

### Card

```css
.card {
  --card-padding: 24px;
  --card-border-radius: var(--radius-lg);
  --card-border-width: 1px;
  --card-shadow: var(--shadow-sm);
}
```

---

## Touch Targets

| Element         | Minimum Size | Current | Status           |
| --------------- | ------------ | ------- | ---------------- |
| Buttons         | 44×44px      | 40×40px | ⚠️ Needs review  |
| Nav Items       | 44×44px      | 48×48px | ✅ Good          |
| Icon Buttons    | 44×44px      | 40×40px | ⚠️ Needs review  |
| Toggle Switches | 44×44px      | 36×20px | ⚠️ Needs wrapper |

### Recommendations

- Add touch-target wrapper class for mobile
- Increase icon button sizes on touch devices
- Use CSS to extend hit areas without changing visual size

---

## Implementation Guide

### Using Design Tokens in CSS

```css
/* globals.css */
:root {
  /* Typography */
  --font-sans: var(--font-geist-sans);
  --font-mono: var(--font-geist-mono);

  /* Spacing */
  --space-page: 24px;

  /* Panels */
  --sidebar-width: 256px;
  --sidebar-collapsed: 64px;

  /* Animation */
  --duration-normal: 250ms;
  --ease-out: cubic-bezier(0, 0, 0.2, 1);
}

/* Utility classes */
.p-page {
  padding: var(--space-page);
}
.transition-panel {
  transition: all var(--duration-normal) var(--ease-out);
}
```

### Using Design Tokens in Tailwind

```js
// tailwind.config.js
module.exports = {
  theme: {
    extend: {
      width: {
        sidebar: "256px",
        "sidebar-collapsed": "64px",
      },
      transitionDuration: {
        normal: "250ms",
      },
    },
  },
};
```

---

## Changelog

| Date       | Version | Changes                            |
| ---------- | ------- | ---------------------------------- |
| 2025-12-25 | 1.0.0   | Initial design token documentation |

---

_These tokens should be implemented as CSS custom properties and/or Tailwind extensions for consistent usage across the application._
