# SPEC-008: Unified EdgeQuake Website — Marketing + Published Documentation

| Field       | Value                             |
| ----------- | --------------------------------- |
| **Spec ID** | SPEC-008                          |
| **Title**   | Unified Astro + Starlight Website |
| **Status**  | Draft                             |
| **Author**  | EdgeQuake Team                    |
| **Created** | 2026-03-21                        |
| **Updated** | 2026-03-21                        |

---

## 1. Executive Summary

EdgeQuake currently has two separate web properties:

1. **`edgequake-website/`** — A Next.js 16 marketing site (~4000 lines of TSX) at `edgequake.com`
2. **`docs/`** — 52 markdown documentation files accessible only via GitHub

This specification defines the strategy to **replace the Next.js marketing site and publish the documentation as a single, unified Astro + Starlight project**, deployed to `edgequake.com`. Marketing pages (home, demo, ecosystem, enterprise, contact) become custom Astro pages. Documentation lives under `/docs/` powered by Starlight.

**Key principles:**

- `docs/` remains the single source of truth — zero content duplication.
- One project, one build, one deployment — no two-framework complexity.
- Marketing pages use Astro's island architecture for interactivity (graph animation, demo mode selector).

---

## 2. Goals

| #   | Goal                                                | Measure                                                                                     |
| --- | --------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| G1  | Publish all `docs/` content as a searchable website | 100% of docs/ files rendered at edgequake.com/docs/                                         |
| G2  | Zero content duplication                            | No markdown files copied; symlink or build-time reference only                              |
| G3  | Built-in full-text search                           | Pagefind indexes docs pages; search returns results in < 200ms                              |
| G4  | SEO-optimized                                       | Sitemap generated, meta tags present, Lighthouse SEO ≥ 95                                   |
| G5  | Branded & consistent                                | Custom logo, colors, fonts matching EdgeQuake brand                                         |
| G6  | Developer-friendly authoring                        | Edit `docs/`, see changes on dev server instantly                                           |
| G7  | Migrate all Next.js pages to Astro                  | 7 routes ported with visual parity: /, /contact, /demo, /docs, /ecosystem, /enterprise, 404 |
| G8  | Single deployment point                             | One build artifact, one CI/CD pipeline, one domain                                          |

---

## 3. Non-Goals

- Internationalization (i18n) — English-only for now
- User-generated content or comments
- API auto-generation from Rust code (future enhancement)
- Backend contact form processing (remains client-side)

---

## 4. Scope

```
+---------------------------------------------------------------+
|                  edgequake-website/                            |
|                  (Unified Astro + Starlight)                   |
|                  edgequake.com                                 |
|                                                               |
|   Custom Astro Pages          Starlight Integration           |
|   ┌─────────────────┐        ┌─────────────────────┐         |
|   │ /               │        │ /docs/              │         |
|   │ /demo/          │        │ /docs/getting-started│         |
|   │ /ecosystem/     │        │ /docs/concepts/     │         |
|   │ /enterprise/    │        │ /docs/tutorials/    │         |
|   │ /contact/       │        │ /docs/api-reference/│         |
|   │ 404             │        │ /docs/deep-dives/   │         |
|   └─────────────────┘        │ ... (52 pages)      │         |
|         │                    └────────┬────────────┘         |
|         │ React Islands               │ symlink              |
|         │ (graph animation,           │                      |
|         │  demo interactivity)        ▼                      |
|         │                    ┌─────────────────────┐         |
|         │                    │ docs/               │         |
|         │                    │ (Single Source of   │         |
|         │                    │  Truth, 52 files)   │         |
|         ▼                    └─────────────────────┘         |
|   Astro Island Architecture                                   |
|   (ship zero JS except where needed)                          |
+---------------------------------------------------------------+
```

---

## 5. Document Index

This spec is split into 14 focused documents, each cross-referenced:

| Document                                                                   | Title                           | Purpose                                                  |
| -------------------------------------------------------------------------- | ------------------------------- | -------------------------------------------------------- |
| [00-overview.md](./00-overview.md)                                         | Overview (this document)        | Goals, scope, document index                             |
| [01-architecture-decision-record.md](./01-architecture-decision-record.md) | Architecture Decision Record    | Why Astro + Starlight, migration rationale               |
| [02-single-source-strategy.md](./02-single-source-strategy.md)             | Single Source of Truth Strategy | Symlink approach, no duplication                         |
| [03-information-architecture.md](./03-information-architecture.md)         | Information Architecture        | Unified sitemap, sidebar, navigation map                 |
| [04-starlight-project-setup.md](./04-starlight-project-setup.md)           | Project Setup                   | Astro config, Starlight integration, directory structure |
| [05-build-pipeline.md](./05-build-pipeline.md)                             | Build Pipeline                  | Frontmatter injection, CI/CD, build scripts              |
| [06-search-navigation-seo.md](./06-search-navigation-seo.md)               | Search, Navigation & SEO        | Pagefind, sitemap, meta tags                             |
| [07-content-authoring-standards.md](./07-content-authoring-standards.md)   | Content Authoring Standards     | Frontmatter schema, markdown conventions                 |
| [08-deployment-strategy.md](./08-deployment-strategy.md)                   | Deployment Strategy             | Hosting, domain, CDN, CI/CD workflows                    |
| [09-migration-roadmap.md](./09-migration-roadmap.md)                       | Migration Roadmap               | Next.js → Astro phased implementation                    |
| [10-content-mapping-matrix.md](./10-content-mapping-matrix.md)             | Content Mapping Matrix          | Next.js pages + docs/ → unified site mapping             |
| [11-design-system.md](./11-design-system.md)                               | Design System & Tokens          | Colors, typography, spacing, motion, dark/light mode     |
| [12-page-layouts-wireframes.md](./12-page-layouts-wireframes.md)           | Page Layouts & Wireframes       | ASCII wireframes, responsive behavior, section rhythm    |
| [13-component-library.md](./13-component-library.md)                       | Component Library               | Primitives, marketing sections, React islands, overrides |

---

## 6. Key Decisions Summary

| Decision          | Choice                                 | Rationale                                                         |
| ----------------- | -------------------------------------- | ----------------------------------------------------------------- |
| Site framework    | **Astro + Starlight**                  | Unified marketing + docs, island architecture, zero JS by default |
| Migration scope   | **Replace Next.js entirely**           | Single deployment, no two-framework maintenance                   |
| Content sourcing  | **Symlinks** from Starlight to `docs/` | Zero duplication, instant sync, git-friendly                      |
| Deployment target | **`edgequake.com`** (same domain)      | Single deployment point, unified SEO                              |
| Interactivity     | **Astro Islands** (React)              | Graph animation and demo page keep React; rest is static          |
| Search engine     | **Pagefind** (built-in)                | Zero-config, client-side, no external service                     |
| Hosting           | **Vercel / Cloudflare Pages**          | Static site, edge CDN, automatic deploys                          |

See [01-architecture-decision-record.md](./01-architecture-decision-record.md) for full rationale.

---

## 7. Current State Analysis

### 7.1 Existing `docs/` Structure

```
docs/
├── getting-started/        # 2 files: installation, quick-start
├── concepts/               # 4 files: entity-extraction, graph-rag, hybrid-retrieval, knowledge-graph
├── architecture/           # 4 files: overview, data-flow, lineage-tracking, crates/
├── deep-dives/             # 13 files: chunking, embeddings, entity-normalization, etc.
├── api-reference/          # 4 files: rest-api, extended-api, lineage-endpoints, document-upload
├── tutorials/              # 7 files: document-ingestion, first-rag-app, migration, pdf, etc.
├── operations/             # 5 files: configuration, deployment, monitoring, performance, metadata
├── security/               # 1 file: best-practices
├── integrations/           # 3 files: custom-clients, langchain, open-webui
├── comparisons/            # 4 files: vs-graphrag, vs-lightrag, vs-traditional-rag, superiority
├── troubleshooting/        # 1 file: common-issues
├── fixes/                  # 1 file: embedding-api-validation
├── cookbook.md              # Standalone
├── faq.md                  # Standalone
└── features.md             # Standalone
Total: ~52 markdown files
```

### 7.2 Existing `edgequake-website/` (Next.js 16)

| Property       | Detail                                                     |
| -------------- | ---------------------------------------------------------- |
| Framework      | Next.js 16 + React 19                                      |
| Output         | Static export (SSG)                                        |
| Routes         | 7: /, /contact, /demo, /docs, /ecosystem, /enterprise, 404 |
| Lines of code  | ~4000 TSX + 135 CSS                                        |
| Components     | 6 layout/visual + 8 section + 17 shadcn UI                 |
| Key animations | Canvas graph (145 lines), Framer Motion fade/stagger       |
| Domain         | edgequake.com (CNAME)                                      |
| Dark mode      | next-themes, dark by default                               |
| Dependencies   | 12 production, 6 dev                                       |

This site will be **fully replaced** by the unified Astro project.

---

## 8. Success Criteria

| Criteria                       | Target                                            |
| ------------------------------ | ------------------------------------------------- |
| All `docs/` files published    | 52/52 files rendered at /docs/                    |
| All marketing pages ported     | 7/7 routes with visual parity                     |
| Search works                   | Full-text search returns relevant results         |
| Lighthouse Performance         | ≥ 90                                              |
| Lighthouse SEO                 | ≥ 95                                              |
| Build time                     | < 60 seconds (marketing + docs)                   |
| Zero broken links              | All internal cross-references resolve             |
| Mobile responsive              | Readable on 375px viewport                        |
| Single deployment              | One build, one domain, one CI pipeline            |
| Interactive features preserved | Graph animation, demo mode selector, theme toggle |

---

## 9. Related Documents

- [specs/website/00-overview.md](../website/00-overview.md) — Original marketing website spec (historical)
- [specs/website/04-technical-architecture.md](../website/04-technical-architecture.md) — Original tech arch (historical)
- [docs/README.md](../../docs/README.md) — Documentation index
