# SPEC-WEBSITE-06: Design System

> **Status**: DRAFT  
> **Created**: 2026-03-21  
> **Parent**: [00-overview.md](./00-overview.md)  
> **Related**: [03-page-specifications.md](./03-page-specifications.md) · [07-component-library.md](./07-component-library.md)

---

## 1. Design Principles

<<<<<<< HEAD
| Principle | Description |
|-----------|------------|
| **Clarity First** | Information density over decoration. Every element earns its place. |
| **Developer Trust** | Monospace code, real examples, no marketing fluff. |
| **Speed Signals** | The site itself must feel fast — proving EdgeQuake's performance ethos. |
| **Accessible by Default** | WCAG AA compliant. Dark mode native. Reduced motion respected. |
| **Modular** | Design tokens → components → sections → pages. Swap any layer. |
=======
| Principle                 | Description                                                             |
| ------------------------- | ----------------------------------------------------------------------- |
| **Clarity First**         | Information density over decoration. Every element earns its place.     |
| **Developer Trust**       | Monospace code, real examples, no marketing fluff.                      |
| **Speed Signals**         | The site itself must feel fast — proving EdgeQuake's performance ethos. |
| **Accessible by Default** | WCAG AA compliant. Dark mode native. Reduced motion respected.          |
| **Modular**               | Design tokens → components → sections → pages. Swap any layer.          |
>>>>>>> origin/edgequake-main

---

## 2. Color Palette

### 2.1 Semantic Colors

```
┌─────────────────── LIGHT MODE ───────────────────┐
│                                                    │
│  Background       #FFFFFF  (white)                 │
│  Background Alt   #F8F9FA  (gray-50)              │
│  Surface          #F1F3F5  (gray-100)             │
│  Border           #DEE2E6  (gray-300)             │
│  Text Primary     #1A1A2E  (near-black)           │
│  Text Secondary   #6C757D  (gray-600)             │
│  Text Muted       #ADB5BD  (gray-500)             │
│                                                    │
└────────────────────────────────────────────────────┘

┌─────────────────── DARK MODE ────────────────────┐
│                                                    │
│  Background       #0A0A0F  (near-black)           │
│  Background Alt   #111118  (dark surface)         │
│  Surface          #1A1A24  (dark card)            │
│  Border           #2A2A3A  (dark border)          │
│  Text Primary     #F8F9FA  (near-white)           │
│  Text Secondary   #ADB5BD  (gray-500)             │
│  Text Muted       #6C757D  (gray-600)             │
│                                                    │
└────────────────────────────────────────────────────┘
```

### 2.2 Brand Colors

```
┌─────────────────── BRAND ────────────────────────┐
│                                                    │
│  Primary          #3B82F6  (blue-500)    → CTAs   │
│  Primary Hover    #2563EB  (blue-600)             │
│  Primary Light    #DBEAFE  (blue-100)   → badges  │
│                                                    │
│  Accent           #8B5CF6  (violet-500) → graphs  │
│  Accent Light     #EDE9FE  (violet-100)           │
│                                                    │
│  Success          #10B981  (emerald-500) → badges  │
│  Warning          #F59E0B  (amber-500)   → notes   │
│  Error            #EF4444  (red-500)     → alerts   │
│                                                    │
└────────────────────────────────────────────────────┘
```

### 2.3 Gradient

```
Hero gradient:  linear-gradient(135deg, #3B82F6, #8B5CF6)
Card highlight: linear-gradient(135deg, #3B82F650, #8B5CF620)
```

### 2.4 CSS Variables (Tailwind + shadcn)

```css
/* globals.css */
@layer base {
  :root {
    --background: 0 0% 100%;
    --foreground: 230 33% 14%;
    --card: 210 17% 95%;
    --card-foreground: 230 33% 14%;
    --primary: 217 91% 60%;
    --primary-foreground: 0 0% 100%;
    --secondary: 210 11% 96%;
    --secondary-foreground: 230 33% 14%;
    --muted: 210 11% 96%;
    --muted-foreground: 210 7% 56%;
    --accent: 258 90% 66%;
    --accent-foreground: 0 0% 100%;
    --destructive: 0 84% 60%;
    --destructive-foreground: 0 0% 100%;
    --border: 210 14% 89%;
    --ring: 217 91% 60%;
    --radius: 0.5rem;
  }

  .dark {
    --background: 240 20% 4%;
    --foreground: 210 17% 98%;
    --card: 240 14% 10%;
    --card-foreground: 210 17% 98%;
    --primary: 217 91% 60%;
    --primary-foreground: 0 0% 100%;
    --secondary: 240 14% 14%;
    --secondary-foreground: 210 17% 98%;
    --muted: 240 14% 14%;
    --muted-foreground: 210 7% 56%;
    --accent: 258 90% 66%;
    --accent-foreground: 0 0% 100%;
    --destructive: 0 84% 60%;
    --destructive-foreground: 0 0% 100%;
    --border: 240 10% 20%;
    --ring: 217 91% 60%;
  }
}
```

---

## 3. Typography

### 3.1 Font Stack

```
--font-sans:  "Inter", system-ui, -apple-system, sans-serif;
--font-mono:  "JetBrains Mono", "Fira Code", ui-monospace, monospace;
```

<<<<<<< HEAD
| Usage | Font | Weight | Tracking |
|-------|------|--------|----------|
| Headlines | Inter | 700 (bold) | -0.02em (tight) |
| Body | Inter | 400 (regular) | 0 (normal) |
| Code / badges | JetBrains Mono | 400 | 0 |
| Labels / captions | Inter | 500 (medium) | 0.05em (wide) |
=======
| Usage             | Font           | Weight        | Tracking        |
| ----------------- | -------------- | ------------- | --------------- |
| Headlines         | Inter          | 700 (bold)    | -0.02em (tight) |
| Body              | Inter          | 400 (regular) | 0 (normal)      |
| Code / badges     | JetBrains Mono | 400           | 0               |
| Labels / captions | Inter          | 500 (medium)  | 0.05em (wide)   |
>>>>>>> origin/edgequake-main

### 3.2 Type Scale

```
┌─── Type Scale (rem / px) ─────────────────────────┐
│                                                     │
│  text-xs     0.75rem   / 12px   line-height: 1rem  │
│  text-sm     0.875rem  / 14px   line-height: 1.25  │
│  text-base   1rem      / 16px   line-height: 1.5   │
│  text-lg     1.125rem  / 18px   line-height: 1.75  │
│  text-xl     1.25rem   / 20px   line-height: 1.75  │
│  text-2xl    1.5rem    / 24px   line-height: 2      │
│  text-3xl    1.875rem  / 30px   line-height: 2.25   │
│  text-4xl    2.25rem   / 36px   line-height: 2.5    │
│  text-5xl    3rem      / 48px   line-height: 1.1    │
│  text-6xl    3.75rem   / 60px   line-height: 1.1    │
│                                                     │
└─────────────────────────────────────────────────────┘
```

### 3.3 Usage Map

<<<<<<< HEAD
| Element | Size | Weight | Color |
|---------|------|--------|-------|
| Page H1 (hero) | text-5xl / sm:text-6xl | bold | foreground |
| Section H2 | text-3xl / sm:text-4xl | bold | foreground |
| Card H3 | text-xl | semibold | foreground |
| Body paragraph | text-base / text-lg | regular | muted-foreground |
| Code inline | text-sm | mono 400 | foreground on muted bg |
| Badge text | text-xs | mono 500 | primary |
| Navigation link | text-sm | medium | muted-foreground → foreground on hover |
| Footer text | text-sm | regular | muted-foreground |
=======
| Element         | Size                   | Weight   | Color                                  |
| --------------- | ---------------------- | -------- | -------------------------------------- |
| Page H1 (hero)  | text-5xl / sm:text-6xl | bold     | foreground                             |
| Section H2      | text-3xl / sm:text-4xl | bold     | foreground                             |
| Card H3         | text-xl                | semibold | foreground                             |
| Body paragraph  | text-base / text-lg    | regular  | muted-foreground                       |
| Code inline     | text-sm                | mono 400 | foreground on muted bg                 |
| Badge text      | text-xs                | mono 500 | primary                                |
| Navigation link | text-sm                | medium   | muted-foreground → foreground on hover |
| Footer text     | text-sm                | regular  | muted-foreground                       |
>>>>>>> origin/edgequake-main

---

## 4. Spacing System

Uses Tailwind's default 4px base grid:

```
┌─── Spacing Scale ─────────────────────────────────┐
│                                                     │
│  space-0     0                                      │
│  space-1     0.25rem  (4px)    → inline padding     │
│  space-2     0.5rem   (8px)    → tight gaps          │
│  space-3     0.75rem  (12px)   → icon gaps           │
│  space-4     1rem     (16px)   → card padding        │
│  space-6     1.5rem   (24px)   → section gap         │
│  space-8     2rem     (32px)   → between elements    │
│  space-12    3rem     (48px)   → section dividers     │
│  space-16    4rem     (64px)   → section padding      │
│  space-20    5rem     (80px)   → major section gap     │
│  space-24    6rem     (96px)   → hero vertical pad     │
│                                                     │
└─────────────────────────────────────────────────────┘
```

### Spacing Application

<<<<<<< HEAD
| Context | Value |
|---------|-------|
| Section top/bottom padding | `py-16 md:py-24` (64px / 96px) |
| Card internal padding | `p-6` (24px) |
| Grid gap | `gap-6 md:gap-8` (24px / 32px) |
| Inline element spacing | `gap-2 md:gap-3` (8px / 12px) |
| Container max-width | `max-w-7xl` (1280px) |
| Container padding | `px-4 md:px-6 lg:px-8` |
=======
| Context                    | Value                          |
| -------------------------- | ------------------------------ |
| Section top/bottom padding | `py-16 md:py-24` (64px / 96px) |
| Card internal padding      | `p-6` (24px)                   |
| Grid gap                   | `gap-6 md:gap-8` (24px / 32px) |
| Inline element spacing     | `gap-2 md:gap-3` (8px / 12px)  |
| Container max-width        | `max-w-7xl` (1280px)           |
| Container padding          | `px-4 md:px-6 lg:px-8`         |
>>>>>>> origin/edgequake-main

---

## 5. Layout Grid

```
┌─── Container Layout ──────────────────────────────┐
│                                                     │
│  ← px-4 →┌─── max-w-7xl (1280px) ─────┐← px-4 → │
│           │                            │           │
│           │  12-column grid            │           │
│           │  gap-6 md:gap-8            │           │
│           │                            │           │
│           └────────────────────────────┘           │
│                                                     │
└─────────────────────────────────────────────────────┘
```

<<<<<<< HEAD
| Grid Config | Value |
|-------------|-------|
| Columns (base) | 1 (mobile) |
| Columns (md) | 2 |
| Columns (lg) | 3 |
| Feature grid (lg) | `grid-cols-3` |
| Docs layout | Sidebar 240px + Content flex-1 + TOC 200px |
=======
| Grid Config       | Value                                      |
| ----------------- | ------------------------------------------ |
| Columns (base)    | 1 (mobile)                                 |
| Columns (md)      | 2                                          |
| Columns (lg)      | 3                                          |
| Feature grid (lg) | `grid-cols-3`                              |
| Docs layout       | Sidebar 240px + Content flex-1 + TOC 200px |
>>>>>>> origin/edgequake-main

---

## 6. Border & Radius

<<<<<<< HEAD
| Token | Value | Usage |
|-------|-------|-------|
| `rounded-sm` | 0.25rem / 4px | Badges, inline code |
| `rounded-md` | 0.375rem / 6px | Buttons, inputs |
| `rounded-lg` | 0.5rem / 8px | Cards, modals |
| `rounded-xl` | 0.75rem / 12px | Hero sections, featured cards |
| `border` | 1px solid hsl(var(--border)) | Cards, separators |
=======
| Token        | Value                        | Usage                         |
| ------------ | ---------------------------- | ----------------------------- |
| `rounded-sm` | 0.25rem / 4px                | Badges, inline code           |
| `rounded-md` | 0.375rem / 6px               | Buttons, inputs               |
| `rounded-lg` | 0.5rem / 8px                 | Cards, modals                 |
| `rounded-xl` | 0.75rem / 12px               | Hero sections, featured cards |
| `border`     | 1px solid hsl(var(--border)) | Cards, separators             |
>>>>>>> origin/edgequake-main

---

## 7. Shadow & Elevation

```
┌─── Elevation Levels ──────────────────────────────┐
│                                                     │
│  Level 0: No shadow (flat elements, most content)   │
│                                                     │
│  Level 1: shadow-sm                                 │
│           0 1px 2px rgba(0,0,0,0.05)               │
│           → Cards at rest                           │
│                                                     │
│  Level 2: shadow-md                                 │
│           0 4px 6px rgba(0,0,0,0.07)               │
│           → Cards on hover, dropdowns               │
│                                                     │
│  Level 3: shadow-lg                                 │
│           0 10px 15px rgba(0,0,0,0.1)              │
│           → Modals, floating menus                  │
│                                                     │
└─────────────────────────────────────────────────────┘
```

---

## 8. Dark Mode

### 8.1 Implementation

Uses `class` strategy (Tailwind `darkMode: "class"`):

```typescript
// Theme toggle using next-themes
import { ThemeProvider } from "next-themes";

<ThemeProvider attribute="class" defaultTheme="system">
  {children}
</ThemeProvider>
```

### 8.2 Color Mapping

<<<<<<< HEAD
| Element | Light | Dark |
|---------|-------|------|
| Page background | `#FFFFFF` | `#0A0A0F` |
| Card background | `#F1F3F5` | `#1A1A24` |
| Text primary | `#1A1A2E` | `#F8F9FA` |
| Code block bg | `#F8F9FA` | `#111118` |
| Border | `#DEE2E6` | `#2A2A3A` |
| Primary button | Blue on white | Blue on dark |
| Graph nodes | Blue/violet filled | Blue/violet filled (brighter) |
=======
| Element         | Light              | Dark                          |
| --------------- | ------------------ | ----------------------------- |
| Page background | `#FFFFFF`          | `#0A0A0F`                     |
| Card background | `#F1F3F5`          | `#1A1A24`                     |
| Text primary    | `#1A1A2E`          | `#F8F9FA`                     |
| Code block bg   | `#F8F9FA`          | `#111118`                     |
| Border          | `#DEE2E6`          | `#2A2A3A`                     |
| Primary button  | Blue on white      | Blue on dark                  |
| Graph nodes     | Blue/violet filled | Blue/violet filled (brighter) |
>>>>>>> origin/edgequake-main

### 8.3 Rules

- Default to system preference (`prefers-color-scheme`)
- Store user preference in `localStorage`
- No flash of wrong theme on load (next-themes handles this)
- All custom illustrations must work on both backgrounds
- Code blocks: Shiki `github-light` / `github-dark` themes

---

## 9. Motion & Animation

### 9.1 Framer Motion Presets

```typescript
// Shared animation presets
export const fadeIn = {
  initial: { opacity: 0, y: 20 },
  animate: { opacity: 1, y: 0 },
  transition: { duration: 0.5, ease: "easeOut" },
};

export const staggerContainer = {
  animate: { transition: { staggerChildren: 0.1 } },
};

export const slideInLeft = {
  initial: { opacity: 0, x: -30 },
  animate: { opacity: 1, x: 0 },
  transition: { duration: 0.5 },
};
```

### 9.2 Animation Rules

<<<<<<< HEAD
| Rule | Detail |
|------|--------|
| Respect `prefers-reduced-motion` | Disable all animations if set |
| Duration | 200–500ms for UI, 500–1000ms for scroll reveals |
| Easing | `easeOut` for entrances, `easeInOut` for transitions |
| Scroll trigger | Intersection Observer at 20% visibility |
| No layout shift | Animate `opacity` and `transform` only |
| Hero animation | Knowledge graph nodes pulse at 3s interval |
| Benchmark bars | Width transitions from 0% to target on scroll |
=======
| Rule                             | Detail                                               |
| -------------------------------- | ---------------------------------------------------- |
| Respect `prefers-reduced-motion` | Disable all animations if set                        |
| Duration                         | 200–500ms for UI, 500–1000ms for scroll reveals      |
| Easing                           | `easeOut` for entrances, `easeInOut` for transitions |
| Scroll trigger                   | Intersection Observer at 20% visibility              |
| No layout shift                  | Animate `opacity` and `transform` only               |
| Hero animation                   | Knowledge graph nodes pulse at 3s interval           |
| Benchmark bars                   | Width transitions from 0% to target on scroll        |
>>>>>>> origin/edgequake-main

---

## 10. Iconography

### 10.1 Icon Set

Primary: **Lucide React** (MIT, 1000+ icons, tree-shakeable)

### 10.2 Icon Usage

<<<<<<< HEAD
| Context | Size | Stroke Width |
|---------|------|-------------|
| Navigation | 20px | 2px |
| Feature cards | 24px | 1.5px |
| Inline body text | 16px | 2px |
| Hero badges | 16px | 2px |

### 10.3 Custom Icons

| Icon | Purpose | Format |
|------|---------|--------|
| EdgeQuake logo | Brand | SVG (inline) |
| Knowledge graph | Feature icon | SVG (custom) |
| Crate box | Ecosystem | SVG (custom, derived from Lucide `package`) |
=======
| Context          | Size | Stroke Width |
| ---------------- | ---- | ------------ |
| Navigation       | 20px | 2px          |
| Feature cards    | 24px | 1.5px        |
| Inline body text | 16px | 2px          |
| Hero badges      | 16px | 2px          |

### 10.3 Custom Icons

| Icon            | Purpose      | Format                                      |
| --------------- | ------------ | ------------------------------------------- |
| EdgeQuake logo  | Brand        | SVG (inline)                                |
| Knowledge graph | Feature icon | SVG (custom)                                |
| Crate box       | Ecosystem    | SVG (custom, derived from Lucide `package`) |
>>>>>>> origin/edgequake-main

---

## 11. Brand Assets

<<<<<<< HEAD
| Asset | Format | Sizes |
|-------|--------|-------|
| Logo (full) | SVG | — |
| Logo (mark only) | SVG | 32px, 64px, 128px |
| Favicon | ICO + PNG | 16, 32, 180, 192, 512 |
| OG background | PNG | 1200x630 |
| Social banner | PNG | 1500x500 (Twitter) |
=======
| Asset            | Format    | Sizes                 |
| ---------------- | --------- | --------------------- |
| Logo (full)      | SVG       | —                     |
| Logo (mark only) | SVG       | 32px, 64px, 128px     |
| Favicon          | ICO + PNG | 16, 32, 180, 192, 512 |
| OG background    | PNG       | 1200x630              |
| Social banner    | PNG       | 1500x500 (Twitter)    |
>>>>>>> origin/edgequake-main

### Logo Lockup

```
  🔷 EdgeQuake        ← Logo mark + wordmark (horizontal)

  🔷                  ← Logo mark only (square, for favicons)
  EdgeQuake
```

---

<<<<<<< HEAD
*Previous: [05-seo-strategy.md](./05-seo-strategy.md) · Next: [07-component-library.md](./07-component-library.md)*
=======
_Previous: [05-seo-strategy.md](./05-seo-strategy.md) · Next: [07-component-library.md](./07-component-library.md)_
>>>>>>> origin/edgequake-main
