# SPEC-WEBSITE-00: EdgeQuake Website — Project Overview

> **Status**: DRAFT  
> **Created**: 2026-03-21  
> **Author**: EdgeQuake Team  
> **Related Specs**: [01-information-architecture](./01-information-architecture.md) · [02-content-strategy](./02-content-strategy.md) · [03-page-specifications](./03-page-specifications.md) · [04-technical-architecture](./04-technical-architecture.md) · [05-seo-strategy](./05-seo-strategy.md) · [06-design-system](./06-design-system.md) · [07-component-library](./07-component-library.md) · [08-contact-lead-generation](./08-contact-lead-generation.md) · [09-implementation-roadmap](./09-implementation-roadmap.md)

---

## 1. Executive Summary

This specification defines the **edgequake.dev** promotional website — a GitHub Pages-hosted, statically-exported Next.js site designed to:

1. **Explain** what EdgeQuake is and why it matters (WHY → WHAT → HOW)
2. **Convert** visitors into users, contributors, and enterprise leads
3. **Rank** for Graph-RAG and Rust RAG search queries via aggressive SEO
4. **Generate business leads** for consulting, partnership, and enterprise use cases

The site takes direct inspiration from [opendataloader.org](https://opendataloader.org/) — a single-page hero-driven marketing site with docs sidebar, live demo, benchmarks, and contact form.

---

## 2. Project Goals

| # | Goal | Success Metric |
|---|------|---------------|
| G1 | Explain EdgeQuake's value in < 30 seconds | Hero section bounce rate < 40% |
| G2 | Drive GitHub stars and contributions | 50+ stars/month from website referral |
| G3 | Generate enterprise leads | 10+ contact form submissions/month |
| G4 | Rank on first page for target keywords | Top 10 for "graph rag framework rust" |
| G5 | Lighthouse score ≥ 95 on all axes | Performance, Accessibility, SEO, Best Practices |
| G6 | < 3 second Time to Interactive | Core Web Vitals all "Good" |

---

## 3. Target Audience

```
                    ┌─────────────────────────────────┐
                    │       TARGET AUDIENCES           │
                    └──────────┬──────────────────────┘
                               │
           ┌───────────────────┼───────────────────┐
           │                   │                   │
    ┌──────▼──────┐    ┌──────▼──────┐    ┌──────▼──────┐
    │  DEVELOPERS │    │  DECISION   │    │  COMMUNITY  │
    │             │    │  MAKERS     │    │             │
    │ • AI/ML eng │    │ • CTOs      │    │ • OSS cont- │
    │ • Backend   │    │ • VP Eng    │    │   ributors  │
    │ • Full-stack│    │ • Tech Lead │    │ • Bloggers  │
    │ • Data eng  │    │ • Architect │    │ • Educators │
    └─────────────┘    └─────────────┘    └─────────────┘
         │                   │                   │
    ┌────▼────┐        ┌────▼────┐        ┌────▼────┐
    │ WANT:   │        │ WANT:   │        │ WANT:   │
    │ Quick   │        │ ROI,    │        │ Docs,   │
    │ start,  │        │ bench-  │        │ contrib │
    │ API docs│        │ marks,  │        │ guide,  │
    │ examples│        │ support │        │ roadmap │
    └─────────┘        └─────────┘        └─────────┘
```

### Persona Definitions

| Persona | Name | Role | Pain Point | What They Need |
|---------|------|------|-----------|----------------|
| P1 | **Alex the AI Engineer** | ML/AI Engineer building RAG pipelines | Vector-only RAG misses relationships; Python too slow | Quick start, code examples, performance benchmarks |
| P2 | **Sarah the CTO** | Technical decision maker | Evaluating RAG solutions for production | Architecture overview, enterprise features, SLA/support |
| P3 | **Marcus the Contributor** | Open-source enthusiast | Wants to contribute to cutting-edge Rust projects | Contributing guide, architecture docs, community links |
| P4 | **Diana the Data Engineer** | Building knowledge management systems | Need production-grade ingestion pipelines | Pipeline docs, storage options, deployment guides |

---

## 4. Competitive Landscape

```
  Performance ▲
              │
         100x │                          ★ EdgeQuake
              │                            (Rust, Graph-RAG)
              │
          10x │
              │     ◆ LightRAG
              │       (Python, Graph-RAG)
           1x │───◆─Microsoft─GraphRAG──────────────────►
              │     (Python, Community-based)
              │
              │  ◆ LangChain RAG        ◆ LlamaIndex
              │    (Vector-only)          (Vector-only)
              │
              └──────────────────────────────────────────►
                Simple                          Advanced
                (Vector Only)            (Graph + Vector)
```

### Key Differentiators to Communicate

| Feature | EdgeQuake | Others |
|---------|-----------|--------|
| **Language** | Rust → 10x throughput | Python → GIL bottleneck |
| **Retrieval** | 6 query modes (hybrid) | 1-2 modes |
| **Graph DB** | Apache AGE on PostgreSQL | Neo4j / custom |
| **Multi-tenant** | Built-in workspace isolation | Manual configuration |
| **PDF Pipeline** | Text + Vision LLM | Text-only |
| **MCP Support** | 18 tools for AI agents | None |
| **Streaming** | SSE real-time | Batch only |
| **Cost** | Hybrid provider mode | Single provider |

---

## 5. Technology Stack

| Layer | Technology | Version | Rationale |
|-------|-----------|---------|-----------|
| **Framework** | Next.js | 16.x | Static export, App Router, SSG |
| **UI Library** | shadcn/ui | Latest | Radix primitives, Tailwind, accessible |
| **Primitives** | Radix UI | Latest | Headless, accessible, composable |
| **Styling** | Tailwind CSS | 4.x | Utility-first, tree-shakeable |
| **Deployment** | GitHub Pages | - | Free, global CDN, custom domain |
| **CI/CD** | GitHub Actions | - | Automated build + deploy on push |
| **Analytics** | Plausible / Umami | - | Privacy-first, GDPR compliant |
| **Forms** | Formspree / Resend | - | Serverless form handling |
| **Search** | Fumadocs / Pagefind | - | Static-site search |
| **Content** | MDX | - | Markdown + React components |
| **OpenGraph** | next/og + Satori | - | Dynamic OG image generation |
| **Icons** | Lucide React | - | Consistent icon set |
| **Animations** | Framer Motion | - | Smooth page transitions |
| **Code Blocks** | Shiki | - | Syntax highlighting |

---

## 6. Site URL & Domain Strategy

| Environment | URL | Purpose |
|------------|-----|---------|
| **Production** | `https://edgequake.dev` | Custom domain (preferred) |
| **Fallback** | `https://raphaelmansuy.github.io/edgequake` | GitHub Pages default |
| **Staging** | PR preview deploys | Review before merge |

---

## 7. Document Index

This specification is split into focused documents for maintainability:

```
specs/website/
├── 00-overview.md                  ← YOU ARE HERE
├── 01-information-architecture.md  ← Site map, navigation, page hierarchy
├── 02-content-strategy.md          ← WHY/WHAT/HOW messaging framework
├── 03-page-specifications.md       ← Per-page wireframes & content specs
├── 04-technical-architecture.md    ← Next.js setup, CI/CD, deployment
├── 05-seo-strategy.md              ← Meta tags, OpenGraph, JSON-LD, sitemap
├── 06-design-system.md             ← Colors, typography, spacing, dark mode
├── 07-component-library.md         ← shadcn/Radix components, custom widgets
├── 08-contact-lead-generation.md   ← Forms, CTAs, analytics, lead pipeline
└── 09-implementation-roadmap.md    ← Phases, milestones, acceptance criteria
```

---

## 8. Inspiration & Reference Sites

| Site | What to Learn |
|------|--------------|
| [opendataloader.org](https://opendataloader.org/) | Hero layout, problem→solution flow, benchmarks, contact form |
| [astro.build](https://astro.build/) | Dev tool marketing, community section, ecosystem showcase |
| [turso.tech](https://turso.tech/) | Technical product positioning, dark theme, benchmarks |
| [supabase.com](https://supabase.com/) | Feature grid, code examples, enterprise CTA |
| [neon.tech](https://neon.tech/) | PostgreSQL product marketing, developer-first design |

---

## 9. Non-Functional Requirements

| NFR | Requirement | Rationale |
|-----|------------|-----------|
| NFR-01 | **Static export only** — no server runtime | GitHub Pages hosting |
| NFR-02 | **Lighthouse ≥ 95** on all 4 axes | SEO + UX quality |
| NFR-03 | **< 200KB** initial JS bundle | Fast load on mobile |
| NFR-04 | **Dark/Light mode** with system preference detection | Developer preference |
| NFR-05 | **Mobile-first** responsive design | 40%+ mobile traffic expected |
| NFR-06 | **WCAG 2.1 AA** accessibility compliance | Inclusive design |
| NFR-07 | **Zero cookies** by default (privacy-first analytics) | GDPR compliance |
| NFR-08 | **Build time < 60s** for the full site | Fast iteration |
| NFR-09 | **Image optimization** via sharp / next/image custom loader | Performance |
| NFR-10 | **i18n-ready** structure (English first, French second) | Elitizon is France-based |

---

## 10. Out of Scope (v1)

- Blog engine (defer to GitHub Discussions / DEV.to)
- User authentication / dashboard
- Live playground / WASM demo
- Pricing page (all open-source for v1)
- Multi-language content (English only for v1; i18n-ready structure)
- A/B testing infrastructure
- Newsletter subscription management

---

*Next: [01-information-architecture.md](./01-information-architecture.md) — Site structure, navigation, and page hierarchy*
