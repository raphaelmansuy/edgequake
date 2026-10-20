# SPEC-170 — Perfect Astra Design

> **Astra** = the EdgeQuake **Astro** marketing website (the published site at
> [edgequake.com](https://edgequake.com)). Source lives in
> [`edgequake-website/`](../../edgequake-website). This spec redesigns the
> **public marketing homepage** (and the shared marketing shell) to be
> *exceptional, not clunky, and not AI slop* — grounded in the real codebase,
> real product screenshots from [demo.edgequake.com](https://demo.edgequake.com),
> and **measurable** layout health (CLS → 0).

---

## 0. Document map (cross-reference hub)

```
                        ┌─────────────────────────────┐
                        │  README.md  (you are here)   │
                        │  index · glossary · nav      │
                        └───────────────┬──────────────┘
                                        │
        ┌───────────────┬───────────────┼───────────────┬───────────────┐
        ▼               ▼               ▼               ▼               ▼
 ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────────┐
 │ 00 WHY &   │  │ 01 Current │  │ 02–07      │  │ 08 Design  │  │ 09 Impl.   │
 │ First      │─▶│ State      │─▶│ Six Lenses │─▶│ Spec       │─▶│ Plan       │
 │ Principles │  │ Assessment │  │ (PO/FS/DB/ │  │ (REQ-170)  │  │ (DRY/SOLID)│
 └────────────┘  └─────┬──────┘  │  UX/FE/MKT)│  └─────┬──────┘  └─────┬──────┘
                       │         └────────────┘        │               │
                       │ metrics                        │ requirements  │ tasks
                       ▼                                ▼               ▼
                 ┌────────────┐                  ┌────────────┐  ┌────────────┐
                 │ 11 Metrics │◀─────────────────│ 10 Edge    │◀─│ e2e/       │
                 │ & Accept.  │   acceptance     │ Cases &    │  │ harness +  │
                 │ (CLS gate) │                  │ Testing    │  │ screenshots│
                 └────────────┘                  └────────────┘  └────────────┘
```

| # | Document | Purpose | Primary lens |
|---|----------|---------|--------------|
| 00 | [00-why-first-principles.md](./00-why-first-principles.md) | Why redesign, from first principles | Strategy |
| 01 | [01-assessment-current-state.md](./01-assessment-current-state.md) | Grounded critique + baseline metrics | Analysis |
| 02 | [02-lens-product-owner.md](./02-lens-product-owner.md) | Outcomes, scope, success criteria | Product Owner |
| 03 | [03-lens-fullstack.md](./03-lens-fullstack.md) | Architecture, components, data flow | Full-Stack Dev |
| 04 | [04-lens-database-data-layer.md](./04-lens-database-data-layer.md) | Build-time data & content sourcing | Database/Data |
| 05 | [05-lens-ux-ui.md](./05-lens-ux-ui.md) | Information architecture, interaction, a11y | UX/UI Designer |
| 06 | [06-lens-front-design.md](./06-lens-front-design.md) | Visual system, type, motion, no-CLS craft | Front Designer |
| 07 | [07-lens-marketing-growth.md](./07-lens-marketing-growth.md) | Narrative, conversion, positioning | Marketing/Growth |
| 08 | [08-specification.md](./08-specification.md) | Normative requirements (REQ-170-xxx) | Spec |
| 09 | [09-implementation-plan.md](./09-implementation-plan.md) | Phased plan, DRY/SOLID, components | Engineering |
| 10 | [10-edge-cases-and-testing.md](./10-edge-cases-and-testing.md) | Edge cases envisioned, mitigated, tested | QA |
| 11 | [11-metrics-and-acceptance.md](./11-metrics-and-acceptance.md) | Measurable gates (CLS, overflow, a11y) | Verification |
| 12 | [12-design-plan-frontend-skill.md](./12-design-plan-frontend-skill.md) | Applies Anthropic **frontend-design** skill: typefaces, palette, de-tell | Front Designer |

---

## 1. Scope

**In scope** — the marketing surface rendered by
[`MarketingLayout.astro`](../../edgequake-website/src/layouts/MarketingLayout.astro):

- Homepage [`index.astro`](../../edgequake-website/src/pages/index.astro) and its
  eight sections in
  [`src/components/sections/`](../../edgequake-website/src/components/sections).
- Shared shell: [`Header.astro`](../../edgequake-website/src/components/Header.astro),
  [`Footer.astro`](../../edgequake-website/src/components/Footer.astro),
  design tokens in [`global.css`](../../edgequake-website/src/styles/global.css).
- The dead-code [`FadeIn.astro`](../../edgequake-website/src/components/FadeIn.astro)
  (a no-op today) → replaced by a real, reduced-motion-safe reveal primitive.

**Round 4 extension** — marketing secondary routes, Starlight docs chrome
(shared `SITE_NAV`, prose links), and agent entry points (`/llms.txt`,
`/llms-full.txt`) are in scope for polish; capture harness:
[`e2e/capture-docs.mjs`](./e2e/capture-docs.mjs).

**Still out of scope** — product WebUI (`edgequake_webui/`) and backend. Those
remain the *source of truth* for screenshots and claims.

## 2. Non-negotiable principles

1. **Grounded, not invented** — every claim, number, and screenshot traces to the
   repo or the live product. See [01](./01-assessment-current-state.md).
2. **Measured, not eyeballed** — "perfect" is defined by gates in
   [11](./11-metrics-and-acceptance.md): **CLS = 0**, zero unintended horizontal
   overflow, AA contrast, 44px tap targets.
3. **DRY + SOLID** — one design-token source, one reusable `SectionHeader`,
   one `Reveal` primitive, data-driven sections. See [09](./09-implementation-plan.md).
4. **No AI slop** — no filler copy, no fake logos/testimonials, no stock gradients
   for their own sake. Craft is justified by a principle. See [06](./06-lens-front-design.md).

## 3. Screenshot provenance

The product imagery is captured **live from demo.edgequake.com** via Playwright
([`e2e/capture-product.mjs`](./e2e/capture-product.mjs)). A member account was
provisioned through the demo invite/magic-link, then the five core product
screens were captured authenticated at 3200×2000 (2× DPR) into
[`e2e/screenshots/product/`](./e2e/screenshots/product):

| Screen | Route | Evidence of real data |
|--------|-------|-----------------------|
| `dashboard.png` | `/` | 15 documents · 13,200 entities · 12,023 relationships · 591 chunks |
| `documents.png` | `/documents` | 15-document table |
| `query.png` | `/query` | six-mode query console |
| `graph.png` | `/graph` | knowledge graph (9 canvas layers rendered) |
| `members.png` | `/members` | members & invites |

The demo uses single-use refresh-token rotation, so captures run in one fresh
login session (`DEMO_USER`/`DEMO_PASS`). See
[04](./04-lens-database-data-layer.md) §Screenshots.

## 4. Status

| Phase | State |
|-------|-------|
| Reconnaissance + baseline metrics | ✅ done ([01](./01-assessment-current-state.md)) |
| Spec + lenses + plan | ✅ this document set |
| Implementation | ✅ done ([09](./09-implementation-plan.md)) — foundation, primitives, hero, showcase, 7 sections refactored |
| Iterate to CLS=0 | ✅ worst CLS 0.0229 → 0.001; all gates green ([11](./11-metrics-and-acceptance.md)) |
