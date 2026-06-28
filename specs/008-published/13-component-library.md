# SPEC-008-13: Component Library

| Field       | Value                                 |
| ----------- | ------------------------------------- |
| **Spec ID** | SPEC-008-13                           |
| **Parent**  | [SPEC-008 Overview](./00-overview.md) |
| **Title**   | Component Library                     |
| **Status**  | Draft                                 |
| **Created** | 2026-03-21                            |
| **Updated** | 2026-03-21                            |

---

## 1. Purpose

Catalog every reusable component for the unified EdgeQuake Astro + Starlight site. Each entry defines purpose, props, hydration strategy, design tokens used, and the migration path from the current Next.js implementation.

**Cross-references:**

- [00-overview.md](./00-overview.md) — Scope and goals
- [04-starlight-project-setup.md](./04-starlight-project-setup.md) — Directory structure, file locations
- [09-migration-roadmap.md](./09-migration-roadmap.md) — Phase-by-phase migration plan
- [11-design-system.md](./11-design-system.md) — Design tokens referenced by components
- [12-page-layouts-wireframes.md](./12-page-layouts-wireframes.md) — Where components are placed

---

## 2. Component Architecture

```
    ┌────────────────────────────────────────────────────────────┐
    │                    Component Layers                         │
    │                                                            │
    │  ┌──────────────────────────────────────────────────────┐ │
    │  │  LAYER 3: PAGE COMPOSITIONS                           │ │
    │  │  Marketing pages, Docs overrides                      │ │
    │  │  (Assemble Layer 2 components into full pages)        │ │
    │  └──────────────────────────────────────────────────────┘ │
    │                          │ uses                            │
    │  ┌──────────────────────────────────────────────────────┐ │
    │  │  LAYER 2: DOMAIN COMPONENTS                           │ │
    │  │  Hero, ProblemSection, BenchmarksSection, ...          │ │
    │  │  (Business-specific, marketing content)               │ │
    │  └──────────────────────────────────────────────────────┘ │
    │                          │ uses                            │
    │  ┌──────────────────────────────────────────────────────┐ │
    │  │  LAYER 1: PRIMITIVES                                  │ │
    │  │  Button, Badge, Card, CodeBlock, Container, ...       │ │
    │  │  (Styled atoms, no business logic)                    │ │
    │  └──────────────────────────────────────────────────────┘ │
    │                          │ uses                            │
    │  ┌──────────────────────────────────────────────────────┐ │
    │  │  LAYER 0: STARLIGHT BUILT-INS                         │ │
    │  │  Sidebar, TOC, Pagefind, Aside, Tabs, Steps, ...     │ │
    │  │  (Zero-config, used in docs content)                  │ │
    │  └──────────────────────────────────────────────────────┘ │
    └────────────────────────────────────────────────────────────┘
```

---

## 3. Layer 0: Starlight Built-in Components

These require zero custom code. They are used in MDX documentation content via Starlight's component auto-imports.

| Component        | Usage Context             | Import Required | Notes                        |
| ---------------- | ------------------------- | --------------- | ---------------------------- | ---------- | ------ | ---- |
| `<Aside>`        | Tips, notes, warnings     | Auto-imported   | Or use `:::tip` / `:::note`  |
| `<Badge>`        | Status labels             | Auto-imported   | `variant: note               | tip        | danger | ...` |
| `<Card>`         | Feature cards in docs     | Auto-imported   | Title + description          |
| `<CardGrid>`     | Grid of cards             | Auto-imported   | `stagger` prop for animation |
| `<Code>`         | Syntax-highlighted blocks | Auto-imported   | Via Expressive Code          |
| `<FileTree>`     | Directory structures      | Auto-imported   | YAML-like syntax             |
| `<Icon>`         | SVG icons                 | Auto-imported   | Starlight icon set           |
| `<LinkButton>`   | CTA buttons in docs       | Auto-imported   | `variant: primary            | secondary` |
| `<LinkCard>`     | Clickable card with link  | Auto-imported   | Title + description + href   |
| `<Steps>`        | Numbered procedure steps  | Auto-imported   | Ordered `<ol>` styling       |
| `<Tabs>`         | Tab panels                | Auto-imported   | `<TabItem label="...">`      |
| `<ContentPanel>` | Main doc content wrapper  | Internal        | Automatically wraps MDX      |
| Pagefind Search  | Full-text search          | Built-in        | Cmd+K or click 🔍            |
| Sidebar          | Docs navigation           | Built-in        | Configured in `astro.config` |
| TOC              | On-this-page links        | Built-in        | Right-column, sticky         |
| Prev/Next links  | Bottom navigation         | Built-in        | Auto-generated from sidebar  |

> **Pattern source:** Biome and Cloudflare Docs use Starlight built-ins extensively, keeping custom component count low for maintainability.

---

## 4. Layer 1: Primitive Components

Reusable atoms shared between marketing and documentation pages.

### 4.1 Button

```
    ┌─────────────────────────────────────┐
    │  [  Label  ▸ ]     Primary          │
    │  [  Label    ]     Secondary        │
    │  [  Label    ]     Ghost            │
    └─────────────────────────────────────┘
```

| Prop        | Type                                  | Default     |
| ----------- | ------------------------------------- | ----------- |
| `variant`   | `"primary" \| "secondary" \| "ghost"` | `"primary"` |
| `size`      | `"sm" \| "md" \| "lg"`                | `"md"`      |
| `href`      | `string?`                             | —           |
| `icon`      | `LucideIcon?`                         | —           |
| `iconRight` | `boolean`                             | `false`     |

**Styles:**

| Variant   | Background      | Text                | Border          |
| --------- | --------------- | ------------------- | --------------- |
| primary   | `var(--accent)` | `white`             | none            |
| secondary | `transparent`   | `var(--foreground)` | `var(--border)` |
| ghost     | `transparent`   | `var(--muted-fg)`   | none            |

**Hydration:** None (`<a>` tag renders statically).

**File:** `src/components/primitives/Button.astro`

### 4.2 Badge

```
    ┌─────────┐  ┌────────────┐  ┌───────────┐
    │ Apache  │  │ Rust 🦀    │  │ v0.4.0    │
    └─────────┘  └────────────┘  └───────────┘
```

| Prop      | Type                               | Default     |
| --------- | ---------------------------------- | ----------- |
| `variant` | `"default" \| "accent" \| "muted"` | `"default"` |
| `icon`    | `string?`                          | —           |

**Styles:** `text-xs`, `rounded-full`, `px-3 py-1`, border.

**Hydration:** None (static HTML).

**File:** `src/components/primitives/Badge.astro`

### 4.3 Card

Marketing card with icon, title, and description.

```
    ┌──────────────────────┐
    │  🔍                   │
    │                       │
    │  Title here           │
    │                       │
    │  Description text     │
    │  spans multiple       │
    │  lines if needed.     │
    └──────────────────────┘
```

| Prop          | Type      | Default |
| ------------- | --------- | ------- |
| `icon`        | `string`  | —       |
| `title`       | `string`  | —       |
| `description` | `string`  | —       |
| `href`        | `string?` | —       |

**Styles:** `border rounded-xl p-6`, `hover:border-accent/50` transition 200ms. Dark: `bg-card`.

**Hydration:** None (static HTML, optional `<a>` wrapper if `href`).

**File:** `src/components/primitives/Card.astro`

### 4.4 Container

Max-width wrapper for consistent horizontal alignment.

| Prop   | Type                   | Default |
| ------ | ---------------------- | ------- |
| `size` | `"sm" \| "md" \| "lg"` | `"lg"`  |
| `as`   | `string`               | `"div"` |

**Sizes:** `sm` = `max-w-3xl`, `md` = `max-w-5xl`, `lg` = `max-w-7xl` (80rem).

**File:** `src/components/primitives/Container.astro`

### 4.5 SectionWrapper

Wraps each landing-page section with consistent padding and optional background.

| Prop         | Type                       | Default         |
| ------------ | -------------------------- | --------------- |
| `background` | `"transparent" \| "muted"` | `"transparent"` |
| `id`         | `string?`                  | —               |
| `class`      | `string?`                  | —               |

**Styles:** `py-16 md:py-24` on transparent, `bg-muted/50 py-16 md:py-24` on muted.

**File:** `src/components/primitives/SectionWrapper.astro`

---

## 5. Layer 2: Domain Components (Marketing)

### 5.1 Header

Shared across marketing and docs pages, providing unified brand navigation.

| Prop     | Type      | Default |
| -------- | --------- | ------- |
| `active` | `string?` | —       |

**Structure:**

```
    Header.astro
    ├── Logo (⚡ EdgeQuake)
    ├── NavLinks  → [{href, label}]
    ├── Search trigger (opens Pagefind)
    ├── ThemeToggle (React island)
    └── CTA Button ("Get Started")
```

**Nav Links Config:**

```typescript
const navLinks = [
  { href: "/docs/", label: "Docs" },
  { href: "/demo/", label: "Demo" },
  { href: "/ecosystem/", label: "Ecosystem" },
  { href: "/enterprise/", label: "Enterprise" },
];
```

**Starlight Override:** This component overrides Starlight's default `<Header>` for marketing pages. Docs pages use a modified Starlight header that includes the same nav links. Configuration:

```js
// astro.config.mjs
starlight({
  components: {
    Header: "./src/components/overrides/StarlightHeader.astro",
  },
});
```

**Hydration:** ThemeToggle is `client:load` (< 2KB). Rest is static.

**Migration from Next.js:** Direct port from `header.tsx`. Replace `next/link` with `<a>`. Replace `next-themes` with Starlight's `data-theme`.

**File:** `src/components/marketing/Header.astro`

### 5.2 Footer

Full-width footer with brand column + 4 link groups.

**Structure:**

```
    Footer.astro
    ├── BrandColumn (logo + tagline)
    ├── LinkGroups × 4
    │   ├── Product (Get Started, Demo, Ecosystem, Enterprise)
    │   ├── Developers (Docs, Core Concepts, API Reference, crates.io)
    │   ├── Community (GitHub, Issues, Discussions, Changelog)
    │   └── Company (Contact, Elitizon, License)
    └── BottomBar (copyright + social links)
```

**Hydration:** None (static HTML).

**Migration from Next.js:** Direct port from `footer.tsx`.

**File:** `src/components/marketing/Footer.astro`

### 5.3 HeroSection

Two-column hero with headline, description, CTAs, and animated graph.

| Prop           | Type                  | Default |
| -------------- | --------------------- | ------- |
| `title`        | `string`              | —       |
| `highlight`    | `string`              | —       |
| `description`  | `string`              | —       |
| `primaryCTA`   | `{href, label}`       | —       |
| `secondaryCTA` | `{href, label}`       | —       |
| `badges`       | `Array<{icon, text}>` | —       |

**Layout:** See [12-page-layouts-wireframes.md §4](./12-page-layouts-wireframes.md).

**Hydration:** Contains `<GraphAnimation>` React island (`client:visible`).

**Migration from Next.js:** Port `hero.tsx`. Replace framer-motion `FadeIn` with CSS `@keyframes` (see §5.12). GraphAnimation component stays React but hydrated via Astro island.

**File:** `src/components/marketing/HeroSection.astro`

### 5.4 ProblemSection

Three-column card grid explaining why classic RAG fails.

**Structure:** SectionWrapper (muted bg) → Container → H2 + CardGrid (3 cards).

**Hydration:** None.

**File:** `src/components/marketing/ProblemSection.astro`

### 5.5 SolutionSection

Three-column card grid presenting EdgeQuake's approach.

**Structure:** SectionWrapper → Container → H2 + CardGrid (3 cards).

**Hydration:** None.

**File:** `src/components/marketing/SolutionSection.astro`

### 5.6 ArchitectureSection

Centered architecture diagram (ASCII or SVG).

**Structure:** SectionWrapper (muted bg) → Container (size=md) → H2 + Diagram.

**Hydration:** None (static diagram, either SVG `<img>` or `<pre>` ASCII).

**File:** `src/components/marketing/ArchitectureSection.astro`

### 5.7 BenchmarksSection

Four-column stat cards with large numbers.

```
    ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐
    │ 1000+  │ │   6    │ │  10×   │ │  52    │
    │docs/min│ │ query  │ │ faster │ │ pages  │
    │        │ │ modes  │ │        │ │  docs  │
    └────────┘ └────────┘ └────────┘ └────────┘
```

| Prop    | Type                                    | Default |
| ------- | --------------------------------------- | ------- |
| `stats` | `Array<{value: string, label: string}>` | —       |

**Structure:** SectionWrapper → Container → H2 + 4-col grid.

**Hydration:** Optional `client:visible` for animated number count-up.

**File:** `src/components/marketing/BenchmarksSection.astro`

### 5.8 QuickStartSection

Code block with copy-paste install commands.

**Structure:** SectionWrapper (muted bg) → Container (size=md) → H2 + Expressive Code block.

**Hydration:** None (Expressive Code handles copy button server-side).

**File:** `src/components/marketing/QuickStartSection.astro`

### 5.9 EcosystemSection

Grid of integration/SDK cards with icons and badges.

**Structure:** SectionWrapper → Container → H2 + CardGrid.

**Hydration:** None.

**File:** `src/components/marketing/EcosystemSection.astro`

### 5.10 EnterpriseCTASection

Full-width band with accent gradient background, headline, and CTA.

**Structure:** SectionWrapper → `bg-gradient-to-r from-accent/10 to-accent/5` → centered H2 + Button.

**Hydration:** None.

**File:** `src/components/marketing/EnterpriseCTASection.astro`

### 5.11 LogoCloud

Grayscale company logos in a flex row. Horizontal scroll on mobile.

| Prop    | Type                                               | Default |
| ------- | -------------------------------------------------- | ------- |
| `logos` | `Array<{src: string, alt: string, href?: string}>` | —       |

**Styles:** `filter: grayscale(1) opacity(0.6)`, `hover: grayscale(0) opacity(1)`.

**File:** `src/components/marketing/LogoCloud.astro`

### 5.12 FadeIn (CSS-only)

Replaces framer-motion `FadeIn` animation. Uses `IntersectionObserver` in a minimal inline script.

**Implementation approach:**

```astro
<!-- FadeIn.astro -->
<div class="fade-in-section" data-fade>
  <slot />
</div>

<style>
  .fade-in-section {
    opacity: 0;
    transform: translateY(1rem);
    transition: opacity 400ms var(--ease-out),
                transform 400ms var(--ease-out);
  }
  .fade-in-section.is-visible {
    opacity: 1;
    transform: translateY(0);
  }
  @media (prefers-reduced-motion: reduce) {
    .fade-in-section {
      opacity: 1;
      transform: none;
      transition: none;
    }
  }
</style>

<script>
  const observer = new IntersectionObserver(
    (entries) => entries.forEach(e => {
      if (e.isIntersecting) {
        e.target.classList.add('is-visible');
        observer.unobserve(e.target);
      }
    }),
    { threshold: 0.1 }
  );
  document.querySelectorAll('[data-fade]').forEach(el => observer.observe(el));
</script>
```

**JS bundle size:** ~200 bytes (inline, no framework).

**Migration from Next.js:** Replaces `framer-motion` FadeIn/StaggerContainer/StaggerItem entirely. Zero-JS on reduced motion.

---

## 6. React Island Components

These components require client-side interactivity and are hydrated via Astro's island architecture.

### 6.1 GraphAnimation

Canvas-based animated graph visualization shown in the hero section.

| Prop        | Type     | Default |
| ----------- | -------- | ------- |
| `width`     | `number` | `480`   |
| `height`    | `number` | `480`   |
| `nodeCount` | `number` | `14`    |

**Hydration:** `client:visible` — only loads when hero scrolls into view (which is immediate on home, but deferred for other contexts).

**Bundle size:** ~4KB (canvas API, no external deps).

**Migration from Next.js:** Direct copy of `graph-animation.tsx`. Remove Next.js-specific imports. The component is already self-contained.

**File:** `src/components/islands/GraphAnimation.tsx`

### 6.2 ThemeToggle

Light/dark mode toggle using Starlight's `data-theme` attribute.

| Prop | Type | Default |
| ---- | ---- | ------- |
| —    | —    | —       |

**Implementation:**

```tsx
function ThemeToggle() {
  const [theme, setTheme] = useState<"light" | "dark">("light");

  useEffect(() => {
    const current = document.documentElement.dataset.theme;
    setTheme(current === "dark" ? "dark" : "light");
  }, []);

  const toggle = () => {
    const next = theme === "light" ? "dark" : "light";
    document.documentElement.dataset.theme = next;
    localStorage.setItem("starlight-theme", next);
    setTheme(next);
  };

  return (
    <button
      onClick={toggle}
      aria-label={`Switch to ${theme === "light" ? "dark" : "light"} mode`}
    >
      {theme === "light" ? <Moon size={18} /> : <Sun size={18} />}
    </button>
  );
}
```

**Hydration:** `client:load` (needed for immediate interaction).

**Bundle size:** ~2KB.

**Migration from Next.js:** Replace `next-themes` with direct `data-theme` manipulation. Starlight uses `data-theme` on `<html>`, so the toggle must target that.

**File:** `src/components/islands/ThemeToggle.tsx`

### 6.3 DemoModeSelector

Interactive tab selector for the demo page query modes.

| Prop       | Type                              | Default |
| ---------- | --------------------------------- | ------- |
| `modes`    | `Array<{id, label, description}>` | —       |
| `onSelect` | `(modeId: string) => void`        | —       |

**Hydration:** `client:load`.

**Bundle size:** ~5KB.

**File:** `src/components/islands/DemoModeSelector.tsx`

### 6.4 ContactForm

Form with validation and submission handling.

| Prop       | Type     | Default |
| ---------- | -------- | ------- |
| `endpoint` | `string` | —       |

**Fields:** Name, Email, Subject (dropdown), Message (textarea).

**Validation:** Client-side with native HTML5 validation + minimal JS for UX (disabling submit on send, showing success toast).

**Hydration:** `client:load`.

**Bundle size:** ~3KB.

**File:** `src/components/islands/ContactForm.tsx`

---

## 7. Starlight Component Overrides

Starlight allows overriding specific built-in components via the `components` config key.

### 7.1 Override Strategy

```
    ┌──────────────────────────────────────────────────┐
    │  Starlight Component Override Map                 │
    │                                                   │
    │  Override        │ Purpose                        │
    │  ────────────────┼──────────────────────────────  │
    │  Header          │ Add marketing nav links,       │
    │                  │ GitHub link, CTA button         │
    │  ────────────────┼──────────────────────────────  │
    │  SocialIcons     │ Not overridden (use config)    │
    │  ────────────────┼──────────────────────────────  │
    │  Footer          │ Not overridden (Starlight      │
    │                  │ footer is fine for docs; the    │
    │                  │ marketing Footer is separate)   │
    └──────────────────────────────────────────────────┘
```

### 7.2 StarlightHeader Override

The overridden header wraps Starlight's default header content while adding our marketing nav links for cross-navigation.

```astro
---
// src/components/overrides/StarlightHeader.astro
import type { Props } from '@astrojs/starlight/props';
import Default from '@astrojs/starlight/components/Header.astro';
---

<div class="eq-header-wrapper">
  <!-- Inject marketing nav above Starlight's header -->
  <nav class="eq-top-nav" aria-label="Site navigation">
    <a href="/" class="eq-logo">⚡ EdgeQuake</a>
    <div class="eq-nav-links">
      <a href="/docs/">Docs</a>
      <a href="/demo/">Demo</a>
      <a href="/ecosystem/">Ecosystem</a>
      <a href="/enterprise/">Enterprise</a>
    </div>
  </nav>
  <!-- Starlight's built-in header (search, theme, sidebar toggle) -->
  <Default {...Astro.props}><slot /></Default>
</div>

<style>
  .eq-top-nav {
    display: flex;
    align-items: center;
    gap: 1.5rem;
    padding: 0.5rem 1rem;
    border-bottom: 1px solid var(--sl-color-gray-5);
    font-size: var(--sl-text-xs);
  }
  .eq-nav-links {
    display: flex;
    gap: 1rem;
  }
  .eq-nav-links a {
    color: var(--sl-color-gray-2);
    text-decoration: none;
  }
  .eq-nav-links a:hover {
    color: var(--sl-color-white);
  }
</style>
```

**Astro config:**

```js
starlight({
  components: {
    Header: "./src/components/overrides/StarlightHeader.astro",
  },
});
```

---

## 8. Component Migration Map

Mapping from the current Next.js marketing site to new Astro components.

| Next.js Component  | File                  | Astro Equivalent             | Change Type     |
| ------------------ | --------------------- | ---------------------------- | --------------- |
| `Header`           | `header.tsx`          | `Header.astro`               | Rewrite (Astro) |
| `Footer`           | `footer.tsx`          | `Footer.astro`               | Rewrite (Astro) |
| `Hero`             | `hero.tsx`            | `HeroSection.astro`          | Rewrite (Astro) |
| `Problem`          | `problem.tsx`         | `ProblemSection.astro`       | Rewrite (Astro) |
| `Solution`         | `solution.tsx`        | `SolutionSection.astro`      | Rewrite (Astro) |
| `Architecture`     | `architecture.tsx`    | `ArchitectureSection.astro`  | Rewrite (Astro) |
| `Benchmarks`       | `benchmarks.tsx`      | `BenchmarksSection.astro`    | Rewrite (Astro) |
| `QuickStart`       | `quickstart.tsx`      | `QuickStartSection.astro`    | Rewrite (Astro) |
| `Ecosystem`        | `ecosystem.tsx`       | `EcosystemSection.astro`     | Rewrite (Astro) |
| `EnterpriseCTA`    | `enterprise-cta.tsx`  | `EnterpriseCTASection.astro` | Rewrite (Astro) |
| `GraphAnimation`   | `graph-animation.tsx` | `islands/GraphAnimation.tsx` | Copy (React)    |
| `ThemeToggle`      | `theme-toggle.tsx`    | `islands/ThemeToggle.tsx`    | Adapt           |
| `FadeIn`           | `animations.tsx`      | `FadeIn.astro` (CSS)         | Replace (CSS)   |
| `StaggerContainer` | `animations.tsx`      | Removed (use FadeIn cascade) | Remove          |
| `StaggerItem`      | `animations.tsx`      | Removed (use FadeIn cascade) | Remove          |
| shadcn Button      | shadcn/ui             | `Button.astro`               | Simplify        |
| shadcn Badge       | shadcn/ui             | `Badge.astro`                | Simplify        |

**Migration notes:**

1. **framer-motion → CSS**: All framer-motion animations (FadeIn, StaggerContainer, StaggerItem) are replaced by the CSS-only FadeIn component (§5.12). This eliminates ~40KB of JS from the bundle.
2. **shadcn/ui → Astro primitives**: shadcn components are replaced by our simpler Astro primitives. Marketing pages don't need the full shadcn runtime.
3. **next-themes → Starlight theme**: Theme toggle integrates with Starlight's built-in `data-theme` approach.
4. **next/link → `<a>`**: Astro static pages use standard `<a>` tags; no client-side router.
5. **next/font → Fontsource**: Inter and JetBrains Mono loaded via Fontsource packages. See [11-design-system.md §4](./11-design-system.md).

---

## 9. File Organization

```
    src/
    ├── components/
    │   ├── primitives/          # Layer 1: Reusable atoms
    │   │   ├── Button.astro
    │   │   ├── Badge.astro
    │   │   ├── Card.astro
    │   │   ├── Container.astro
    │   │   └── SectionWrapper.astro
    │   │
    │   ├── marketing/           # Layer 2: Domain components
    │   │   ├── Header.astro
    │   │   ├── Footer.astro
    │   │   ├── HeroSection.astro
    │   │   ├── ProblemSection.astro
    │   │   ├── SolutionSection.astro
    │   │   ├── ArchitectureSection.astro
    │   │   ├── BenchmarksSection.astro
    │   │   ├── QuickStartSection.astro
    │   │   ├── EcosystemSection.astro
    │   │   ├── EnterpriseCTASection.astro
    │   │   ├── LogoCloud.astro
    │   │   └── FadeIn.astro
    │   │
    │   ├── islands/             # React islands (client hydrated)
    │   │   ├── GraphAnimation.tsx
    │   │   ├── ThemeToggle.tsx
    │   │   ├── DemoModeSelector.tsx
    │   │   └── ContactForm.tsx
    │   │
    │   └── overrides/           # Starlight component overrides
    │       └── StarlightHeader.astro
    │
    ├── layouts/
    │   └── MarketingLayout.astro    # Wraps marketing pages
    │
    └── pages/
        ├── index.astro              # Landing page (Layer 3)
        ├── demo.astro               # Demo page
        ├── ecosystem.astro          # Ecosystem page
        ├── enterprise.astro         # Enterprise page
        ├── contact.astro            # Contact page
        └── 404.astro                # Error page
```

---

## 10. Bundle Size Budget

| Category             | Target      | Components Included                               |
| -------------------- | ----------- | ------------------------------------------------- |
| Static HTML/CSS      | 0 KB JS     | All Astro components, FadeIn, Starlight built-ins |
| React islands (home) | < 20 KB     | GraphAnimation (4KB) + ThemeToggle (2KB)          |
| React islands (demo) | < 30 KB     | DemoModeSelector (5KB) + ResultPanel (8KB)        |
| Pagefind search      | ~15 KB      | Loaded on-demand (Cmd+K)                          |
| Fonts (Inter)        | ~20 KB      | WOFF2 subset via Fontsource                       |
| Fonts (JetBrains)    | ~15 KB      | WOFF2 subset via Fontsource                       |
| **Total per page**   | **< 50 KB** | First load JS + CSS (excluding fonts)             |

> **Comparison:** Current Next.js site ships ~150KB JS (React runtime + framer-motion + shadcn). The Astro migration targets a 3× reduction by eliminating client-side routing and animation framework.

---

## 11. Accessibility per Component

| Component          | ARIA                             | Keyboard                     | Focus        |
| ------------------ | -------------------------------- | ---------------------------- | ------------ |
| Button             | `role="link"` if href            | Enter/Space activates        | Visible ring |
| Header mobile menu | `aria-expanded`, `aria-controls` | Esc closes, Tab trap in menu | First item   |
| ThemeToggle        | `aria-label` dynamic             | Enter toggles                | Visible ring |
| GraphAnimation     | `aria-hidden="true"`             | Not focusable (decorative)   | N/A          |
| DemoModeSelector   | `role="tablist"`, `role="tab"`   | Arrow keys navigate          | Active tab   |
| ContactForm        | Labels linked via `htmlFor`      | Tab through fields           | Visible ring |
| Pagefind search    | `role="dialog"`, `aria-modal`    | Esc closes, Tab within modal | Search input |
| Cards (marketing)  | If link: card is `<a>`           | Enter follows link           | Visible ring |
| Sidebar (docs)     | `nav` landmark, `aria-current`   | Tab through links            | Current item |

---

## 12. Testing Strategy

| Test Type     | Tool                     | Scope                                                |
| ------------- | ------------------------ | ---------------------------------------------------- |
| Visual        | Playwright screenshots   | All pages at 3 breakpoints (mobile, tablet, desktop) |
| Accessibility | Playwright + axe-core    | Automated A11y audit per page                        |
| Bundle size   | Custom script            | Verify `< 50KB` JS per page                          |
| Hydration     | Playwright               | Verify islands load and function                     |
| Cross-browser | Playwright multi-browser | Chrome, Firefox, Safari                              |

---

_Cross-references: [00-overview](./00-overview.md) · [04-project-setup](./04-starlight-project-setup.md) · [09-migration-roadmap](./09-migration-roadmap.md) · [11-design-system](./11-design-system.md) · [12-page-layouts](./12-page-layouts-wireframes.md)_
