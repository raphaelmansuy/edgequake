# SPEC-WEBSITE-09: Implementation Roadmap

> **Status**: DRAFT  
> **Created**: 2026-03-21  
> **Parent**: [00-overview.md](./00-overview.md)  
> **Related**: [04-technical-architecture.md](./04-technical-architecture.md) · [09-implementation-roadmap.md](./09-implementation-roadmap.md)

---

## 1. Phased Delivery Plan

```
┌─────────────────────────────────────────────────────────────────────┐
│                                                                      │
│  PHASE 1                PHASE 2                PHASE 3               │
│  Foundation             Content Engine          Polish & Launch       │
│  ──────────             ──────────────          ───────────────       │
│  Week 1-2               Week 3-4                Week 5-6              │
│                                                                      │
│  • Project scaffold     • Docs hub (MDX)       • Demo page (D3)     │
│  • Design system        • 25+ doc pages        • SEO audit           │
│  • Header / Footer      • Pagefind search      • OG images           │
│  • Homepage sections    • API reference         • Performance audit   │
│  • Contact form         • Guides & tutorials   • Accessibility audit │
│  • Dark mode            • Comparisons          • Analytics setup      │
│  • GitHub Actions CI    • Ecosystem page       • Custom domain        │
│  • Enterprise page      • Changelog page       • Launch checklist     │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 2. Phase 1: Foundation (Week 1-2)

### 2.1 Tasks

<<<<<<< HEAD
| # | Task | Deliverable | Acceptance Criteria |
|---|------|-------------|-------------------|
| 1.1 | Project Scaffold | Next.js 16 project with Tailwind, shadcn, TypeScript | `pnpm dev` starts, `pnpm build` succeeds |
| 1.2 | Design System Tokens | CSS variables in `globals.css` per [06-design-system.md](./06-design-system.md) | Light + dark mode toggle works |
| 1.3 | Layout Shell | Header + Footer + theme toggle | Responsive at all breakpoints |
| 1.4 | Homepage — Hero | Animated hero with CTAs and badge row | Lighthouse performance ≥ 90 |
| 1.5 | Homepage — Problem | 6 pain cards in 3-column grid | Cards animate on scroll |
| 1.6 | Homepage — Solution | 6 feature cards with links | All links resolve to /docs stubs |
| 1.7 | Homepage — Architecture | SVG diagram with hover tooltips | Interactive tooltips work |
| 1.8 | Homepage — Benchmarks | 4 horizontal bar charts | Bars animate on scroll entry |
| 1.9 | Homepage — Quick Start | 3-tab code block (Rust/Docker/API) | Copy button works |
| 1.10 | Homepage — Ecosystem Preview | 5 crate cards + "Explore All" link | Cards link to /ecosystem |
| 1.11 | Homepage — Enterprise CTA | Full-width banner with CTA | Links to /contact |
| 1.12 | Contact Page | Full form with Formspree integration | Submit succeeds, email received |
| 1.13 | Enterprise Page | Feature list + use cases + CTA | Links to /contact |
| 1.14 | GitHub Actions | Build + deploy to GitHub Pages | Auto-deploys on push to main |
| 1.15 | 404 Page | Custom not-found with links | Matches site design |
=======
| #    | Task                         | Deliverable                                                                     | Acceptance Criteria                      |
| ---- | ---------------------------- | ------------------------------------------------------------------------------- | ---------------------------------------- |
| 1.1  | Project Scaffold             | Next.js 16 project with Tailwind, shadcn, TypeScript                            | `pnpm dev` starts, `pnpm build` succeeds |
| 1.2  | Design System Tokens         | CSS variables in `globals.css` per [06-design-system.md](./06-design-system.md) | Light + dark mode toggle works           |
| 1.3  | Layout Shell                 | Header + Footer + theme toggle                                                  | Responsive at all breakpoints            |
| 1.4  | Homepage — Hero              | Animated hero with CTAs and badge row                                           | Lighthouse performance ≥ 90              |
| 1.5  | Homepage — Problem           | 6 pain cards in 3-column grid                                                   | Cards animate on scroll                  |
| 1.6  | Homepage — Solution          | 6 feature cards with links                                                      | All links resolve to /docs stubs         |
| 1.7  | Homepage — Architecture      | SVG diagram with hover tooltips                                                 | Interactive tooltips work                |
| 1.8  | Homepage — Benchmarks        | 4 horizontal bar charts                                                         | Bars animate on scroll entry             |
| 1.9  | Homepage — Quick Start       | 3-tab code block (Rust/Docker/API)                                              | Copy button works                        |
| 1.10 | Homepage — Ecosystem Preview | 5 crate cards + "Explore All" link                                              | Cards link to /ecosystem                 |
| 1.11 | Homepage — Enterprise CTA    | Full-width banner with CTA                                                      | Links to /contact                        |
| 1.12 | Contact Page                 | Full form with Formspree integration                                            | Submit succeeds, email received          |
| 1.13 | Enterprise Page              | Feature list + use cases + CTA                                                  | Links to /contact                        |
| 1.14 | GitHub Actions               | Build + deploy to GitHub Pages                                                  | Auto-deploys on push to main             |
| 1.15 | 404 Page                     | Custom not-found with links                                                     | Matches site design                      |
>>>>>>> origin/edgequake-main

### 2.2 Scaffold Commands

```bash
# Initialize project
pnpm create next-app@latest edgequake-website \
  --typescript --tailwind --eslint --app --src-dir \
  --import-alias "@/*"

cd edgequake-website

# Add shadcn/ui
pnpm dlx shadcn@latest init

# Install core shadcn components
pnpm dlx shadcn@latest add button card tabs sheet \
  navigation-menu accordion badge input select \
  textarea separator tooltip dropdown-menu dialog \
  scroll-area toggle skeleton

# Install additional dependencies
pnpm add framer-motion lucide-react next-themes shiki

# Dev dependencies
pnpm add -D @types/d3
```

### 2.3 Phase 1 Exit Criteria

- [ ] `pnpm build` produces `out/` with no errors
- [ ] Homepage loads in < 2.5s LCP
- [ ] Contact form submits successfully to Formspree
- [ ] Dark mode toggle works across all pages
- [ ] Mobile hamburger menu works
- [ ] GitHub Actions deploys successfully to Pages
- [ ] All CTA links navigate correctly

---

## 3. Phase 2: Content Engine (Week 3-4)

### 3.1 Tasks

<<<<<<< HEAD
| # | Task | Deliverable | Acceptance Criteria |
|---|------|-------------|-------------------|
| 2.1 | MDX Pipeline | Content pipeline with frontmatter + Shiki | MDX files render correctly |
| 2.2 | Docs Layout | Sidebar + breadcrumbs + prev/next | Navigation is consistent |
| 2.3 | Getting Started (3 pages) | Installation, Quick Start, First Ingestion | End-to-end flow is complete |
| 2.4 | Concepts (5 pages) | Graph-RAG, Entity Extraction, Knowledge Graphs, Query Modes, Hybrid Retrieval | Accurate technical content |
| 2.5 | Architecture (4 pages) | System Overview, Pipeline, Storage, LLM Providers | Diagrams included |
| 2.6 | Guides (4 pages) | PDF Ingestion, MCP Integration, Docker, Hybrid Providers | Step-by-step with code |
| 2.7 | API Reference (3 pages) | REST API, Rust SDK, MCP Tools | Endpoint tables complete |
| 2.8 | Deployment (3 pages) | Docker, Kubernetes, Configuration | Covers all env variables |
| 2.9 | Comparisons (3 pages) | vs LightRAG, vs GraphRAG, vs Traditional RAG | Fair, data-backed |
| 2.10 | Pagefind Search | Client-side search integration | Keyboard shortcut (Cmd+K) works |
| 2.11 | Ecosystem Page | 10 crate cards + 5 integration cards | Filter by category works |
| 2.12 | Changelog Page | Rendered from CHANGELOG.md | Versions displayed correctly |

### 3.2 Content Sourcing

| Content | Source | Transform |
|---------|--------|-----------|
| Getting Started | `docs/getting-started/` | Copy + adapt for web |
| Concepts | `docs/concepts/` | Copy + add SEO metadata |
| Architecture | `docs/architecture/` | Copy + add interactive diagrams |
| Guides | `docs/tutorials/` + `docs/integrations/` | Merge + restructure |
| API Reference | `docs/api-reference/` + crate doc comments | Compile + format |
| Comparisons | `docs/comparisons/` | Copy + update data |
| Changelog | `CHANGELOG.md` | Parse and render |
=======
| #    | Task                      | Deliverable                                                                   | Acceptance Criteria             |
| ---- | ------------------------- | ----------------------------------------------------------------------------- | ------------------------------- |
| 2.1  | MDX Pipeline              | Content pipeline with frontmatter + Shiki                                     | MDX files render correctly      |
| 2.2  | Docs Layout               | Sidebar + breadcrumbs + prev/next                                             | Navigation is consistent        |
| 2.3  | Getting Started (3 pages) | Installation, Quick Start, First Ingestion                                    | End-to-end flow is complete     |
| 2.4  | Concepts (5 pages)        | Graph-RAG, Entity Extraction, Knowledge Graphs, Query Modes, Hybrid Retrieval | Accurate technical content      |
| 2.5  | Architecture (4 pages)    | System Overview, Pipeline, Storage, LLM Providers                             | Diagrams included               |
| 2.6  | Guides (4 pages)          | PDF Ingestion, MCP Integration, Docker, Hybrid Providers                      | Step-by-step with code          |
| 2.7  | API Reference (3 pages)   | REST API, Rust SDK, MCP Tools                                                 | Endpoint tables complete        |
| 2.8  | Deployment (3 pages)      | Docker, Kubernetes, Configuration                                             | Covers all env variables        |
| 2.9  | Comparisons (3 pages)     | vs LightRAG, vs GraphRAG, vs Traditional RAG                                  | Fair, data-backed               |
| 2.10 | Pagefind Search           | Client-side search integration                                                | Keyboard shortcut (Cmd+K) works |
| 2.11 | Ecosystem Page            | 10 crate cards + 5 integration cards                                          | Filter by category works        |
| 2.12 | Changelog Page            | Rendered from CHANGELOG.md                                                    | Versions displayed correctly    |

### 3.2 Content Sourcing

| Content         | Source                                     | Transform                       |
| --------------- | ------------------------------------------ | ------------------------------- |
| Getting Started | `docs/getting-started/`                    | Copy + adapt for web            |
| Concepts        | `docs/concepts/`                           | Copy + add SEO metadata         |
| Architecture    | `docs/architecture/`                       | Copy + add interactive diagrams |
| Guides          | `docs/tutorials/` + `docs/integrations/`   | Merge + restructure             |
| API Reference   | `docs/api-reference/` + crate doc comments | Compile + format                |
| Comparisons     | `docs/comparisons/`                        | Copy + update data              |
| Changelog       | `CHANGELOG.md`                             | Parse and render                |
>>>>>>> origin/edgequake-main

### 3.3 Phase 2 Exit Criteria

- [ ] 25+ documentation pages render correctly
- [ ] Sidebar navigation works with collapsible sections
- [ ] Search returns relevant results for "entity extraction", "query modes", "docker"
- [ ] Prev/Next navigation flows logically through docs
- [ ] All internal links resolve (no 404s within docs)
- [ ] Code blocks highlighted with correct language
- [ ] Ecosystem page shows all 10 crates with working filters

---

## 4. Phase 3: Polish & Launch (Week 5-6)

### 4.1 Tasks

<<<<<<< HEAD
| # | Task | Deliverable | Acceptance Criteria |
|---|------|-------------|-------------------|
| 3.1 | Demo Page | Interactive demo with D3.js graph + 6 query modes | Graph renders, mode switching works |
| 3.2 | OG Images | Pre-generated images for all pages | Twitter/LinkedIn preview correct |
| 3.3 | JSON-LD | Structured data per [05-seo-strategy.md](./05-seo-strategy.md) | Rich Results Test passes |
| 3.4 | Sitemap | Auto-generated sitemap.xml | All pages included |
| 3.5 | Analytics | Plausible integration with custom events | Pageviews + CTA tracking live |
| 3.6 | Performance Audit | Lighthouse CI in GitHub Actions | All pages ≥ 90 performance |
| 3.7 | Accessibility Audit | axe-core scan | 0 critical issues |
| 3.8 | Custom Domain | DNS setup for edgequake.dev | HTTPS enforced |
| 3.9 | README | Update project README with website link | Links work |
| 3.10 | Launch Announcement | Draft LinkedIn/Twitter post | Includes OG image |
=======
| #    | Task                | Deliverable                                                    | Acceptance Criteria                 |
| ---- | ------------------- | -------------------------------------------------------------- | ----------------------------------- |
| 3.1  | Demo Page           | Interactive demo with D3.js graph + 6 query modes              | Graph renders, mode switching works |
| 3.2  | OG Images           | Pre-generated images for all pages                             | Twitter/LinkedIn preview correct    |
| 3.3  | JSON-LD             | Structured data per [05-seo-strategy.md](./05-seo-strategy.md) | Rich Results Test passes            |
| 3.4  | Sitemap             | Auto-generated sitemap.xml                                     | All pages included                  |
| 3.5  | Analytics           | Plausible integration with custom events                       | Pageviews + CTA tracking live       |
| 3.6  | Performance Audit   | Lighthouse CI in GitHub Actions                                | All pages ≥ 90 performance          |
| 3.7  | Accessibility Audit | axe-core scan                                                  | 0 critical issues                   |
| 3.8  | Custom Domain       | DNS setup for edgequake.dev                                    | HTTPS enforced                      |
| 3.9  | README              | Update project README with website link                        | Links work                          |
| 3.10 | Launch Announcement | Draft LinkedIn/Twitter post                                    | Includes OG image                   |
>>>>>>> origin/edgequake-main

### 4.2 Demo Data Generation

```bash
# Generate pre-computed demo data at build time
# Script processes sample documents through EdgeQuake,
# captures results for each query mode, exports as JSON

node scripts/generate-demo-data.mjs \
  --input zz_test_docs/lightrag_2410.05779v3.pdf \
  --output src/lib/demo-data.json \
  --modes naive,local,global,hybrid,mix,bypass
```

### 4.3 Phase 3 Exit Criteria

- [ ] Demo page D3.js graph renders with sample data
- [ ] All 6 query modes show different results for same question
- [ ] OG images display correctly on Twitter, LinkedIn, Slack
- [ ] Lighthouse score ≥ 90 on performance, accessibility, SEO, best practices
- [ ] sitemap.xml includes all published pages
- [ ] Custom domain resolves with HTTPS
- [ ] Plausible dashboard shows pageviews
- [ ] No broken links (checked with `lychee` or similar)

---

## 5. Launch Checklist

```
PRE-LAUNCH
──────────
[ ] All pages render correctly on Chrome, Firefox, Safari
[ ] Mobile responsive at 375px, 768px, 1024px, 1440px
[ ] Dark mode works on all pages
[ ] Contact form test submission received at contact@elitizon.com
[ ] All doc pages have correct meta descriptions
[ ] robots.txt allows crawling
[ ] sitemap.xml submitted to Google Search Console
[ ] Lighthouse CI passing in GitHub Actions
[ ] No console errors in browser DevTools
[ ] 404 page works for invalid routes

LAUNCH
──────
[ ] DNS CNAME configured for edgequake.dev
[ ] GitHub Pages custom domain + HTTPS enabled
[ ] Analytics tracking verified (Plausible live)
[ ] Announce on GitHub README
[ ] Announce on social media (LinkedIn, Twitter/X)
[ ] Submit to relevant directories:
      - Awesome Rust
      - Awesome RAG
      - Product Hunt (optional)
      - Hacker News Show HN (optional)

POST-LAUNCH
───────────
[ ] Monitor Plausible for first 48 hours
[ ] Respond to any contact form submissions
[ ] Fix any broken links reported
[ ] Monitor Google Search Console for indexing
[ ] Track Core Web Vitals in Search Console
```

---

## 6. Dependency List

### 6.1 Production Dependencies

<<<<<<< HEAD
| Package | Purpose | Size Impact |
|---------|---------|------------|
| `next` | Framework | Core (tree-shaken) |
| `react` + `react-dom` | UI library | Core |
| `framer-motion` | Animations | ~30KB gzipped |
| `lucide-react` | Icons | Tree-shaken per icon |
| `next-themes` | Dark mode | ~2KB |
| `shiki` | Syntax highlighting | Build-time only |
| `d3` | Graph visualization (demo) | ~20KB (force module only) |
| `@radix-ui/*` | UI primitives | Via shadcn, tree-shaken |

### 6.2 Dev Dependencies

| Package | Purpose |
|---------|---------|
| `typescript` | Type checking |
| `tailwindcss` | CSS framework |
| `eslint` | Linting |
| `@next/bundle-analyzer` | Bundle size monitoring |
| `pagefind` | Search index generation |
=======
| Package               | Purpose                    | Size Impact               |
| --------------------- | -------------------------- | ------------------------- |
| `next`                | Framework                  | Core (tree-shaken)        |
| `react` + `react-dom` | UI library                 | Core                      |
| `framer-motion`       | Animations                 | ~30KB gzipped             |
| `lucide-react`        | Icons                      | Tree-shaken per icon      |
| `next-themes`         | Dark mode                  | ~2KB                      |
| `shiki`               | Syntax highlighting        | Build-time only           |
| `d3`                  | Graph visualization (demo) | ~20KB (force module only) |
| `@radix-ui/*`         | UI primitives              | Via shadcn, tree-shaken   |

### 6.2 Dev Dependencies

| Package                 | Purpose                 |
| ----------------------- | ----------------------- |
| `typescript`            | Type checking           |
| `tailwindcss`           | CSS framework           |
| `eslint`                | Linting                 |
| `@next/bundle-analyzer` | Bundle size monitoring  |
| `pagefind`              | Search index generation |
>>>>>>> origin/edgequake-main

---

## 7. Risk Matrix

<<<<<<< HEAD
| Risk | Probability | Impact | Mitigation |
|------|------------|--------|-----------|
| Next.js static export limitations | Medium | Medium | Tested constraints in [04-technical-architecture.md](./04-technical-architecture.md) |
| D3.js bundle size bloat (demo) | Medium | Low | Import only `d3-force` + `d3-selection` modules |
| MDX build errors | Low | Medium | CI catches build failures before deploy |
| Formspree rate limits | Low | Low | 50/mo is sufficient; Resend fallback planned |
| SEO competition for "graph rag" | Medium | Medium | Long-tail keywords + niche terms give edge |
| Content staleness (docs vs code) | High | Medium | CI lint to check docs match latest release |
| GitHub Pages downtime | Very Low | High | Accept risk — free hosting, global CDN |
=======
| Risk                              | Probability | Impact | Mitigation                                                                           |
| --------------------------------- | ----------- | ------ | ------------------------------------------------------------------------------------ |
| Next.js static export limitations | Medium      | Medium | Tested constraints in [04-technical-architecture.md](./04-technical-architecture.md) |
| D3.js bundle size bloat (demo)    | Medium      | Low    | Import only `d3-force` + `d3-selection` modules                                      |
| MDX build errors                  | Low         | Medium | CI catches build failures before deploy                                              |
| Formspree rate limits             | Low         | Low    | 50/mo is sufficient; Resend fallback planned                                         |
| SEO competition for "graph rag"   | Medium      | Medium | Long-tail keywords + niche terms give edge                                           |
| Content staleness (docs vs code)  | High        | Medium | CI lint to check docs match latest release                                           |
| GitHub Pages downtime             | Very Low    | High   | Accept risk — free hosting, global CDN                                               |
>>>>>>> origin/edgequake-main

---

## 8. Maintenance Plan

<<<<<<< HEAD
| Cadence | Activity |
|---------|---------|
| Every release | Update changelog, version badge, benchmark data |
| Monthly | Review analytics, update keyword targets |
| Quarterly | Content audit — update docs with new features |
| Yearly | Refresh design, update dependencies |

---

*Previous: [08-contact-lead-generation.md](./08-contact-lead-generation.md) · Back to: [00-overview.md](./00-overview.md)*
=======
| Cadence       | Activity                                        |
| ------------- | ----------------------------------------------- |
| Every release | Update changelog, version badge, benchmark data |
| Monthly       | Review analytics, update keyword targets        |
| Quarterly     | Content audit — update docs with new features   |
| Yearly        | Refresh design, update dependencies             |

---

_Previous: [08-contact-lead-generation.md](./08-contact-lead-generation.md) · Back to: [00-overview.md](./00-overview.md)_
>>>>>>> origin/edgequake-main
