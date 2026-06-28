# SPEC-008-04: Starlight Project Setup

| Field      | Value                        |
| ---------- | ---------------------------- |
| **Parent** | [SPEC-008](./00-overview.md) |
| **Status** | Draft                        |
| **Date**   | 2026-03-21                   |

---

## 1. Overview

This document specifies the concrete directory structure, configuration files, and dependency setup for the unified Astro + Starlight project that replaces the current Next.js marketing site.

The project lives in `edgequake-website/` and serves:

- **Marketing pages** via `src/pages/` (Astro file-based routing)
- **Documentation** via `src/content/docs/` (Starlight content collections, symlinked to `docs/`)

---

## 2. Directory Structure

```
edgequake-website/
|
+-- astro.config.mjs              Astro + Starlight integration config
+-- package.json                  Dependencies and scripts
+-- tsconfig.json                 TypeScript configuration
+-- src/
|   +-- content.config.ts         Content collection definitions (docsLoader, docsSchema)
|   +-- assets/                   Optimized images, logos
|   |   +-- logo-light.svg
|   |   +-- logo-dark.svg
|   |   +-- og-image.png
|   +-- components/               Shared Astro + React components
|   |   +-- Header.astro          Global navigation header
|   |   +-- Footer.astro          Global footer
|   |   +-- ThemeToggle.astro     Dark/light mode toggle
|   |   +-- sections/             Homepage section components
|   |   |   +-- Hero.astro
|   |   |   +-- Problem.astro
|   |   |   +-- Solution.astro
|   |   |   +-- Architecture.astro
|   |   |   +-- Benchmarks.astro
|   |   |   +-- QuickStart.astro
|   |   |   +-- Ecosystem.astro
|   |   |   +-- EnterpriseCTA.astro
|   |   +-- islands/              React components (client-side interactivity)
|   |       +-- GraphAnimation.tsx    Canvas-based graph (React island)
|   |       +-- DemoModeSelector.tsx  Demo page mode tabs (React island)
|   |       +-- ContactForm.tsx       Contact form state (React island)
|   +-- content/
|   |   +-- docs/                 SYMLINK -> ../../../docs/
|   +-- layouts/
|   |   +-- MarketingLayout.astro Base layout for marketing pages
|   +-- pages/                    Astro file-based routing
|   |   +-- index.astro           / (home)
|   |   +-- demo.astro            /demo/
|   |   +-- ecosystem.astro       /ecosystem/
|   |   +-- enterprise.astro      /enterprise/
|   |   +-- contact.astro         /contact/
|   |   +-- 404.astro             Custom 404 page
|   +-- styles/
|       +-- global.css            Global styles, CSS custom properties, Tailwind
+-- public/
    +-- favicon.svg
    +-- CNAME                     edgequake.com
    +-- fonts/                    Self-hosted Inter font (optional)
```

---

## 3. Astro Configuration

```javascript
// astro.config.mjs
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import react from "@astrojs/react";
import tailwind from "@astrojs/tailwind";
import sitemap from "@astrojs/sitemap";

export default defineConfig({
  site: "https://edgequake.com",
  trailingSlash: "always",
  output: "static",

  integrations: [
    starlight({
      title: "EdgeQuake",
      description:
        "Graph-RAG framework built for speed. Knowledge graph engine powered by Rust.",
      logo: {
        light: "./src/assets/logo-light.svg",
        dark: "./src/assets/logo-dark.svg",
        replacesTitle: false,
      },
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/edgequake/edgequake",
        },
      ],
      editLink: {
        baseUrl: "https://github.com/edgequake/edgequake/edit/main/",
      },
      customCss: ["./src/styles/global.css"],
      lastUpdated: true,
      pagination: true,
      disable404Route: true, // We provide our own src/pages/404.astro
      sidebar: [
        {
          label: "Getting Started",
          autogenerate: { directory: "getting-started" },
        },
        {
          label: "Concepts",
          autogenerate: { directory: "concepts" },
        },
        {
          label: "Architecture",
          autogenerate: { directory: "architecture" },
        },
        {
          label: "Tutorials",
          autogenerate: { directory: "tutorials" },
        },
        {
          label: "API Reference",
          autogenerate: { directory: "api-reference" },
        },
        {
          label: "Deep Dives",
          collapsed: true,
          autogenerate: { directory: "deep-dives" },
        },
        {
          label: "Operations",
          collapsed: true,
          autogenerate: { directory: "operations" },
        },
        {
          label: "Integrations",
          autogenerate: { directory: "integrations" },
        },
        {
          label: "Comparisons",
          collapsed: true,
          autogenerate: { directory: "comparisons" },
        },
        {
          label: "Security",
          autogenerate: { directory: "security" },
        },
        {
          label: "Troubleshooting",
          autogenerate: { directory: "troubleshooting" },
        },
        {
          label: "Resources",
          items: [{ slug: "cookbook" }, { slug: "faq" }, { slug: "features" }],
        },
      ],
    }),
    react(), // Enables React islands for interactive components
    tailwind({
      applyBaseStyles: false, // We manage base styles via global.css
    }),
    sitemap(),
  ],
});
```

### 3.1 Key Configuration Choices

| Setting                      | Value                   | Why                                                    |
| ---------------------------- | ----------------------- | ------------------------------------------------------ |
| `output: 'static'`           | Static HTML             | Same as current Next.js `output: "export"` — full SSG  |
| `trailingSlash: 'always'`    | `/docs/` not `/docs`    | Matches current Next.js behavior                       |
| `disable404Route: true`      | Custom 404              | We port the existing 404 page to `src/pages/404.astro` |
| `lastUpdated: true`          | Git-based dates         | Shows freshness, builds trust                          |
| Deep Dives `collapsed: true` | Collapsed sidebar group | 13 items — too many to show expanded                   |
| `react()` integration        | React islands           | Enables `client:visible` on React components           |
| `tailwind()` integration     | Tailwind CSS            | Reuse existing Tailwind classes from Next.js           |
| `sitemap()` integration      | /sitemap.xml            | Unified sitemap for SEO                                |

---

## 4. Content Collections Configuration

```typescript
// src/content.config.ts
import { defineCollection } from "astro:content";
import { docsLoader } from "@astrojs/starlight/loaders";
import { docsSchema } from "@astrojs/starlight/schema";

export const collections = {
  docs: defineCollection({
    loader: docsLoader(),
    schema: docsSchema(),
  }),
};
```

This configuration tells Starlight to:

1. Load markdown files from `src/content/docs/` (which is our symlink to `docs/`)
2. Validate frontmatter using Starlight's built-in Zod schema (requires `title` field)

---

## 5. Marketing Layout

Marketing pages share a common layout distinct from Starlight's docs layout:

```astro
---
// src/layouts/MarketingLayout.astro
import Header from '../components/Header.astro';
import Footer from '../components/Footer.astro';
import '../styles/global.css';

interface Props {
  title: string;
  description?: string;
}

const { title, description } = Astro.props;
const siteTitle = `${title} | EdgeQuake`;
---

<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <meta name="description" content={description} />
    <meta property="og:title" content={siteTitle} />
    <meta property="og:description" content={description} />
    <meta property="og:type" content="website" />
    <meta property="og:url" content={Astro.url.href} />
    <title>{siteTitle}</title>
    <link rel="icon" type="image/svg+xml" href="/favicon.svg" />
  </head>
  <body>
    <Header />
    <main>
      <slot />
    </main>
    <Footer />
  </body>
</html>
```

### 5.1 Page Example: Home

```astro
---
// src/pages/index.astro
import MarketingLayout from '../layouts/MarketingLayout.astro';
import Hero from '../components/sections/Hero.astro';
import Problem from '../components/sections/Problem.astro';
import Solution from '../components/sections/Solution.astro';
import Architecture from '../components/sections/Architecture.astro';
import Benchmarks from '../components/sections/Benchmarks.astro';
import QuickStart from '../components/sections/QuickStart.astro';
import Ecosystem from '../components/sections/Ecosystem.astro';
import EnterpriseCTA from '../components/sections/EnterpriseCTA.astro';
---

<MarketingLayout title="Graph-RAG Built for Speed" description="EdgeQuake is a Rust-powered Graph-RAG framework with knowledge graph engine, 6 query modes, and sub-100ms latency.">
  <Hero />
  <Problem />
  <Solution />
  <Architecture />
  <Benchmarks />
  <QuickStart />
  <Ecosystem />
  <EnterpriseCTA />
</MarketingLayout>
```

---

## 6. React Islands

Interactive components from the Next.js site are preserved as React islands. Astro loads them client-side only where needed.

### 6.1 Graph Animation Island

```astro
---
// Inside src/components/sections/Hero.astro
import GraphAnimation from '../islands/GraphAnimation';
---

<section class="hero">
  <div class="hero-content">
    <h1>Graph-RAG. Built for Speed.</h1>
    <!-- Static content here -->
  </div>
  <div class="hero-animation">
    <!-- client:visible = load React only when element scrolls into view -->
    <GraphAnimation client:visible />
  </div>
</section>
```

### 6.2 Island Strategy

| Component          | Directive        | Rationale                                 |
| ------------------ | ---------------- | ----------------------------------------- |
| `GraphAnimation`   | `client:visible` | Heavy canvas code; only load when visible |
| `DemoModeSelector` | `client:load`    | Core to demo page; load immediately       |
| `ContactForm`      | `client:visible` | Form only needed when user scrolls to it  |
| `ThemeToggle`      | `client:load`    | Immediate interaction expected            |

### 6.3 What Does NOT Need a React Island

Most Next.js components are purely presentational. These become plain Astro components:

| Component              | Migration                                              |
| ---------------------- | ------------------------------------------------------ |
| `Header`               | Astro component (HTML + CSS)                           |
| `Footer`               | Astro component (HTML + CSS)                           |
| `Hero` (text + badges) | Astro component                                        |
| `ProblemSection`       | Astro component (icon cards)                           |
| `SolutionSection`      | Astro component (icon cards)                           |
| `ArchitectureSection`  | Astro component (inline SVG)                           |
| `BenchmarksSection`    | Astro component (CSS animations replace Framer Motion) |
| `EcosystemSection`     | Astro component (card grid)                            |
| `EnterpriseCTA`        | Astro component (card + link)                          |
| All shadcn UI wrappers | Plain HTML + Tailwind classes                          |

---

## 7. Dependencies

### 7.1 Production Dependencies

```json
{
  "dependencies": {
    "astro": "^5.x",
    "@astrojs/starlight": "^0.33.x",
    "@astrojs/react": "^4.x",
    "@astrojs/tailwind": "^6.x",
    "@astrojs/sitemap": "^3.x",
    "react": "^19.x",
    "react-dom": "^19.x",
    "tailwindcss": "^4.x",
    "lucide-react": "^0.577.x"
  }
}
```

### 7.2 Removed Dependencies (No Longer Needed)

| Dependency                 | Why Removed                                                     |
| -------------------------- | --------------------------------------------------------------- |
| `next`                     | Replaced by Astro                                               |
| `framer-motion`            | CSS animations for static effects; React island only for canvas |
| `next-themes`              | Astro's built-in theme support or simple script                 |
| `@base-ui/react`           | Not needed — Astro components are HTML-native                   |
| `shadcn`                   | Components replaced by Tailwind + HTML                          |
| `shiki`                    | Starlight's Expressive Code handles syntax highlighting         |
| `class-variance-authority` | Simplified — Astro components use direct Tailwind               |
| `clsx`                     | `class:list` directive built into Astro                         |
| `tailwind-merge`           | Reduced need with Astro's templating                            |
| `tw-animate-css`           | CSS animations directly in global.css                           |

### 7.3 Net Dependency Reduction

```
Next.js site: 12 production + 6 dev = 18 dependencies
Astro site:   7 production + 2 dev  = 9 dependencies  (50% reduction)
```

---

## 8. TypeScript Configuration

```json
{
  "extends": "astro/tsconfigs/strict",
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@/*": ["./src/*"]
    }
  }
}
```

---

## 9. Build & Development Scripts

```json
{
  "scripts": {
    "dev": "astro dev",
    "build": "astro build",
    "preview": "astro preview",
    "check": "astro check",
    "lint": "astro check && tsc --noEmit"
  }
}
```

| Command        | Purpose                   |
| -------------- | ------------------------- |
| `pnpm dev`     | Local dev server with HMR |
| `pnpm build`   | Static build to `dist/`   |
| `pnpm preview` | Serve built site locally  |
| `pnpm check`   | Validate Astro files      |
| `pnpm lint`    | Full type check           |

---

## 10. Cross-References

- [00-overview.md](./00-overview.md) — Goals G7 (migrate pages) and G8 (single deployment)
- [01-architecture-decision-record.md](./01-architecture-decision-record.md) — Why Astro + Starlight
- [02-single-source-strategy.md](./02-single-source-strategy.md) — Symlink to `docs/`
- [03-information-architecture.md](./03-information-architecture.md) — Sidebar configuration source
- [05-build-pipeline.md](./05-build-pipeline.md) — CI/CD build steps
- [09-migration-roadmap.md](./09-migration-roadmap.md) — Phased implementation plan
