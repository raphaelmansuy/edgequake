# 001 — Contrast & Color Audit

**First Principle: Clarity** — If you can't read it, it doesn't exist.

---

## Color System Overview

The app uses **OKLCH** color space (via Tailwind v4), which is perceptually uniform and excellent for generating accessible color scales. However, the current semantic token values need calibration.

### Light Mode Contrast Matrix

| Token | OKLCH Value | Approx Hex | On White Contrast | WCAG AA (4.5:1) |
|-------|-------------|------------|-------------------|-----------------|
| `--foreground` | oklch(0.145 0 0) | #1a1a1a | ~14:1 | ✅ Pass |
| `--primary` | oklch(0.205 0 0) | #252525 | ~11:1 | ✅ Pass |
| `--muted-foreground` | oklch(0.556 0 0) | #737373 | ~4.2:1 | ⚠️ Borderline |
| `--border` | oklch(0.922 0 0) | #ebebeb | ~1.3:1 | ❌ Fail (border) |
| `--input` | oklch(0.922 0 0) | #ebebeb | ~1.3:1 | ❌ Fail (input border) |

### Dark Mode Contrast Matrix

| Token | OKLCH Value | On Dark BG Contrast | WCAG AA |
|-------|-------------|---------------------|---------|
| `--foreground` | oklch(0.985 0 0) | ~18:1 | ✅ Pass |
| `--muted-foreground` | oklch(0.708 0 0) | ~5.4:1 | ✅ Pass |
| `--border` | oklch(0.269 0 0) | ~1.8:1 | ⚠️ Low border contrast |

---

## Critical Issues

### CC-01 · `muted-foreground` Borderline Contrast at Small Sizes

`oklch(0.556)` on white gives ~4.2:1 — this passes WCAG AA for **normal text** (≥14px, non-bold) but **fails for text smaller than 14px**.

Current uses of `muted-foreground` on text ≤ 12px:
- Stats card description text (`text-xs`)
- Table metadata (cost, date columns)
- Sidebar nav group labels (proposed)
- Document upload progress percentages
- Chat message timestamps

**Fix:** Darken `muted-foreground` to achieve 4.5:1 at all sizes:

```css
/* globals.css */
:root {
  /* CURRENT: --muted-foreground: oklch(0.556 0 0); (~4.2:1 on white) */
  --muted-foreground: oklch(0.48 0 0); /* ~5.7:1 on white — safe for all sizes */
}

.dark {
  /* Dark mode is already fine: oklch(0.708) → ~5.4:1 on dark bg */
  --muted-foreground: oklch(0.708 0 0); /* keep as-is */
}
```

### CC-02 · Input Border Fails WCAG 1.4.11 (Non-text Contrast)

```css
--input: oklch(0.922 0 0);  /* ~1.4:1 on white background */
--border: oklch(0.922 0 0); /* Same — table dividers also barely visible */
```

WCAG 1.4.11 requires **3:1** contrast for UI components (input borders, button outlines, focus indicators).

**Fix:**

```css
:root {
  --input: oklch(0.78 0 0);  /* ~3.1:1 on white */
  --border: oklch(0.82 0 0); /* ~2.5:1 — dividers can be slightly lighter than inputs */
}
```

Note: This will visibly darken all input borders and dividers — this is intentional and creates cleaner definition.

### CC-03 · Status Color Overload (12 Color Variants)

The document status badge system uses 12 distinct color categories:

```
blue     → uploading
amber    → queued
indigo   → converting
blue     → preprocessing  ← SAME as uploading (different shade?)
blue     → chunking       ← SAME family
purple   → extracting
purple   → gleaning       ← SAME as extracting
amber    → merging        ← SAME as queued
orange   → summarizing
cyan     → embedding
teal     → storing
green    → completed
red      → failed
orange   → partial_failure ← SAME as summarizing
amber    → partial_success ← SAME as queued/merging
```

**Problems:**
1. Blue appears 3 times (uploading, preprocessing, chunking)
2. Amber appears 3 times (queued, merging, partial_success)
3. Purple appears twice (extracting, gleaning)
4. Orange appears twice (summarizing, partial_failure)

Users cannot reliably distinguish between processing sub-states by color alone. This violates WCAG 1.4.1 (Use of Color).

**Solution: Reduce to a semantic color system**

```
Category          Color     States
─────────────────────────────────────────────────────────
Pending/Waiting   Amber     queued
In Progress       Blue      uploading, preprocessing, chunking
                            converting, embedding, storing
AI Processing     Purple    extracting, gleaning, merging, summarizing
Terminal/Success  Green     completed, partial_success
Terminal/Error    Red       failed
Terminal/Warn     Orange    partial_failure
```

This reduces unique colors from 7+ to 4 semantic categories. Within "In Progress," differentiate by **icon** (already defined in `statusConfig`) not by hue variation.

**Updated statusConfig:**

```typescript
// Semantic color groups
const IN_PROGRESS_STYLE = 'text-blue-600 dark:text-blue-400';
const AI_STYLE = 'text-purple-600 dark:text-purple-400';
const SUCCESS_STYLE = 'text-green-600 dark:text-green-400';
const ERROR_STYLE = 'text-red-600 dark:text-red-400';
const WARNING_STYLE = 'text-orange-600 dark:text-orange-400';
const PENDING_STYLE = 'text-amber-600 dark:text-amber-400';

const statusConfig = {
  queued:          { icon: Clock,     color: PENDING_STYLE, label: 'Queued', animate: true },
  uploading:       { icon: Upload,    color: IN_PROGRESS_STYLE, label: 'Uploading', animate: true },
  converting:      { icon: FileText,  color: IN_PROGRESS_STYLE, label: 'Converting', animate: true },
  preprocessing:   { icon: Loader2,   color: IN_PROGRESS_STYLE, label: 'Processing', animate: true },
  chunking:        { icon: Scissors,  color: IN_PROGRESS_STYLE, label: 'Chunking', animate: true },
  extracting:      { icon: Brain,     color: AI_STYLE, label: 'Extracting', animate: true },
  gleaning:        { icon: Search,    color: AI_STYLE, label: 'Refining', animate: true },
  merging:         { icon: GitMerge,  color: AI_STYLE, label: 'Merging', animate: true },
  summarizing:     { icon: FileText,  color: AI_STYLE, label: 'Summarizing', animate: true },
  embedding:       { icon: Cpu,       color: IN_PROGRESS_STYLE, label: 'Embedding', animate: true },
  storing:         { icon: Database,  color: IN_PROGRESS_STYLE, label: 'Storing', animate: true },
  completed:       { icon: CheckCircle, color: SUCCESS_STYLE, label: 'Completed', animate: false },
  failed:          { icon: XCircle,   color: ERROR_STYLE, label: 'Failed', animate: false },
  partial_failure: { icon: AlertTriangle, color: WARNING_STYLE, label: 'Partial failure', animate: false },
  partial_success: { icon: CheckCircle, color: WARNING_STYLE, label: 'Partial', animate: false },
};
```

### CC-04 · Hard-coded Color Classes in Components

Multiple components use hard-coded Tailwind color classes that don't participate in the theme system:

```typescript
// document-table-row.tsx
case 'pdf': return { icon: FileText, color: 'text-red-500' };
case 'doc': return { icon: FileType, color: 'text-blue-500' };

// quick-actions.tsx
color: 'text-blue-500',
bgColor: 'bg-blue-500/10 hover:bg-blue-500/20',
```

These won't respect custom themes or high-contrast modes. While not a critical WCAG failure, it's a design system hygiene issue.

**Fix:** Define semantic semantic palette tokens for UI accent colors:

```css
/* design-tokens.css */
:root {
  --color-docs: oklch(0.55 0.15 220);     /* Blue — documents */
  --color-entities: oklch(0.55 0.15 145); /* Green — entities */
  --color-graph: oklch(0.55 0.12 280);    /* Purple — graph */
  --color-query: oklch(0.55 0.15 45);     /* Amber — query/AI */
}
```

---

## Focus Indicator Audit

### CC-05 · Focus Ring Contrast

The current focus ring uses `focus-visible:ring-2 focus-visible:ring-primary`:
- Light mode: `--ring: oklch(0.708 0 0)` — gray ring on white background ≈ 2.7:1 — **fails WCAG 1.4.11**
- The focus ring needs 3:1 contrast against adjacent colors

**Fix:**

```css
/* Use primary color for focus ring, not --ring which is gray */
:root {
  --ring: oklch(0.205 0 0); /* Use primary (near-black) for focus ring */
}

/* Or in Tailwind config: */
/* focus-visible:ring-primary instead of focus-visible:ring-ring */
```

---

## Color Reference Charts

### Semantic Color Map

```
OKLCH Lightness Guide:
─────────────────────────────────────────────────────
L=0.95+   Background surfaces (muted, cards)
L=0.70-0.95  Borders, separators, subtle backgrounds
L=0.50-0.70  Muted text (must be ≥0.48 for AA)
L=0.20-0.50  Body text, labels
L=0.00-0.20  Foreground, headings (maximum contrast)
```

### Status Color Grid (Proposed)

```
PROCESSING STATES (Blue family)
┌────────────────────────────────────────────────────┐
│  ⬤ blue-500 dark:blue-400  │ Uploading             │
│  ⚡ icon varies             │ Converting/Chunking   │
│  💾 icon varies             │ Embedding/Storing     │
├────────────────────────────────────────────────────┤
│  🧠 purple-500 dark:purple-400 │ AI Processing     │
│  Extracting / Gleaning / Merging / Summarizing     │
├────────────────────────────────────────────────────┤
│  ✓ green-600 dark:green-400 │ Completed             │
│  ✕ red-600 dark:red-400    │ Failed                │
│  △ orange-600 dark:orange-400│ Partial failure      │
│  ⏳ amber-600 dark:amber-400 │ Queued               │
└────────────────────────────────────────────────────┘
```

---

## External References

- [OKLCH Color Picker — oklch.com](https://oklch.com/)
- [WCAG 1.4.1 Use of Color](https://www.w3.org/WAI/WCAG21/Understanding/use-of-color)
- [WCAG 1.4.3 Contrast (Minimum)](https://www.w3.org/WAI/WCAG21/Understanding/contrast-minimum)
- [WCAG 1.4.11 Non-text Contrast](https://www.w3.org/WAI/WCAG21/Understanding/non-text-contrast)
- [APCA Contrast — Accessible Perceptual Contrast Algorithm](https://www.myndex.com/APCA/)
- [ColorBox by Lyft Design](https://colorbox.io/)
- [Who Can Use — Contrast checker](https://www.whocanuse.com/)
