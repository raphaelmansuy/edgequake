# SPEC-008-01: Architecture Decision Record — Unified Astro + Starlight

| Field        | Value                                             |
| ------------ | ------------------------------------------------- |
| **Parent**   | [SPEC-008](./00-overview.md)                      |
| **Status**   | Accepted                                          |
| **Decision** | Migrate Next.js site to unified Astro + Starlight |
| **Date**     | 2026-03-21                                        |

---

## 1. Context

EdgeQuake has two web concerns:

1. **Marketing site** — `edgequake-website/` (Next.js 16, ~4000 lines) serving 7 routes at `edgequake.com`
2. **Documentation** — `docs/` (52 markdown files) accessible only via GitHub

We need to:

- Publish documentation as a searchable, branded website
- Reduce deployment complexity (one project, one build, one domain)
- Eliminate the two-framework maintenance burden (Next.js + a separate docs tool)

The question became: should we add a **separate Starlight project** alongside Next.js, or **replace Next.js entirely** with an Astro project that integrates Starlight?

---

## 2. Options Evaluated

### Option A: Unified Astro + Starlight — ✅ Selected

Replace the entire Next.js marketing site with an Astro project. Marketing pages become custom Astro pages. Docs use the Starlight integration under `/docs/`.

```
+-----------------------------------------------+
|  Unified Astro + Starlight                    |
|                                               |
|  Marketing (Astro pages)   Docs (Starlight)   |
|  +-----------------------+  +---------------+ |
|  | /         (home)      |  | /docs/        | |
|  | /demo/                |  | Pagefind      | |
|  | /ecosystem/           |  | Sidebar       | |
|  | /enterprise/          |  | TOC           | |
|  | /contact/             |  | 52 pages      | |
|  +-----------------------+  +---------------+ |
|                                               |
|  * Single deployment at edgequake.com         |
|  * One build pipeline                         |
|  * Shared header/footer across all pages      |
|  * Pagefind search covers docs                |
|  * Zero JS by default, islands for activity   |
|  * No React runtime shipped to static pages   |
+-----------------------------------------------+
```

| Criterion         | Score                               |
| ----------------- | ----------------------------------- |
| Single deployment | ★★★★★ One project, one domain       |
| Docs search       | ★★★★★ Pagefind built-in             |
| Bundle size       | ★★★★★ Zero JS for static pages      |
| Build speed       | ★★★★☆ < 60s for marketing + 52 docs |
| Migration effort  | ★★★☆☆ Port 7 pages + animations     |
| Maintenance       | ★★★★★ One framework to maintain     |

### Option B: Separate Starlight Site + Keep Next.js

Keep the Next.js marketing site. Add a second Starlight project at `docs.edgequake.com`.

| Criterion         | Score                                   |
| ----------------- | --------------------------------------- |
| Single deployment | ★☆☆☆☆ Two deploys, two domains          |
| Docs search       | ★★★★★ Pagefind built-in                 |
| Bundle size       | ★★★☆☆ Next.js still ships React runtime |
| Build speed       | ★★★★★ Each project builds independently |
| Migration effort  | ★★★★★ No marketing page changes         |
| Maintenance       | ★★☆☆☆ Two frameworks, two CI pipelines  |

**Declined because:** Maintains two-framework complexity, two deploy pipelines, fragmented SEO, inconsistent UX between marketing and docs.

### Option C: Docusaurus (Separate or Integrated)

| Criterion         | Score                                        |
| ----------------- | -------------------------------------------- |
| Single deployment | ★★★☆☆ Possible but awkward with custom pages |
| Docs search       | ★★★★☆ Algolia or local plugin                |
| Bundle size       | ★★☆☆☆ Ships React runtime                    |
| Build speed       | ★★★☆☆ 30-60s for 50 pages                    |
| Migration effort  | ★★★☆☆ React-based, some component reuse      |
| Maintenance       | ★★★☆☆ Plugin ecosystem adds complexity       |

**Declined because:** Ships React runtime, heavier than Astro, Algolia dependency for search.

### Option D: Keep Next.js + Add MDX Documentation

Build docs rendering into the existing Next.js site using MDX pages.

| Criterion         | Score                                                             |
| ----------------- | ----------------------------------------------------------------- |
| Single deployment | ★★★★★ Same project                                                |
| Docs search       | ★★☆☆☆ Must build from scratch                                     |
| Bundle size       | ★★☆☆☆ React + Next.js runtime on every page                       |
| Build speed       | ★★☆☆☆ Next.js overhead significant                                |
| Migration effort  | ★★★★★ No migration needed                                         |
| Maintenance       | ★☆☆☆☆ Must build sidebar, TOC, search, code highlighting manually |

**Declined because:** Enormous build-vs-buy gap. Would require building sidebar, search, table of contents, code highlighting — all features Starlight provides out of the box.

### Option E: VitePress

| Criterion         | Score                                      |
| ----------------- | ------------------------------------------ |
| Single deployment | ★★★☆☆ Possible with custom theme           |
| Docs search       | ★★★★☆ MiniSearch built-in                  |
| Bundle size       | ★★★☆☆ Vue runtime shipped                  |
| Build speed       | ★★★★☆ ~15s                                 |
| Migration effort  | ★★☆☆☆ Must rewrite React components in Vue |
| Maintenance       | ★★★★☆ Well-maintained                      |

**Declined because:** Vue runtime, must rewrite React-based interactive components (graph animation, demo), smaller ecosystem.

---

## 3. Decision

**Migrate the entire `edgequake-website/` to a unified Astro project with Starlight** deployed at `edgequake.com`.

### Rationale

1. **Single deployment point**: One project, one build, one domain — eliminates multi-site orchestration
2. **Purpose-built docs**: Starlight provides sidebar, search, TOC, code highlighting, dark mode out of the box
3. **Zero JS by default**: Static marketing pages ship no JavaScript; only interactive islands load client-side code
4. **Pagefind search**: Client-side full-text search across all docs, no external service
5. **Markdown-native**: Existing `docs/*.md` files work as-is with minimal frontmatter additions
6. **Island architecture**: Astro's `client:*` directives let us keep React components (graph animation, framer-motion) only where needed
7. **Unified SEO**: Single domain, single sitemap, coherent link graph for search engines
8. **Active ecosystem**: Astro has 50k+ GitHub stars, Starlight 10k+, maintained by the Astro core team

### Why Not Keep Next.js?

The current Next.js site uses `output: "export"` (static HTML). It ships the React runtime to every page even though:

- 5 of 7 routes have **zero client-side interactivity**
- The only truly interactive elements are the canvas graph animation and the demo mode selector
- The contact form could be a simple HTML form

Astro's island architecture is a better fit: static HTML by default, React only where interactivity demands it.

---

## 4. Consequences

### Positive

- **One deployment** instead of two separate sites
- **Docs become searchable and branded** at edgequake.com/docs/
- **Smaller bundles**: Static pages ship zero JS
- **Unified navigation**: Shared header links between marketing and docs
- **Unified SEO**: Single domain, single sitemap
- **Reduced maintenance**: One framework, one build system, one CI pipeline
- **Zero content duplication**: Symlink strategy from `docs/` (see [02-single-source-strategy.md](./02-single-source-strategy.md))

### Negative

- **Migration effort**: 7 Next.js pages must be ported to Astro (~4000 lines of TSX)
- **Animation porting**: Canvas graph animation and Framer Motion effects need Astro island wrappers
- **Component library change**: shadcn/ui components need Astro-compatible alternatives or island wrappers
- **Learning curve**: Team must learn Astro basics (templates, islands, content collections)

### Risk Mitigations

| Risk                          | Mitigation                                                                                                        |
| ----------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Migration takes too long      | Phased approach — docs first, then pages one at a time (see [09-migration-roadmap.md](./09-migration-roadmap.md)) |
| React component compatibility | Astro natively supports React components via `@astrojs/react` integration                                         |
| Animation fidelity loss       | Canvas graph runs in React island; Framer Motion stays in React; CSS animations replace simple fade-ins           |
| shadcn/ui components          | Most are wrappers around Radix — use bare HTML/CSS for static parts, React islands for interactive ones           |
| Team learning curve           | Astro templates are HTML-like — minimal ramp-up for React devs                                                    |

---

## 5. Cross-References

- [00-overview.md](./00-overview.md) — Goals and scope of unified approach
- [02-single-source-strategy.md](./02-single-source-strategy.md) — How symlinks avoid docs duplication
- [04-starlight-project-setup.md](./04-starlight-project-setup.md) — Concrete Astro + Starlight configuration
- [08-deployment-strategy.md](./08-deployment-strategy.md) — Hosting and CI/CD
- [09-migration-roadmap.md](./09-migration-roadmap.md) — Phased Next.js → Astro migration plan
