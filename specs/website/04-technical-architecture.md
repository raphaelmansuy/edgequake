# SPEC-WEBSITE-04: Technical Architecture

> **Status**: DRAFT  
> **Created**: 2026-03-21  
> **Parent**: [00-overview.md](./00-overview.md)  
> **Related**: [03-page-specifications.md](./03-page-specifications.md) · [07-component-library.md](./07-component-library.md) · [09-implementation-roadmap.md](./09-implementation-roadmap.md)

---

## 1. Technology Stack

| Layer             | Technology     | Version | Purpose                                |
| ----------------- | -------------- | ------- | -------------------------------------- |
| Framework         | Next.js        | 16.x    | Static site generation with App Router |
| Language          | TypeScript     | 5.x     | Type safety                            |
| Styling           | Tailwind CSS   | 4.x     | Utility-first CSS                      |
| Components        | shadcn/ui      | latest  | Pre-built accessible components        |
| Primitives        | Radix UI       | latest  | Headless UI primitives                 |
| Animations        | Framer Motion  | 12.x    | Scroll/entrance animations             |
| Code Highlighting | Shiki          | 3.x     | Build-time syntax highlighting         |
| Content           | MDX            | 3.x     | Markdown with JSX for docs             |
| Search            | Pagefind       | 1.x     | Static search index                    |
| Graph Viz         | D3.js          | 7.x     | Demo knowledge graph                   |
| Icons             | Lucide React   | latest  | Consistent icon set                    |
| Package Manager   | pnpm           | 10.x    | Fast, disk-efficient                   |
| Deploy            | GitHub Pages   | —       | Free static hosting                    |
| CI/CD             | GitHub Actions | —       | Build + deploy pipeline                |

---

## 2. Project Structure

```
edgequake-website/
├── .github/
│   └── workflows/
│       └── deploy.yml             # GitHub Actions: build → deploy
├── public/
│   ├── og/                        # OpenGraph images per page
│   │   ├── home.png
│   │   ├── docs.png
│   │   └── demo.png
│   ├── logos/
│   │   ├── edgequake.svg
│   │   └── edgequake-dark.svg
│   ├── favicon.ico
│   ├── robots.txt
│   └── CNAME                      # edgequake.dev
├── content/
│   └── docs/                      # MDX documentation content
│       ├── getting-started/
│       │   ├── installation.mdx
│       │   ├── quick-start.mdx
│       │   └── first-ingestion.mdx
│       ├── concepts/
│       │   ├── graph-rag.mdx
│       │   ├── entity-extraction.mdx
│       │   ├── knowledge-graphs.mdx
│       │   ├── query-modes.mdx
│       │   └── hybrid-retrieval.mdx
│       ├── architecture/
│       │   ├── system-overview.mdx
│       │   ├── pipeline.mdx
│       │   ├── storage.mdx
│       │   └── llm-providers.mdx
│       ├── guides/
│       │   ├── pdf-ingestion.mdx
│       │   ├── mcp-integration.mdx
│       │   ├── docker-deployment.mdx
│       │   └── hybrid-providers.mdx
│       ├── api-reference/
│       │   ├── rest-api.mdx
│       │   ├── rust-sdk.mdx
│       │   └── mcp-tools.mdx
│       ├── deployment/
│       │   ├── docker.mdx
│       │   ├── kubernetes.mdx
│       │   └── configuration.mdx
│       └── comparisons/
│           ├── vs-lightrag.mdx
│           ├── vs-graphrag.mdx
│           └── vs-traditional-rag.mdx
├── src/
│   ├── app/
│   │   ├── layout.tsx             # Root layout (header + footer)
│   │   ├── page.tsx               # Homepage
│   │   ├── not-found.tsx          # Custom 404
│   │   ├── demo/
│   │   │   └── page.tsx           # Interactive demo
│   │   ├── ecosystem/
│   │   │   └── page.tsx           # Crate showcase
│   │   ├── enterprise/
│   │   │   └── page.tsx           # Enterprise landing
│   │   ├── contact/
│   │   │   └── page.tsx           # Contact form
│   │   ├── changelog/
│   │   │   └── page.tsx           # Version history
│   │   └── docs/
│   │       ├── layout.tsx         # Sidebar docs layout
│   │       ├── page.tsx           # Docs hub /docs
│   │       └── [...slug]/
│   │           └── page.tsx       # Dynamic doc pages
│   ├── components/
│   │   ├── ui/                    # shadcn/ui components
│   │   │   ├── button.tsx
│   │   │   ├── card.tsx
│   │   │   ├── tabs.tsx
│   │   │   ├── sheet.tsx
│   │   │   ├── navigation-menu.tsx
│   │   │   ├── accordion.tsx
│   │   │   ├── badge.tsx
│   │   │   ├── input.tsx
│   │   │   ├── select.tsx
│   │   │   ├── textarea.tsx
│   │   │   └── separator.tsx
│   │   ├── layout/
│   │   │   ├── header.tsx
│   │   │   ├── footer.tsx
│   │   │   ├── mobile-nav.tsx
│   │   │   └── docs-sidebar.tsx
│   │   ├── home/
│   │   │   ├── hero.tsx
│   │   │   ├── problem-section.tsx
│   │   │   ├── solution-grid.tsx
│   │   │   ├── architecture-diagram.tsx
│   │   │   ├── benchmark-chart.tsx
│   │   │   ├── quick-start.tsx
│   │   │   ├── ecosystem-preview.tsx
│   │   │   └── enterprise-cta.tsx
│   │   ├── demo/
│   │   │   ├── query-panel.tsx
│   │   │   ├── graph-viewer.tsx
│   │   │   └── result-panel.tsx
│   │   ├── ecosystem/
│   │   │   ├── crate-card.tsx
│   │   │   └── integration-card.tsx
│   │   ├── contact/
│   │   │   └── contact-form.tsx
│   │   └── shared/
│   │       ├── code-block.tsx
│   │       ├── mdx-components.tsx
│   │       ├── seo-head.tsx
│   │       └── scroll-animation.tsx
│   ├── lib/
│   │   ├── content.ts             # MDX loading + frontmatter
│   │   ├── demo-data.ts           # Pre-computed demo results
│   │   ├── crates.ts              # Crate metadata
│   │   └── utils.ts               # cn() helper, etc.
│   └── styles/
│       └── globals.css            # Tailwind directives + custom vars
├── next.config.ts
├── tailwind.config.ts
├── tsconfig.json
├── package.json
└── README.md
```

---

## 3. Next.js Configuration

### 3.1 `next.config.ts`

```typescript
import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  output: "export", // Static HTML export
  basePath: "", // "/" for custom domain; "/edgequake" for GH pages fallback
  trailingSlash: true, // Ensure clean URLs on static hosting
  images: {
    unoptimized: true, // Required for static export
  },
};

export default nextConfig;
```

### 3.2 Key Constraints (Static Export)

| Feature                            | Supported | Notes                                     |
| ---------------------------------- | --------- | ----------------------------------------- |
| Server Components                  | Yes       | Run at build time                         |
| Client Components (`"use client"`) | Yes       | Hydrate on load                           |
| `generateStaticParams()`           | Yes       | Required for dynamic routes (`[...slug]`) |
| `generateMetadata()`               | Yes       | Build-time OpenGraph/SEO                  |
| Image Optimization                 | No        | Use `unoptimized: true`                   |
| Route Handlers (GET)               | Yes       | Pre-rendered at build                     |
| Middleware                         | No        | Requires server                           |
| `cookies()`/`headers()`            | No        | Requires server                           |
| ISR / `revalidate`                 | No        | Requires server                           |

---

## 4. Content Pipeline

### 4.1 MDX Processing Flow

```
┌──────────────────┐     ┌──────────────┐     ┌──────────────┐
│  content/docs/   │────►│  MDX Loader  │────►│  React       │
│  *.mdx files     │     │  (next-mdx)  │     │  Components  │
│                  │     │              │     │              │
│  ---             │     │  • Frontmatter│    │  Rendered at │
│  title: ...      │     │  • Shiki code │    │  build time  │
│  ---             │     │  • Custom     │     │              │
│  # Heading       │     │    components │     │              │
└──────────────────┘     └──────────────┘     └──────────────┘
                                                     │
                                                     ▼
                                              ┌──────────────┐
                                              │  out/ folder  │
                                              │  Static HTML  │
                                              └──────────────┘
```

### 4.2 Frontmatter Schema

```yaml
---
title: "Entity Extraction"
description: "How EdgeQuake uses LLMs to extract entities and relationships"
category: "concepts"
order: 2
lastUpdated: "2026-03-15"
editUrl: "https://github.com/raphaelmansuy/edgequake/edit/main/docs/concepts/entity-extraction.md"
---
```

### 4.3 MDX Custom Components

| Component        | Usage in MDX          | Purpose                          |
| ---------------- | --------------------- | -------------------------------- |
| `<CodeBlock>`    | Code blocks with tabs | Multi-language code samples      |
| `<Callout>`      | Tips, warnings, notes | Highlighted content blocks       |
| `<Architecture>` | SVG diagrams          | Interactive architecture visuals |
| `<CrateLink>`    | Link to crate         | Auto-resolved crate reference    |
| `<ApiEndpoint>`  | REST endpoint doc     | Formatted API documentation      |

---

## 5. Build & Deploy Pipeline

### 5.1 GitHub Actions Workflow

```yaml
# .github/workflows/deploy.yml
name: Deploy to GitHub Pages

on:
  push:
    branches: [main]
  workflow_dispatch:

permissions:
  contents: read
  pages: write
  id-token: write

concurrency:
  group: "pages"
  cancel-in-progress: false

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
        with:
          version: 10
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: "pnpm"
          cache-dependency-path: "edgequake-website/pnpm-lock.yaml"
      - name: Install dependencies
        working-directory: edgequake-website
        run: pnpm install --frozen-lockfile
      - name: Build
        working-directory: edgequake-website
        run: pnpm build
      - name: Build search index
        working-directory: edgequake-website
        run: pnpm exec pagefind --site out
      - uses: actions/upload-pages-artifact@v3
        with:
          path: edgequake-website/out

  deploy:
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    runs-on: ubuntu-latest
    needs: build
    steps:
      - id: deployment
        uses: actions/deploy-pages@v4
```

### 5.2 Build Pipeline Diagram

```
┌──────────┐    ┌────────────┐    ┌────────────┐    ┌──────────┐
│  git push│───►│ GitHub     │───►│ pnpm build │───►│ Pagefind │
│  (main)  │    │ Actions    │    │ (next      │    │ (search  │
│          │    │            │    │  build +   │    │  index)  │
│          │    │            │    │  export)   │    │          │
└──────────┘    └────────────┘    └────────────┘    └──────────┘
                                                         │
                                                         ▼
                                                  ┌──────────┐
                                                  │ GitHub   │
                                                  │ Pages    │
                                                  │ Deploy   │
                                                  └──────────┘
```

### 5.3 Build Outputs

| Artifact     | Path                       | Size Target     |
| ------------ | -------------------------- | --------------- |
| HTML pages   | `out/*.html`               | < 15KB each     |
| CSS bundle   | `out/_next/static/css/`    | < 40KB gzipped  |
| JS bundle    | `out/_next/static/chunks/` | < 150KB gzipped |
| Search index | `out/pagefind/`            | < 200KB         |
| OG images    | `out/og/`                  | < 100KB each    |
| Total site   | `out/`                     | < 5MB           |

---

## 6. Performance Architecture

### 6.1 Rendering Strategy

| Page Type    | Strategy                        | Rationale           |
| ------------ | ------------------------------- | ------------------- |
| Homepage     | Static (Server Component)       | Maximum performance |
| Docs pages   | Static (MDX at build time)      | SEO + speed         |
| Demo page    | Static shell + Client hydration | D3.js needs DOM     |
| Contact form | Static shell + Client hydration | Form state needs JS |
| Ecosystem    | Static (Server Component)       | Data from JSON      |

### 6.2 Code Splitting

```
out/_next/static/chunks/
├── main-[hash].js              # React runtime (~40KB)
├── pages/
│   ├── index-[hash].js         # Homepage-specific (~30KB)
│   ├── demo-[hash].js          # D3.js + graph viz (~100KB)
│   └── contact-[hash].js       # Form logic (~15KB)
├── components/
│   ├── code-block-[hash].js    # Loaded for docs only
│   └── graph-viewer-[hash].js  # Loaded for demo only
└── framework-[hash].js         # Next.js framework (~30KB)
```

### 6.3 Asset Optimization

| Asset Type | Strategy                                                      |
| ---------- | ------------------------------------------------------------- |
| Images     | WebP/AVIF format, `loading="lazy"`, explicit `width`/`height` |
| Fonts      | System font stack (no web fonts) or self-hosted Inter subset  |
| SVGs       | Inline for critical, lazy-loaded for non-critical             |
| OG images  | Pre-generated at build time (1200x630px PNG)                  |
| CSS        | Tailwind JIT purging (only used classes shipped)              |

---

## 7. Domain & Hosting

### 7.1 Primary: Custom Domain

```
edgequake.dev  →  CNAME → raphaelmansuy.github.io
```

**DNS Configuration:**

```
Type   Name    Value
CNAME  www     raphaelmansuy.github.io
A      @       185.199.108.153
A      @       185.199.109.153
A      @       185.199.110.153
A      @       185.199.111.153
```

### 7.2 Fallback: GitHub Pages Default

```
raphaelmansuy.github.io/edgequake
```

Requires `basePath: "/edgequake"` in `next.config.ts`.

---

## 8. Third-Party Services

| Service                    | Purpose                 | Integration                          |
| -------------------------- | ----------------------- | ------------------------------------ |
| **Formspree**              | Contact form backend    | `action` URL in form, no server code |
| **Plausible** or **Umami** | Privacy-first analytics | `<script>` tag, no cookies           |
| **GitHub API**             | Star count badge        | Static fetch at build time           |
| **Pagefind**               | Client-side search      | Build-time index generation          |

All services selected for zero-server-dependency compatibility with static export.

---

## 9. Local Development

```bash
# Clone and setup
git clone https://github.com/raphaelmansuy/edgequake.git
cd edgequake/edgequake-website

# Install dependencies
pnpm install

# Start dev server
pnpm dev          # http://localhost:3000

# Build for production
pnpm build        # Generates out/

# Preview production build
pnpm exec serve out

# Lint + type check
pnpm lint && pnpm type-check
```

---

## 10. Environment Variables

| Variable                   | Required | Purpose                  | Example                 |
| -------------------------- | -------- | ------------------------ | ----------------------- |
| `NEXT_PUBLIC_SITE_URL`     | Yes      | Canonical URL            | `https://edgequake.dev` |
| `NEXT_PUBLIC_FORMSPREE_ID` | Yes      | Contact form endpoint    | `xpzgjdkl`              |
| `NEXT_PUBLIC_ANALYTICS_ID` | No       | Plausible/Umami site ID  | `edgequake.dev`         |
| `GITHUB_TOKEN`             | No       | Star count at build time | `ghp_...`               |

Note: All `NEXT_PUBLIC_` variables are embedded at build time and visible in client JS. No secrets should use this prefix.

---

_Previous: [03-page-specifications.md](./03-page-specifications.md) · Next: [05-seo-strategy.md](./05-seo-strategy.md)_
