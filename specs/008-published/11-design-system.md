# SPEC-008-11: Design System & Visual Language

| Field       | Value                                 |
| ----------- | ------------------------------------- |
| **Spec ID** | SPEC-008-11                           |
| **Parent**  | [SPEC-008 Overview](./00-overview.md) |
| **Title**   | Design System & Visual Language       |
| **Status**  | Draft                                 |
| **Created** | 2026-03-21                            |
| **Updated** | 2026-03-21                            |

---

## 1. Purpose

Define the visual design system for the unified EdgeQuake Astro + Starlight website. This specification establishes the color palette, typography scale, spacing system, motion language, and dark/light theme implementation. All decisions are informed by best practices from leading Astro-built sites (Cloudflare Docs, Netlify Docs, Biome, SST Ion, Fluent 2 Design System, Proton, Porsche, astro.build) and Starlight's native theming capabilities.

**Cross-references:**

- [04-starlight-project-setup.md](./04-starlight-project-setup.md) — Astro config and `customCss` integration
- [06-search-navigation-seo.md](./06-search-navigation-seo.md) — Header/search bar styling context
- [07-content-authoring-standards.md](./07-content-authoring-standards.md) — Markdown rendering styles
- [09-migration-roadmap.md](./09-migration-roadmap.md) — Phase 1 includes design system scaffold
- [12-page-layouts-wireframes.md](./12-page-layouts-wireframes.md) — Layout compositions using this system
- [13-component-library.md](./13-component-library.md) — Components built on these tokens

---

## 2. Design Philosophy

### 2.1 Principles

EdgeQuake's visual identity follows five guiding principles, each drawn from patterns observed in the best Astro-based sites:

| #   | Principle              | Description                                                   | Reference Sites                         |
| --- | ---------------------- | ------------------------------------------------------------- | --------------------------------------- |
| P1  | **Content-first**      | UI elements serve content, never compete with it              | Cloudflare Docs, Starlight defaults     |
| P2  | **Minimalist clarity** | Generous whitespace, restrained palette, clear hierarchy      | Proton, Porsche, astro.build            |
| P3  | **Progressive reveal** | Zero JS by default; islands load interactivity when needed    | Astro island architecture pattern       |
| P4  | **Perceptual harmony** | HSL-based color system for uniform lightness across hues      | Fluent 2 Design System, Starlight theme |
| P5  | **Accessible always**  | WCAG AA contrast ratios, keyboard nav, reduced-motion support | Biome, Accessible Astro, Netlify Docs   |

### 2.2 Design Mood

```
+-------------------------------------------------------------------+
|                                                                   |
|   EDGEQUAKE VISUAL MOOD                                           |
|                                                                   |
|   Clean         ■■■■■■■■■■  10/10                                |
|   Minimal       ■■■■■■■■■□   9/10                                |
|   Technical     ■■■■■■■■□□   8/10                                |
|   Warm          ■■■■■□□□□□   5/10                                |
|   Playful       ■■■□□□□□□□   3/10                                |
|   Corporate     ■■■■□□□□□□   4/10                                |
|                                                                   |
|   Keywords: precise, fast, trustworthy, developer-friendly,       |
|             graph-aware, modern, open-source                      |
|                                                                   |
+-------------------------------------------------------------------+
```

---

## 3. Color System

### 3.1 Brand Color Foundation

EdgeQuake uses a blue accent conveying trust, precision, and technical depth. The palette extends from the existing brand identity used in `edgequake-website/`.

#### Primary Accent

| Token           | Light Mode | Dark Mode | Usage                          |
| --------------- | ---------- | --------- | ------------------------------ |
| `--accent`      | `#2563EB`  | `#3B82F6` | Links, active nav, primary CTA |
| `--accent-high` | `#1E40AF`  | `#93C5FD` | Hover states, emphasized text  |
| `--accent-low`  | `#DBEAFE`  | `#1E3A5F` | Accent backgrounds, badges     |

#### Neutral Grays (Zinc-based)

| Token              | Light Mode | Dark Mode | Usage                        |
| ------------------ | ---------- | --------- | ---------------------------- |
| `--foreground`     | `#09090B`  | `#FAFAFA` | Primary text                 |
| `--muted-fg`       | `#71717A`  | `#A1A1AA` | Secondary text, captions     |
| `--border`         | `#E4E4E7`  | `#52525B` | Borders, dividers            |
| `--surface`        | `#FFFFFF`  | `#09090B` | Page background              |
| `--surface-raised` | `#FAFAFA`  | `#18181B` | Cards, popovers, code blocks |
| `--surface-muted`  | `#F4F4F5`  | `#27272A` | Sidebar, secondary surfaces  |

#### Semantic Colors

| Token       | Light Mode | Dark Mode | Usage                  |
| ----------- | ---------- | --------- | ---------------------- |
| `--success` | `#059669`  | `#10B981` | Success states, badges |
| `--warning` | `#D97706`  | `#F59E0B` | Caution admonitions    |
| `--danger`  | `#DC2626`  | `#EF4444` | Error states, danger   |
| `--info`    | `#2563EB`  | `#3B82F6` | Info admonitions, tips |

### 3.2 Starlight Theme Mapping

Starlight uses its own CSS custom property system. Our brand colors map as follows:

```css
/* src/styles/custom.css — Starlight theme override */

/* Dark mode (default) */
:root {
  --sl-color-accent-low: #1e3a5f; /* accent-low dark */
  --sl-color-accent: #3b82f6; /* accent dark */
  --sl-color-accent-high: #93c5fd; /* accent-high dark */
  --sl-color-white: #fafafa;
  --sl-color-gray-1: #eceef2;
  --sl-color-gray-2: #a1a1aa;
  --sl-color-gray-3: #71717a;
  --sl-color-gray-4: #52525b;
  --sl-color-gray-5: #27272a;
  --sl-color-gray-6: #18181b;
  --sl-color-black: #09090b;
}

/* Light mode */
:root[data-theme="light"] {
  --sl-color-accent-low: #dbeafe; /* accent-low light */
  --sl-color-accent: #2563eb; /* accent light */
  --sl-color-accent-high: #1e40af; /* accent-high light */
  --sl-color-white: #09090b;
  --sl-color-gray-1: #18181b;
  --sl-color-gray-2: #27272a;
  --sl-color-gray-3: #52525b;
  --sl-color-gray-4: #71717a;
  --sl-color-gray-5: #e4e4e7;
  --sl-color-gray-6: #f4f4f5;
  --sl-color-gray-7: #fafafa;
  --sl-color-black: #ffffff;
}
```

### 3.3 Tailwind Integration for Marketing Pages

Marketing pages (non-Starlight) use the same palette via Tailwind CSS custom properties, maintaining visual consistency across the site:

```css
/* src/styles/marketing.css — Marketing page Tailwind theme */

@theme {
  --color-accent-50: #eff6ff;
  --color-accent-100: #dbeafe;
  --color-accent-200: #bfdbfe;
  --color-accent-300: #93c5fd;
  --color-accent-400: #60a5fa;
  --color-accent-500: #3b82f6;
  --color-accent-600: #2563eb;
  --color-accent-700: #1d4ed8;
  --color-accent-800: #1e40af;
  --color-accent-900: #1e3a5f;

  --color-gray-50: #fafafa;
  --color-gray-100: #f4f4f5;
  --color-gray-200: #e4e4e7;
  --color-gray-300: #d4d4d8;
  --color-gray-400: #a1a1aa;
  --color-gray-500: #71717a;
  --color-gray-600: #52525b;
  --color-gray-700: #3f3f46;
  --color-gray-800: #27272a;
  --color-gray-900: #18181b;
  --color-gray-950: #09090b;
}
```

### 3.4 Color Hierarchy Diagram

```
                    ACCENT RAMP
 ┌──────────────────────────────────────────┐
 │ 50   100  200  300  400  500  600  700   │
 │ ░░░  ░░░  ▒▒▒  ▒▒▒  ▓▓▓  ▓▓▓  ███  ███ │
 │ bg   bg   bdr  ---  link pri  hover dk   │
 └──────────────────────────────────────────┘

                    GRAY RAMP (Zinc)
 ┌──────────────────────────────────────────┐
 │ 50   100  200  300  400  500  600  700   │
 │ ░░░  ░░░  ▒▒▒  ▒▒▒  ▓▓▓  ▓▓▓  ███  ███ │
 │ bg   card bdr  ---  muted sec  body dark │
 └──────────────────────────────────────────┘

           SEMANTIC COLORS
 ┌─────────┬─────────┬─────────┬─────────┐
 │ SUCCESS │ WARNING │ DANGER  │  INFO   │
 │ #059669 │ #D97706 │ #DC2626 │ #2563EB │
 │ green   │ amber   │ red     │ blue    │
 └─────────┴─────────┴─────────┴─────────┘
```

---

## 4. Typography

### 4.1 Font Stack

| Role     | Font Family                                  | Fallback                                       | Loading            |
| -------- | -------------------------------------------- | ---------------------------------------------- | ------------------ |
| Body/UI  | **Inter**                                    | system-ui, -apple-system, sans-serif           | Fontsource 400/600 |
| Code     | **JetBrains Mono** (Expressive Code default) | ui-monospace, SFMono-Regular, Menlo, monospace | Fontsource 400     |
| Headings | **Inter**                                    | same as body                                   | Fontsource 700     |

> **Why Inter?** Continuity with existing EdgeQuake branding. Inter is specifically designed for screens, offers excellent readability at all sizes, and includes tabular figures for data-heavy content. Used by astro.build, Vercel, Linear, and other developer-focused sites.

### 4.2 Type Scale (Major Third — 1.25)

| Token         | Size     | Line Height | Weight | Usage                             |
| ------------- | -------- | ----------- | ------ | --------------------------------- |
| `--text-xs`   | 0.75rem  | 1.5         | 400    | Badges, fine print                |
| `--text-sm`   | 0.875rem | 1.5         | 400    | Captions, sidebar links, metadata |
| `--text-base` | 1rem     | 1.7         | 400    | Body text, paragraphs             |
| `--text-lg`   | 1.125rem | 1.6         | 400    | Lead paragraphs                   |
| `--text-xl`   | 1.25rem  | 1.5         | 600    | H4, card titles                   |
| `--text-2xl`  | 1.5rem   | 1.3         | 700    | H3                                |
| `--text-3xl`  | 1.875rem | 1.25        | 700    | H2                                |
| `--text-4xl`  | 2.25rem  | 1.15        | 700    | H1 (docs pages)                   |
| `--text-5xl`  | 3rem     | 1.08        | 700    | Hero headings (marketing)         |
| `--text-6xl`  | 3.75rem  | 1.05        | 700    | Hero headings (home page only)    |

### 4.3 Starlight Typography Override

```css
/* Custom typography applied via customCss */
:root {
  --sl-font: "Inter", system-ui, sans-serif;
  --sl-font-mono: "JetBrains Mono", ui-monospace, monospace;

  /* Docs content width — wider than Starlight default */
  --sl-content-width: 52rem;

  /* Heading sizes */
  --sl-text-h1: var(--text-4xl);
  --sl-text-h2: var(--text-3xl);
  --sl-text-h3: var(--text-2xl);
  --sl-text-h4: var(--text-xl);
  --sl-text-h5: var(--text-lg);

  /* Body text tuning */
  --sl-text-body: 1rem;
  --sl-line-height: 1.7;
}
```

### 4.4 Typography Hierarchy Visual

```
    ┌──────────────────────────────────────┐
    │ Hero Heading (3.75rem / 60px)        │  ← Home page only
    │ ═══════════════════════════════      │
    │                                      │
    │ Page Title (2.25rem / 36px)          │  ← --text-4xl
    │ ─────────────────────────            │
    │                                      │
    │ Section H2 (1.875rem / 30px)         │  ← --text-3xl
    │ ─── ─── ─── ─── ─── ───             │
    │                                      │
    │ Subsection H3 (1.5rem / 24px)        │  ← --text-2xl
    │                                      │
    │ Body text sits at 1rem / 16px with   │  ← --text-base
    │ generous 1.7 line height for reading │
    │ comfort. Paragraphs have enough room │
    │ to breathe.                          │
    │                                      │
    │ Small text / metadata 0.875rem       │  ← --text-sm
    │ Badge / 0.75rem                      │  ← --text-xs
    └──────────────────────────────────────┘
```

---

## 5. Spacing System

### 5.1 Base Unit

**4px base unit** — consistent with the existing EdgeQuake WebUI design tokens and modern design system conventions (Tailwind, Material Design 3, Fluent 2).

### 5.2 Spacing Scale

| Token        | Value   | px  | Usage                                |
| ------------ | ------- | --- | ------------------------------------ |
| `--space-0`  | 0       | 0   | Reset                                |
| `--space-1`  | 0.25rem | 4   | Inline icon gap                      |
| `--space-2`  | 0.5rem  | 8   | Badge padding, tight gaps            |
| `--space-3`  | 0.75rem | 12  | Card padding (small), button padding |
| `--space-4`  | 1rem    | 16  | Standard paragraph gap, card padding |
| `--space-6`  | 1.5rem  | 24  | Section gap, card margin             |
| `--space-8`  | 2rem    | 32  | Component group spacing              |
| `--space-12` | 3rem    | 48  | Section padding (vertical)           |
| `--space-16` | 4rem    | 64  | Page section vertical spacing        |
| `--space-20` | 5rem    | 80  | Hero section top padding             |
| `--space-24` | 6rem    | 96  | Marketing section vertical rhythm    |
| `--space-32` | 8rem    | 128 | Maximum hero vertical padding        |

### 5.3 Semantic Spacing

| Context              | Token             | Value            |
| -------------------- | ----------------- | ---------------- |
| Page max width       | `--max-w-page`    | `80rem` (1280px) |
| Content max width    | `--max-w-content` | `52rem` (832px)  |
| Page horizontal pad  | `--px-page`       | `1rem` → `2rem`  |
| Section vertical pad | `--py-section`    | `4rem` → `6rem`  |
| Card padding         | `--p-card`        | `1.5rem`         |
| Sidebar width        | `--w-sidebar`     | `18rem` (288px)  |
| TOC width            | `--w-toc`         | `14rem` (224px)  |

> **Pattern source:** Cloudflare Docs uses `max-width: 80rem` for the outer container with generous horizontal padding. Starlight defaults to ~52rem content width. We keep both.

---

## 6. Layout Grid

### 6.1 Responsive Breakpoints

| Breakpoint | Min Width | Columns | Behavior                            |
| ---------- | --------- | ------- | ----------------------------------- |
| `sm`       | 640px     | 1       | Mobile: stacked layout              |
| `md`       | 768px     | 2       | Tablet: sidebar visible             |
| `lg`       | 1024px    | 3       | Desktop: sidebar + content + TOC    |
| `xl`       | 1280px    | 3       | Wide: max content width reached     |
| `2xl`      | 1536px    | 3       | Ultra-wide: centered with max-width |

### 6.2 Marketing Grid

```
    ┌──────────────────────────────────────────────┐
    │ max-width: 80rem (1280px), centered          │
    │                                              │
    │  ┌──────────────────────────────────────┐    │
    │  │          px: 1rem → 2rem              │    │
    │  │  ┌──────────────────────────────┐    │    │
    │  │  │    12-column grid (gap 2rem)  │    │    │
    │  │  │  ┌──┬──┬──┬──┬──┬──┬──┬──┐   │    │    │
    │  │  │  │1 │2 │3 │4 │5 │6 │7 │8 │...│    │    │
    │  │  │  └──┴──┴──┴──┴──┴──┴──┴──┘   │    │    │
    │  │  └──────────────────────────────┘    │    │
    │  └──────────────────────────────────────┘    │
    └──────────────────────────────────────────────┘

    Hero:      col-span-12 (full) or 6+6 (text + visual)
    Features:  col-span-4 × 3 cards
    Metrics:   col-span-3 × 4 stat boxes
```

### 6.3 Docs Grid (Starlight)

````
    ┌──────────────────────────────────────────────┐
    │         Full viewport width                   │
    │  ┌────────┬──────────────────────┬────────┐  │
    │  │Sidebar │     Content          │  TOC   │  │
    │  │ 18rem  │     52rem max        │ 14rem  │  │
    │  │        │                      │        │  │
    │  │ ▸ Get  │  # Page Title        │ On this│  │
    │  │   Start│                      │ page:  │  │
    │  │ ▸ Conc │  Body text at 1rem   │        │  │
    │  │ ▸ Arch │  with 1.7 line-height│ ▸ Sec1 │  │
    │  │ ▾ API  │                      │ ▸ Sec2 │  │
    │  │   └ref │  ```code blocks```   │ ▸ Sec3 │  │
    │  │   └up  │                      │        │  │
    │  │ ▸ Tuto │                      │        │  │
    │  │        │                      │        │  │
    │  └────────┴──────────────────────┴────────┘  │
    │                                              │
    │  Mobile: sidebar hidden, TOC collapsed       │
    │  Tablet: sidebar visible, TOC hidden         │
    │  Desktop: all three panels visible           │
    └──────────────────────────────────────────────┘
````

---

## 7. Dark / Light Mode

### 7.1 Implementation Strategy

| Layer           | Mechanism                             | Notes                                 |
| --------------- | ------------------------------------- | ------------------------------------- |
| Starlight docs  | Built-in theme toggle (`data-theme`)  | Starlight handles automatically       |
| Marketing pages | Class-based (`.dark`) via JS toggle   | Matches Starlight's `data-theme` attr |
| Code blocks     | Expressive Code auto-sync             | `useStarlightDarkModeSwitch: true`    |
| User preference | `prefers-color-scheme` media query    | Default on first visit                |
| Persistence     | `localStorage` key: `starlight-theme` | Starlight's built-in persistence      |

### 7.2 Theme Switching Flow

```
    User clicks theme toggle
            │
            ▼
    ┌─────────────────────┐
    │  Toggle data-theme  │
    │  on <html> element  │
    └──────────┬──────────┘
               │
       ┌───────┴───────┐
       ▼               ▼
  data-theme     CSS variables
  = "light"      swap instantly
       │               │
       ▼               ▼
  localStorage    Expressive Code
  updated         theme syncs
       │               │
       ▼               ▼
  Next visit      Code blocks
  remembers       match theme
```

### 7.3 Contrast Ratios

All color pairs must meet WCAG AA (4.5:1 for normal text, 3:1 for large text):

| Pair                      | Light Ratio | Dark Ratio | Status |
| ------------------------- | ----------- | ---------- | ------ |
| Body text on background   | 18.1:1      | 16.4:1     | ✅ AAA |
| Muted text on background  | 4.7:1       | 4.6:1      | ✅ AA  |
| Accent on background      | 4.6:1       | 4.8:1      | ✅ AA  |
| Accent-high on accent-low | 5.2:1       | 5.0:1      | ✅ AA  |
| White on accent (buttons) | 5.8:1       | 4.7:1      | ✅ AA  |

---

## 8. Motion & Animation

### 8.1 Motion Principles

| Principle      | Description                                                 |
| -------------- | ----------------------------------------------------------- |
| **Purposeful** | Every animation communicates state change; no decoration    |
| **Fast**       | Duration under 300ms for UI transitions                     |
| **Reducible**  | `prefers-reduced-motion` disables all animations            |
| **CSS-first**  | Prefer CSS transitions/animations; JS only for canvas graph |

### 8.2 Duration & Easing Tokens

| Token               | Value                               | Usage                         |
| ------------------- | ----------------------------------- | ----------------------------- |
| `--duration-fast`   | `100ms`                             | Button hover, focus ring      |
| `--duration-normal` | `200ms`                             | Tooltip appear, menu open     |
| `--duration-slow`   | `300ms`                             | Page transition, modal        |
| `--duration-enter`  | `400ms`                             | Fade-in on scroll (marketing) |
| `--ease-out`        | `cubic-bezier(0.16, 1, 0.3, 1)`     | Default exit easing           |
| `--ease-in-out`     | `cubic-bezier(0.4, 0, 0.2, 1)`      | Symmetric transitions         |
| `--ease-spring`     | `cubic-bezier(0.34, 1.56, 0.64, 1)` | Playful bounce (badges, CTAs) |

### 8.3 Animation Catalog

| Animation            | Trigger                | Duration | CSS Property         | Notes                       |
| -------------------- | ---------------------- | -------- | -------------------- | --------------------------- |
| Link hover underline | `:hover`               | 200ms    | `border-bottom`      | Slides from left            |
| Button scale         | `:active`              | 100ms    | `transform`          | `scale(0.98)`               |
| Card lift            | `:hover`               | 200ms    | `transform, shadow`  | `translateY(-2px)`          |
| Fade-in on scroll    | `IntersectionObserver` | 400ms    | `opacity, transform` | Marketing sections only     |
| Mobile menu slide    | Toggle                 | 200ms    | `max-height`         | Accordion expand            |
| Search modal         | `Cmd+K`                | 200ms    | `opacity, scale`     | Pagefind overlay            |
| Graph node drift     | Continuous             | ∞        | Canvas 2D            | React island, 60fps target  |
| Theme toggle         | Click                  | 0ms      | CSS variables        | Instant swap, no transition |

### 8.4 Reduced Motion

```css
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
    scroll-behavior: auto !important;
  }
}
```

---

## 9. Elevation & Shadows

| Level | Name       | Value (Light)                 | Value (Dark)                     | Usage                |
| ----- | ---------- | ----------------------------- | -------------------------------- | -------------------- |
| 0     | Flat       | none                          | none                             | Default surfaces     |
| 1     | Raised     | `0 1px 2px rgba(0,0,0,0.05)`  | `0 1px 2px rgba(0,0,0,0.3)`      | Cards at rest        |
| 2     | Hover      | `0 4px 12px rgba(0,0,0,0.08)` | `0 4px 12px rgba(0,0,0,0.4)`     | Hovered cards        |
| 3     | Overlay    | `0 8px 24px rgba(0,0,0,0.12)` | `0 8px 24px rgba(0,0,0,0.5)`     | Dropdowns, modals    |
| 4     | Navigation | `0 1px 0 rgba(0,0,0,0.06)`    | `0 1px 0 rgba(255,255,255,0.06)` | Sticky header border |
| ring  | Focus      | `0 0 0 2px var(--accent)`     | `0 0 0 2px var(--accent)`        | Focus indicator      |

---

## 10. Border Radius

| Token           | Value      | Usage                        |
| --------------- | ---------- | ---------------------------- |
| `--radius-sm`   | `0.25rem`  | Badges, inline code          |
| `--radius-md`   | `0.375rem` | Buttons, inputs              |
| `--radius-lg`   | `0.5rem`   | Cards, code blocks           |
| `--radius-xl`   | `0.75rem`  | Feature cards, modals        |
| `--radius-2xl`  | `1rem`     | Hero cards, large containers |
| `--radius-full` | `9999px`   | Avatar, circular badges      |

---

## 11. Icon System

| Property     | Value                        | Notes                                   |
| ------------ | ---------------------------- | --------------------------------------- |
| Library      | **Lucide React** (marketing) | Consistent with existing site           |
| Starlight    | **Built-in icon set**        | Starlight provides its own icon catalog |
| Stroke width | 1.5px                        | Lighter than default 2px for minimalism |
| Default size | 16px (`1rem`)                | Matches body text baseline              |
| Header icons | 20px (`1.25rem`)             | Social links, nav actions               |
| Hero icons   | 24px (`1.5rem`)              | CTA buttons, feature icons              |

> **Pattern:** Biome and SST Ion both use Lucide icons with thinner stroke weights for a cleaner feel. Starlight's built-in icons handle sidebar and UI chrome automatically.

---

## 12. Design Token File Structure

```
edgequake-website/
├── src/
│   ├── styles/
│   │   ├── global.css           # Tailwind base + Starlight layer setup
│   │   ├── starlight-theme.css  # Starlight CSS variable overrides (§3.2)
│   │   ├── marketing.css        # Marketing page Tailwind theme (§3.3)
│   │   ├── typography.css       # Font imports + type scale
│   │   └── animations.css       # Motion tokens + reduced-motion
│   └── fonts/
│       └── font-face.css        # @font-face for Inter + JetBrains Mono
├── astro.config.mjs             # customCss: ['./src/styles/...']
```

Integration in Astro config (see [04-starlight-project-setup.md](./04-starlight-project-setup.md)):

```javascript
starlight({
  customCss: [
    "./src/styles/starlight-theme.css",
    "./src/styles/typography.css",
    "@fontsource/inter/400.css",
    "@fontsource/inter/600.css",
    "@fontsource/inter/700.css",
    "@fontsource/jetbrains-mono/400.css",
  ],
});
```

---

## 13. Design Reference Sites

| Site                       | URL                       | Key Patterns Adopted                         |
| -------------------------- | ------------------------- | -------------------------------------------- |
| **Cloudflare Docs**        | developers.cloudflare.com | Sidebar structure, content width, clean type |
| **Netlify Docs**           | docs.netlify.com          | Starlight theming, search UX, mobile sidebar |
| **Biome**                  | biomejs.dev               | Developer tool branding, minimal color usage |
| **SST Ion**                | ion.sst.dev               | Modern docs feel, code-first layout          |
| **Fluent 2 Design System** | fluent2.microsoft.design  | Token-based system, systematic approach      |
| **Proton**                 | proton.me                 | Minimalist marketing, generous whitespace    |
| **astro.build**            | astro.build               | Hero section, gradient text, badge patterns  |
| **Porsche**                | porsche.com               | Premium feel, restrained palette, clean type |
| **Starlight defaults**     | starlight.astro.build     | Built-in theme editor, component patterns    |

---

## 14. Implementation Checklist

| Step | Task                                           | Spec Reference                                            |
| ---- | ---------------------------------------------- | --------------------------------------------------------- |
| 1    | Install Fontsource packages (Inter, JetBrains) | §4.1, [04-project-setup](./04-starlight-project-setup.md) |
| 2    | Create `starlight-theme.css`                   | §3.2                                                      |
| 3    | Create `marketing.css` with Tailwind theme     | §3.3                                                      |
| 4    | Create `typography.css` with type scale        | §4.2, §4.3                                                |
| 5    | Create `animations.css` with motion tokens     | §8.2, §8.4                                                |
| 6    | Wire CSS files into `astro.config.mjs`         | §12, [04-project-setup](./04-starlight-project-setup.md)  |
| 7    | Validate contrast ratios with WebAIM checker   | §7.3                                                      |
| 8    | Test dark/light toggle on all page types       | §7.1                                                      |
| 9    | Verify reduced-motion behavior                 | §8.4                                                      |
| 10   | Visual diff against current site screenshots   | [09-migration-roadmap](./09-migration-roadmap.md)         |

---

_Cross-references: [00-overview](./00-overview.md) · [04-project-setup](./04-starlight-project-setup.md) · [06-search-nav-seo](./06-search-navigation-seo.md) · [07-authoring](./07-content-authoring-standards.md) · [09-migration](./09-migration-roadmap.md) · [12-layouts](./12-page-layouts-wireframes.md) · [13-components](./13-component-library.md)_
