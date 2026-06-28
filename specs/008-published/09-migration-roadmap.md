# SPEC-008-09: Migration Roadmap

| Field      | Value                        |
| ---------- | ---------------------------- |
| **Parent** | [SPEC-008](./00-overview.md) |
| **Status** | Draft                        |
| **Date**   | 2026-03-21                   |

---

## 1. Overview

This document defines the phased migration from the current Next.js 16 website to the unified Astro + Starlight project. The migration is divided into four phases, each independently deployable and testable.

---

## 2. Migration Phases

```
Phase 1                Phase 2                Phase 3                Phase 4
Scaffold +             Port Static            Port Interactive       Cutover +
Starlight Docs         Marketing Pages        Components             Cleanup
                                              (React Islands)
[2-3 days]             [2-3 days]             [2-3 days]             [1 day]
     |                      |                      |                      |
     v                      v                      v                      v
Starlight docs         All 7 routes           Canvas animation       DNS points
live at /docs/         rendered in Astro      + Framer Motion        to new site
                                              ported as islands
```

---

## 3. Phase 1 — Scaffold + Starlight Docs

**Goal:** New Astro project with Starlight serving all 52 docs pages.

### Tasks

- [ ] 1.1 Initialize Astro project in `edgequake-website/` (replace Next.js files)
- [ ] 1.2 Install Starlight, React, Tailwind, Sitemap integrations
- [ ] 1.3 Create `content.config.ts` with docs collection
- [ ] 1.4 Symlink `src/content/docs` → `../../../docs`
- [ ] 1.5 Configure sidebar in `astro.config.mjs` (12 groups, autogenerate)
- [ ] 1.6 Inject missing frontmatter (`title`, `description`) into `docs/` files
- [ ] 1.7 Create placeholder `src/pages/index.astro` ("Coming soon" or redirect)
- [ ] 1.8 Verify all 52 docs pages render at `/docs/`
- [ ] 1.9 Verify Pagefind search works across docs
- [ ] 1.10 Set up CI pipeline (build + deploy to preview)

### Exit Criteria

- `astro build` succeeds with zero errors
- All 52 docs pages accessible via browser
- Pagefind search returns results
- CI deploys preview successfully

### Risks

| Risk                           | Mitigation                                                                                        |
| ------------------------------ | ------------------------------------------------------------------------------------------------- |
| Docs files missing frontmatter | Run injection script (see [02-single-source-strategy.md](./02-single-source-strategy.md))         |
| Symlink doesn't work in CI     | Fallback to rsync copy in CI (see [02-single-source-strategy.md](./02-single-source-strategy.md)) |

---

## 4. Phase 2 — Port Static Marketing Pages

**Goal:** All 7 Next.js routes rendered as Astro pages with the same visual design.

### Component Migration Table

| Next.js Component         | Lines | Astro Target                 | Migration Strategy                             |
| ------------------------- | ----- | ---------------------------- | ---------------------------------------------- |
| `app/page.tsx` (Home)     | 20    | `src/pages/index.astro`      | Rewrite as Astro template                      |
| `app/demo/page.tsx`       | 170   | `src/pages/demo.astro`       | Rewrite, keep DemoModeSelector as React island |
| `app/ecosystem/page.tsx`  | 105   | `src/pages/ecosystem.astro`  | Rewrite as Astro template                      |
| `app/enterprise/page.tsx` | 110   | `src/pages/enterprise.astro` | Rewrite as Astro template                      |
| `app/contact/page.tsx`    | 140   | `src/pages/contact.astro`    | Rewrite, keep ContactForm as React island      |
| `app/docs/page.tsx`       | 65    | Starlight handles `/docs/`   | Remove; Starlight serves docs                  |
| `app/not-found.tsx`       | 25    | `src/pages/404.astro`        | Rewrite as Astro template                      |

### Section Component Migration

| Component              | Lines | Strategy                   |
| ---------------------- | ----- | -------------------------- |
| `HeroSection`          | 50    | Convert to Astro component |
| `ProblemSection`       | 60    | Convert to Astro component |
| `SolutionSection`      | 80    | Convert to Astro component |
| `ArchitectureSection`  | 140   | Convert to Astro component |
| `BenchmarksSection`    | 95    | Convert to Astro component |
| `QuickStartSection`    | 125   | Convert to Astro component |
| `EcosystemSection`     | 65    | Convert to Astro component |
| `EnterpriseCTASection` | 65    | Convert to Astro component |
| `Header`               | 92    | Convert to Astro component |
| `Footer`               | 90    | Convert to Astro component |

### Tasks

- [ ] 2.1 Create `MarketingLayout.astro` (HTML head, header, footer, global styles)
- [ ] 2.2 Port `globals.css` to Tailwind 4 config or global stylesheet
- [ ] 2.3 Convert all 8 section components from React TSX to Astro
- [ ] 2.4 Create all 6 marketing pages (home, demo, ecosystem, enterprise, contact, 404)
- [ ] 2.5 Port Header and Footer as Astro components
- [ ] 2.6 Add `data-pagefind-body` to marketing page content areas
- [ ] 2.7 Verify visual parity with current Next.js site (screenshot comparison)
- [ ] 2.8 Configure `disable404Route: true` in Starlight, add custom 404 page

### Exit Criteria

- All 7 routes render with identical visual appearance
- No React runtime shipped for static pages (only Astro HTML)
- Lighthouse Performance ≥ 95, SEO = 100

---

## 5. Phase 3 — Port Interactive Components (React Islands)

**Goal:** Bring across the interactive elements as React islands with `client:` directives.

### Island Migration Table

| Component          | Lines | Interaction                                | Client Directive | Priority |
| ------------------ | ----- | ------------------------------------------ | ---------------- | -------- |
| `GraphAnimation`   | 145   | Canvas 2D animation, requestAnimationFrame | `client:visible` | High     |
| `DemoModeSelector` | ~40   | Tab switching between demo modes           | `client:load`    | High     |
| `ThemeToggle`      | 22    | Dark/light mode switch                     | `client:load`    | Medium   |
| `ContactForm`      | ~50   | Form inputs + validation                   | `client:visible` | Medium   |

### Framer Motion Assessment

The current site uses `framer-motion` for:

- Fade-in on scroll (`FadeIn` component, 42 lines)
- Stagger children animations

**Options:**

1. **Keep framer-motion** — Use as React islands with `client:visible`, adds ~60KB
2. **Replace with CSS** — Convert to CSS `@keyframes` + Intersection Observer, zero JS
3. **Replace with Astro View Transitions** — Native Astro animation API

**Recommendation:** Option 2 (CSS animations) for simple fade/stagger. Reserve React islands for truly interactive components (Canvas, forms).

### Tasks

- [ ] 3.1 Copy `GraphAnimation.tsx` to `src/components/islands/`
- [ ] 3.2 Add `client:visible` directive in Astro template
- [ ] 3.3 Port `DemoModeSelector` as React island with `client:load`
- [ ] 3.4 Port `ThemeToggle` as React island with `client:load`
- [ ] 3.5 Port `ContactForm` as React island with `client:visible`
- [ ] 3.6 Replace Framer Motion fade-in with CSS animations and Intersection Observer
- [ ] 3.7 Remove `framer-motion` from dependencies
- [ ] 3.8 Verify all interactive features work correctly
- [ ] 3.9 Performance audit: JS bundle < 100 KB total

### Exit Criteria

- All interactive features functional
- No full-page React hydration (islands only)
- JS bundle significantly smaller than Next.js bundle
- Zero functionality regression

---

## 6. Phase 4 — Cutover & Cleanup

**Goal:** DNS points to new site. Old Next.js code removed.

### Tasks

- [ ] 4.1 Final visual and functional comparison (old vs new)
- [ ] 4.2 Verify all URLs match old site (no broken bookmarks)
- [ ] 4.3 Set up redirect rules for any changed paths (see [08-deployment-strategy.md](./08-deployment-strategy.md))
- [ ] 4.4 Deploy to production hosting
- [ ] 4.5 Update DNS CNAME to point to new hosting provider
- [ ] 4.6 Verify HTTPS works on custom domain
- [ ] 4.7 Submit updated sitemap to Google Search Console
- [ ] 4.8 Remove old Next.js files (backup in a branch first)
- [ ] 4.9 Update Makefile commands for new website stack
- [ ] 4.10 Update AGENTS.md and project documentation
- [ ] 4.11 Monitor for 404 errors in first 48 hours

### Exit Criteria

- `edgequake.com` serves the Astro site
- Zero 404 errors for previously indexed URLs
- Google Search Console shows clean indexing

---

## 7. Rollback Plan

At any phase, rollback is trivial:

| Phase              | Rollback                                                     |
| ------------------ | ------------------------------------------------------------ |
| Phase 1-3          | Do nothing — old Next.js site still deployed                 |
| Phase 4 (post-DNS) | Revert DNS CNAME to old hosting; takes effect in < 5 minutes |

---

## 8. Timeline Estimate

| Phase     | Effort        | Can Run In Parallel |
| --------- | ------------- | ------------------- |
| Phase 1   | 2-3 days      | Independent         |
| Phase 2   | 2-3 days      | After Phase 1       |
| Phase 3   | 2-3 days      | After Phase 2       |
| Phase 4   | 1 day         | After Phase 3       |
| **Total** | **7-10 days** |                     |

---

## 9. Dependency Removal Summary

After migration, the following dependencies are removed:

| Package                      | Reason                                          |
| ---------------------------- | ----------------------------------------------- |
| `next` (16.x)                | Replaced by Astro                               |
| `react-dom` (full hydration) | Only partial hydration via islands              |
| `framer-motion`              | Replaced by CSS animations                      |
| `next-themes`                | Starlight has built-in theme support            |
| `shiki`                      | Starlight uses Expressive Code (includes Shiki) |
| `@next/third-parties`        | No Next.js-specific analytics needed            |

Dependencies retained for React islands:

- `react` (19.x)
- `lucide-react`
- `clsx`, `tailwind-merge`

---

## 10. Cross-References

- [00-overview.md](./00-overview.md) — Goal G7 (migrate marketing pages), G8 (single deployment)
- [01-architecture-decision-record.md](./01-architecture-decision-record.md) — Why Astro over keeping Next.js
- [04-starlight-project-setup.md](./04-starlight-project-setup.md) — Project structure for Phase 1
- [05-build-pipeline.md](./05-build-pipeline.md) — CI/CD setup in Phase 1
- [08-deployment-strategy.md](./08-deployment-strategy.md) — Hosting and DNS cutover in Phase 4
- [10-content-mapping-matrix.md](./10-content-mapping-matrix.md) — URL mapping verification for Phase 4
