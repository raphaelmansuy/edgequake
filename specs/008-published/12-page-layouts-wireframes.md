# SPEC-008-12: Page Layouts & Wireframes

| Field       | Value                                 |
| ----------- | ------------------------------------- |
| **Spec ID** | SPEC-008-12                           |
| **Parent**  | [SPEC-008 Overview](./00-overview.md) |
| **Title**   | Page Layouts & Wireframes             |
| **Status**  | Draft                                 |
| **Created** | 2026-03-21                            |
| **Updated** | 2026-03-21                            |

---

## 1. Purpose

Define the page-level layout structures, wireframes, and responsive behavior for every page type on the unified EdgeQuake Astro + Starlight site. Each wireframe uses ASCII diagrams to communicate spatial relationships, content hierarchy, and responsive breakpoints.

**Cross-references:**

- [00-overview.md](./00-overview.md) — Route list and scope
- [03-information-architecture.md](./03-information-architecture.md) — Sitemap, navigation taxonomy
- [04-starlight-project-setup.md](./04-starlight-project-setup.md) — Astro directory structure, layouts
- [11-design-system.md](./11-design-system.md) — Tokens, spacing, grid system (§5, §6)
- [13-component-library.md](./13-component-library.md) — Component specifications used in layouts

---

## 2. Page Type Taxonomy

The unified site has two distinct layout families:

```
    ┌───────────────────────────────────────────────┐
    │              Page Type Taxonomy                 │
    │                                                │
    │   ┌─────────────────┐  ┌────────────────────┐ │
    │   │  MARKETING       │  │  DOCUMENTATION      │ │
    │   │  (Custom Astro)  │  │  (Starlight)        │ │
    │   │                  │  │                     │ │
    │   │  • Home (/)      │  │  • Docs Index       │ │
    │   │  • Demo          │  │  • Getting Started  │ │
    │   │  • Ecosystem     │  │  • Concept Pages    │ │
    │   │  • Enterprise    │  │  • API Reference    │ │
    │   │  • Contact       │  │  • Tutorials        │ │
    │   │  • 404           │  │  • Deep Dives       │ │
    │   │                  │  │  • ... (52 pages)   │ │
    │   └─────────────────┘  └────────────────────┘ │
    │          │                       │             │
    │          ▼                       ▼             │
    │   MarketingLayout.astro   Starlight Layout     │
    │   (shared header/footer)  (sidebar, TOC, etc) │
    └───────────────────────────────────────────────┘
```

| Layout Family | Template           | Header    | Footer    | Sidebar | TOC |
| ------------- | ------------------ | --------- | --------- | ------- | --- |
| Marketing     | `MarketingLayout`  | Unified   | Full      | No      | No  |
| Documentation | Starlight built-in | Unified\* | Starlight | Yes     | Yes |
| Splash        | Starlight `splash` | Unified\* | Starlight | No      | No  |

> \*Unified header is achieved via Starlight component override for `Header`. See [13-component-library.md §5](./13-component-library.md).

---

## 3. Shared Header

The header is consistent across marketing and docs pages, providing brand continuity — a pattern used by Cloudflare Docs, Netlify Docs, and astro.build.

### 3.1 Desktop Header (≥ 768px)

```
    ┌──────────────────────────────────────────────────────────────┐
    │  ⚡ EdgeQuake          Docs  Demo  Ecosystem  Enterprise     │
    │                                                 🔍 ☀ 🐙 [Get│
    │                                                      Started]│
    └──────────────────────────────────────────────────────────────┘
    │← Logo+Name  │← Nav Links (text-sm, muted-fg)  │← Actions  →│
    │  gap-2       │  gap-1, hover:foreground         │  toggles   │
    │              │                                   │  + CTA btn │

    Height: 4rem (64px)
    Position: sticky top-0 z-50
    Background: bg-background/80 backdrop-blur-xl
    Border: border-b border-border/50
    Max-width: 80rem centered
```

### 3.2 Mobile Header (< 768px)

```
    ┌────────────────────────────────────┐
    │  ⚡ EdgeQuake              ☀  ☰   │
    └────────────────────────────────────┘
                    │
            (on ☰ tap)
                    ▼
    ┌────────────────────────────────────┐
    │  Docs                              │
    │  Demo                              │
    │  Ecosystem                         │
    │  Enterprise                        │
    │  ─────────────────────────         │
    │  [🐙 GitHub]  [Get Started]        │
    └────────────────────────────────────┘

    Menu: full-width slide-down (max-height transition 200ms)
    Items: text-sm, py-2, hover:bg-muted, rounded-md
    Dismiss: tap ☰ again, tap link, or tap outside
```

### 3.3 Header Behavior

| Behavior                | Implementation                                      |
| ----------------------- | --------------------------------------------------- |
| Sticky positioning      | `position: sticky; top: 0;`                         |
| Blur backdrop           | `backdrop-filter: blur(24px)`, semi-transparent bg  |
| Active page indicator   | Current nav link gets `text-foreground` + underline |
| Docs search integration | `🔍` opens Pagefind modal (Cmd+K)                   |
| Theme persistence       | Syncs with Starlight's `data-theme` attribute       |

> **Pattern source:** astro.build uses the same blur + transparent header pattern. Cloudflare Docs uses a sticky header with integrated search trigger. Both normalize zero layout shift.

---

## 4. Home Page Layout (`/`)

The landing page uses a full-width marketing layout with sectioned content blocks.

### 4.1 Desktop Wireframe (≥ 1024px)

```
    ┌──────────────────────────────────────────────────────────────┐
    │                        HEADER                                │
    ├──────────────────────────────────────────────────────────────┤
    │                                                              │
    │  HERO SECTION (py-24 → py-32)                                │
    │  ┌──────────────────────┐  ┌──────────────────────────┐     │
    │  │  Graph-RAG.           │  │                          │     │
    │  │  Built for Speed.     │  │   ┌─ ─ ─ ─ ─ ─ ─ ┐     │     │
    │  │                       │  │   │  Graph Canvas  │     │     │
    │  │  Turn documents into  │  │   │  Animation     │     │     │
    │  │  knowledge graphs...  │  │   │  (React Island)│     │     │
    │  │                       │  │   └─ ─ ─ ─ ─ ─ ─ ┘     │     │
    │  │  [Get Started ─▸]     │  │                          │     │
    │  │  [🐙 GitHub    ]      │  │     480×480 canvas       │     │
    │  │                       │  │                          │     │
    │  │  ┌Apache─┐ ┌1k+/─┐   │  │                          │     │
    │  │  │2.0    │ │min  │    │  └──────────────────────────┘     │
    │  │  └───────┘ └─────┘   │                                   │
    │  └──────────────────────┘                                    │
    │             col-span-6        col-span-6                     │
    │                                                              │
    ├──────────────────────────────────────────────────────────────┤
    │                                                              │
    │  PROBLEM SECTION (py-16 → py-24)                             │
    │  ┌──────────────────────────────────────────────────────┐   │
    │  │  H2: Why Classic RAG Falls Short                      │   │
    │  │                                                       │   │
    │  │  ┌──────────┐  ┌──────────┐  ┌──────────┐           │   │
    │  │  │ Icon     │  │ Icon     │  │ Icon     │           │   │
    │  │  │ Problem 1│  │ Problem 2│  │ Problem 3│           │   │
    │  │  │ text...  │  │ text...  │  │ text...  │           │   │
    │  │  └──────────┘  └──────────┘  └──────────┘           │   │
    │  │       col-span-4 × 3 cards                           │   │
    │  └──────────────────────────────────────────────────────┘   │
    │                                                              │
    ├──────────────────────────────────────────────────────────────┤
    │                                                              │
    │  SOLUTION SECTION          (same grid pattern)               │
    │                                                              │
    ├──────────────────────────────────────────────────────────────┤
    │                                                              │
    │  ARCHITECTURE SECTION                                        │
    │  ┌──────────────────────────────────────────────────────┐   │
    │  │  ASCII or SVG architecture diagram                    │   │
    │  │  (centered, max-width 48rem)                          │   │
    │  └──────────────────────────────────────────────────────┘   │
    │                                                              │
    ├──────────────────────────────────────────────────────────────┤
    │                                                              │
    │  BENCHMARKS SECTION                                          │
    │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐               │
    │  │ 1000+  │ │  6     │ │  10×   │ │  52    │               │
    │  │ docs/  │ │ query  │ │ faster │ │ pages  │               │
    │  │  min   │ │ modes  │ │        │ │ docs   │               │
    │  └────────┘ └────────┘ └────────┘ └────────┘               │
    │       col-span-3 × 4 stat cards                              │
    │                                                              │
    ├──────────────────────────────────────────────────────────────┤
    │                                                              │
    │  QUICKSTART + ECOSYSTEM + ENTERPRISE CTA                     │
    │  (stacked full-width sections, alternating backgrounds)      │
    │                                                              │
    ├──────────────────────────────────────────────────────────────┤
    │                        FOOTER                                │
    └──────────────────────────────────────────────────────────────┘
```

### 4.2 Mobile Wireframe (< 768px)

```
    ┌──────────────────────────┐
    │       HEADER (mobile)    │
    ├──────────────────────────┤
    │                          │
    │  Graph-RAG.              │
    │  Built for Speed.        │
    │                          │
    │  Turn documents into     │
    │  knowledge graphs...     │
    │                          │
    │  [Get Started ─▸]        │
    │  [🐙 GitHub    ]         │
    │                          │
    │  (graph animation hidden │
    │   on mobile: lg:flex)    │
    │                          │
    ├──────────────────────────┤
    │  Problem cards stacked   │
    │  ┌──────────────────┐   │
    │  │ Problem 1        │   │
    │  └──────────────────┘   │
    │  ┌──────────────────┐   │
    │  │ Problem 2        │   │
    │  └──────────────────┘   │
    │  ┌──────────────────┐   │
    │  │ Problem 3        │   │
    │  └──────────────────┘   │
    ├──────────────────────────┤
    │  Stats: 2×2 grid         │
    │  ┌────────┐ ┌────────┐  │
    │  │ 1000+  │ │  6     │  │
    │  └────────┘ └────────┘  │
    │  ┌────────┐ ┌────────┐  │
    │  │ 10×    │ │  52    │  │
    │  └────────┘ └────────┘  │
    ├──────────────────────────┤
    │       FOOTER (stacked)   │
    └──────────────────────────┘
```

### 4.3 Section Rhythm

| Section        | Desktop Padding | Mobile Padding  | Background      |
| -------------- | --------------- | --------------- | --------------- |
| Hero           | `py-24 → py-32` | `py-16 → py-20` | transparent     |
| Problem        | `py-16 → py-24` | `py-12 → py-16` | `surface-muted` |
| Solution       | `py-16 → py-24` | `py-12 → py-16` | transparent     |
| Architecture   | `py-16 → py-24` | `py-12 → py-16` | `surface-muted` |
| Benchmarks     | `py-16 → py-24` | `py-12 → py-16` | transparent     |
| Quick Start    | `py-16 → py-24` | `py-12 → py-16` | `surface-muted` |
| Ecosystem      | `py-16 → py-24` | `py-12 → py-16` | transparent     |
| Enterprise CTA | `py-16 → py-24` | `py-12 → py-16` | accent gradient |

> **Pattern source:** astro.build alternates section backgrounds (transparent / subtle gray) to create visual rhythm without heavy borders. Proton uses the same technique with even more whitespace.

---

## 5. Documentation Page Layout (`/docs/**`)

### 5.1 Desktop Wireframe (≥ 1024px)

````
    ┌──────────────────────────────────────────────────────────────┐
    │                     HEADER (shared)                          │
    ├──────────┬──────────────────────────────────┬────────────────┤
    │          │                                   │               │
    │ SIDEBAR  │        CONTENT                    │    TOC        │
    │ 18rem    │        52rem max                  │   14rem       │
    │          │                                   │               │
    │ ┌──────┐ │  ┌──────────────────────────┐    │  On this page │
    │ │🔍    │ │  │  # Entity Extraction      │    │  ──────────── │
    │ │Search│ │  │                           │    │  ▸ Overview   │
    │ └──────┘ │  │  Overview text at 1rem    │    │  ▸ Algorithm  │
    │          │  │  with 1.7 line-height.    │    │  ▸ Config     │
    │ Getting  │  │  Comfortable for long     │    │  ▸ Examples   │
    │ Started  │  │  reading sessions.        │    │  ▸ API        │
    │  ▸ Install│  │                           │    │               │
    │  ▸ Quick │  │  ## Algorithm              │    │  (sticky      │
    │          │  │                           │    │   top: 5rem)  │
    │ Concepts │  │  ```rust                  │    │               │
    │  ▸ Entity│  │  fn extract() {           │    │               │
    │  ▸ Graph │  │      // code block        │    │               │
    │  ▸ Hybrid│  │  }                        │    │               │
    │          │  │  ```                      │    │               │
    │ API Ref  │  │                           │    │               │
    │  ▸ REST  │  │  :::tip                   │    │               │
    │  ▸ Upload│  │  Helpful tip here         │    │               │
    │          │  │  :::                      │    │               │
    │          │  │                           │    │               │
    │          │  │  ──────────────────────── │    │               │
    │          │  │  ◀ Previous  Next ▶       │    │               │
    │          │  │  Last updated: ...         │    │               │
    │          │  └──────────────────────────┘    │               │
    │          │                                   │               │
    └──────────┴──────────────────────────────────┴────────────────┘
````

### 5.2 Tablet Wireframe (768px–1023px)

```
    ┌──────────────────────────────────────────────┐
    │                  HEADER                       │
    ├──────────┬──────────────────────────────────┤
    │          │                                   │
    │ SIDEBAR  │        CONTENT                    │
    │ 16rem    │        (fills remaining)          │
    │          │                                   │
    │ (same as │  # Entity Extraction              │
    │  desktop │                                   │
    │  but     │  Body text fills available width  │
    │  narrower│  TOC hidden — accessed via        │
    │  w/      │  "On this page" expandable at     │
    │  collapse│  top of content                   │
    │  groups) │                                   │
    │          │                                   │
    └──────────┴──────────────────────────────────┘
```

### 5.3 Mobile Wireframe (< 768px)

````
    ┌──────────────────────────┐
    │       HEADER (mobile)    │
    ├──────────────────────────┤
    │ [☰ Menu]  [On this page] │
    ├──────────────────────────┤
    │                          │
    │  # Entity Extraction     │
    │                          │
    │  Body text fills full    │
    │  viewport width with     │
    │  1rem horizontal padding │
    │                          │
    │  ```rust                 │
    │  fn extract() {          │
    │      // scrollable       │
    │  }                        │
    │  ```                     │
    │                          │
    │  ◀ Previous   Next ▶     │
    │                          │
    └──────────────────────────┘

    [☰ Menu] opens sidebar as full-screen overlay
    [On this page] opens TOC as collapsible
````

### 5.4 Docs Layout Specifications

| Element             | Desktop       | Tablet        | Mobile         |
| ------------------- | ------------- | ------------- | -------------- |
| Sidebar width       | 18rem         | 16rem         | Full overlay   |
| Content max-width   | 52rem         | Fluid         | Fluid          |
| TOC width           | 14rem         | Hidden        | Collapsible    |
| Content padding-x   | 2rem          | 1.5rem        | 1rem           |
| Content padding-y   | 2rem          | 1.5rem        | 1rem           |
| Code block overflow | scroll-x      | scroll-x      | scroll-x       |
| Heading anchor      | Visible hover | Visible hover | Always visible |

> **Pattern source:** Cloudflare Docs uses this exact three-panel layout with similar widths. Starlight's default layout provides this out of the box; we only adjust widths via CSS custom properties.

---

## 6. Demo Page Layout (`/demo/`)

### 6.1 Desktop Wireframe

```
    ┌──────────────────────────────────────────────────────────────┐
    │                        HEADER                                │
    ├──────────────────────────────────────────────────────────────┤
    │                                                              │
    │  ┌──────────────────────────────────────────────────────┐   │
    │  │  H1: Try EdgeQuake Live                               │   │
    │  │  Subtitle: Experience graph-RAG in your browser       │   │
    │  └──────────────────────────────────────────────────────┘   │
    │                                                              │
    │  ┌─────────────────────────────────────────────────────────┐│
    │  │  MODE SELECTOR TABS                                     ││
    │  │  [Naive] [Local] [Global] [Hybrid] [Mix] [Graph]       ││
    │  └─────────────────────────────────────────────────────────┘│
    │                                                              │
    │  ┌──────────────────────┐  ┌──────────────────────────┐     │
    │  │  INPUT PANEL          │  │  RESULT PANEL            │     │
    │  │                       │  │                          │     │
    │  │  Sample documents     │  │  Query response with     │     │
    │  │  or paste your own    │  │  highlighted entities    │     │
    │  │                       │  │  and graph references    │     │
    │  │  ┌─────────────────┐ │  │                          │     │
    │  │  │  [Run Query ▸]  │ │  │  ┌─ ─ ─ ─ ─ ─ ─ ─ ─ ┐ │     │
    │  │  └─────────────────┘ │  │  │  Mini Graph View   │ │     │
    │  │                       │  │  │  (React Island)    │ │     │
    │  │                       │  │  └─ ─ ─ ─ ─ ─ ─ ─ ─ ┘ │     │
    │  └──────────────────────┘  └──────────────────────────┘     │
    │         col-span-6              col-span-6                   │
    │                                                              │
    ├──────────────────────────────────────────────────────────────┤
    │                        FOOTER                                │
    └──────────────────────────────────────────────────────────────┘
```

### 6.2 Interactive Elements

| Element         | Hydration Strategy | Island Directive    | Size Budget |
| --------------- | ------------------ | ------------------- | ----------- |
| Mode selector   | `client:load`      | Immediate, tab UX   | < 5KB       |
| Input panel     | `client:load`      | Textarea + controls | < 3KB       |
| Result panel    | `client:load`      | Streaming display   | < 8KB       |
| Mini graph view | `client:visible`   | Canvas, see hero    | < 15KB      |

---

## 7. Ecosystem Page Layout (`/ecosystem/`)

### 7.1 Desktop Wireframe

```
    ┌──────────────────────────────────────────────────────────────┐
    │                        HEADER                                │
    ├──────────────────────────────────────────────────────────────┤
    │                                                              │
    │  H1: Ecosystem                                               │
    │  Sub: SDKs, integrations, and community tools                │
    │                                                              │
    │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
    │  │ Rust SDK  │  │ Python   │  │ TypeScript│  │ REST API │   │
    │  │          │  │ SDK      │  │ SDK       │  │          │   │
    │  │ crate    │  │ pkg      │  │ npm       │  │ OpenAPI  │   │
    │  │ badge    │  │ badge    │  │ badge     │  │ spec     │   │
    │  │          │  │          │  │           │  │          │   │
    │  │ [View ▸] │  │ [View ▸] │  │ [View ▸]  │  │ [View ▸] │   │
    │  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
    │       col-span-3 × 4 cards                                   │
    │                                                              │
    │  H2: Integrations                                            │
    │  ┌──────────┐  ┌──────────┐  ┌──────────┐                  │
    │  │ LangChain│  │ Open     │  │ Custom   │                  │
    │  │          │  │ WebUI    │  │ Clients  │                  │
    │  └──────────┘  └──────────┘  └──────────┘                  │
    │                                                              │
    ├──────────────────────────────────────────────────────────────┤
    │                        FOOTER                                │
    └──────────────────────────────────────────────────────────────┘
```

---

## 8. Enterprise Page Layout (`/enterprise/`)

### 8.1 Desktop Wireframe

```
    ┌──────────────────────────────────────────────────────────────┐
    │                        HEADER                                │
    ├──────────────────────────────────────────────────────────────┤
    │                                                              │
    │  HERO (centered, narrower: max-w-3xl)                        │
    │  ┌──────────────────────────────────────────────────────┐   │
    │  │   H1: Enterprise-Grade Graph-RAG                      │   │
    │  │   Sub: Deploy EdgeQuake at scale with dedicated       │   │
    │  │   support, SLAs, and custom integrations              │   │
    │  │                                                       │   │
    │  │   [Contact Sales ▸]    [Book a Demo]                  │   │
    │  └──────────────────────────────────────────────────────┘   │
    │                                                              │
    │  FEATURES GRID (py-24)                                       │
    │  ┌──────────┐  ┌──────────┐  ┌──────────┐                  │
    │  │ 🔒 SOC2  │  │ 📊 SLA   │  │ 🔧 Custom│                  │
    │  │ Compliant│  │ 99.9%    │  │ Deploy   │                  │
    │  │          │  │          │  │          │                  │
    │  │ text...  │  │ text...  │  │ text...  │                  │
    │  └──────────┘  └──────────┘  └──────────┘                  │
    │                                                              │
    │  LOGOS ROW (trusted by)                                      │
    │  ┌──┐  ┌──┐  ┌──┐  ┌──┐  ┌──┐  ┌──┐                       │
    │  │  │  │  │  │  │  │  │  │  │  │  │  (grayscale logos)     │
    │  └──┘  └──┘  └──┘  └──┘  └──┘  └──┘                       │
    │                                                              │
    │  CTA BAND (accent background)                                │
    │  ┌──────────────────────────────────────────────────────┐   │
    │  │  Ready to get started?   [Contact Us ▸]               │   │
    │  └──────────────────────────────────────────────────────┘   │
    │                                                              │
    ├──────────────────────────────────────────────────────────────┤
    │                        FOOTER                                │
    └──────────────────────────────────────────────────────────────┘
```

---

## 9. Contact Page Layout (`/contact/`)

### 9.1 Desktop Wireframe

```
    ┌──────────────────────────────────────────────────────────────┐
    │                        HEADER                                │
    ├──────────────────────────────────────────────────────────────┤
    │                                                              │
    │  ┌──────────────────────┐  ┌──────────────────────────┐     │
    │  │  H1: Get in Touch     │  │  CONTACT FORM            │     │
    │  │                       │  │  ┌────────────────────┐  │     │
    │  │  Subtitle: We'd love  │  │  │ Name               │  │     │
    │  │  to hear from you.    │  │  ├────────────────────┤  │     │
    │  │                       │  │  │ Email              │  │     │
    │  │  📧 hello@edgequake   │  │  ├────────────────────┤  │     │
    │  │  🐙 GitHub Issues     │  │  │ Subject ▼         │  │     │
    │  │  💬 Discord           │  │  ├────────────────────┤  │     │
    │  │                       │  │  │ Message            │  │     │
    │  │                       │  │  │                    │  │     │
    │  │                       │  │  │                    │  │     │
    │  │                       │  │  ├────────────────────┤  │     │
    │  │                       │  │  │ [Send Message ▸]   │  │     │
    │  │                       │  │  └────────────────────┘  │     │
    │  └──────────────────────┘  └──────────────────────────┘     │
    │         col-span-5              col-span-7                   │
    │                                                              │
    ├──────────────────────────────────────────────────────────────┤
    │                        FOOTER                                │
    └──────────────────────────────────────────────────────────────┘

    Form: React island (client:load) for validation
    Submission: Client-side (mailto: or Formspree)
```

---

## 10. 404 Page Layout

Uses Starlight's `splash` template with a centered error message:

```
    ┌──────────────────────────────────────────────────────────────┐
    │                        HEADER                                │
    ├──────────────────────────────────────────────────────────────┤
    │                                                              │
    │                                                              │
    │                     ┌─────────────┐                          │
    │                     │    404      │                          │
    │                     │             │                          │
    │                     │ Page not    │                          │
    │                     │ found.      │                          │
    │                     │             │                          │
    │                     │ [Home ▸]    │                          │
    │                     │ [Docs ▸]    │                          │
    │                     │ [Search 🔍] │                          │
    │                     └─────────────┘                          │
    │                                                              │
    │                                                              │
    ├──────────────────────────────────────────────────────────────┤
    │                        FOOTER                                │
    └──────────────────────────────────────────────────────────────┘

    template: splash
    No sidebar, no TOC
    Centered content, max-w-md
```

---

## 11. Shared Footer

### 11.1 Desktop Footer

```
    ┌──────────────────────────────────────────────────────────────┐
    │  border-t border-border                                      │
    │                                                              │
    │  ⚡ EdgeQuake      Product    Developers   Community  Company│
    │                                                              │
    │  Graph-RAG         Get        Docs         GitHub     Contact│
    │  framework         Started    Core         Issues     Elitizon│
    │  built in Rust.    Demo       Concepts     Discuss    License│
    │  Apache 2.0.       Ecosystem  API Ref      Changelog         │
    │                    Enterprise crates.io                       │
    │                                                              │
    │  ────────────────────────────────────────────────────────── │
    │  © 2026 EdgeQuake · Built by Elitizon · Apache 2.0  GitHub  │
    └──────────────────────────────────────────────────────────────┘

    Grid: 5 columns (brand + 4 link groups)
    Max-width: 80rem, centered
    Padding: py-16 → py-20
    Bottom bar: border-t, flex justify-between
```

### 11.2 Mobile Footer

```
    ┌──────────────────────────┐
    │  ⚡ EdgeQuake             │
    │  Graph-RAG framework...  │
    │                          │
    │  Product                 │
    │    Get Started           │
    │    Demo                  │
    │    Ecosystem             │
    │    Enterprise            │
    │                          │
    │  Developers              │
    │    Docs                  │
    │    Core Concepts         │
    │    API Reference         │
    │    crates.io             │
    │                          │
    │  Community               │
    │    GitHub                │
    │    Issues                │
    │    Discussions           │
    │    Changelog             │
    │                          │
    │  Company                 │
    │    Contact               │
    │    Elitizon              │
    │    License               │
    │                          │
    │  ──────────────────────  │
    │  © 2026 EdgeQuake        │
    │  Built by Elitizon       │
    │  Apache 2.0    GitHub    │
    └──────────────────────────┘

    Grid: 2 columns (brand full, links 2-col)
    Links collapse to single column on very small screens
```

---

## 12. Responsive Behavior Summary

| Component      | Mobile (< 768)    | Tablet (768–1023) | Desktop (≥ 1024)   |
| -------------- | ----------------- | ----------------- | ------------------ |
| Header nav     | Hamburger menu    | Inline links      | Inline links       |
| Hero graph     | Hidden            | Hidden            | Visible (lg:flex)  |
| Hero layout    | Stacked           | Stacked           | 2-column grid      |
| Feature cards  | Stacked           | 2-column          | 3-column           |
| Stat cards     | 2×2 grid          | 4-column          | 4-column           |
| Docs sidebar   | Full overlay      | Visible (narrow)  | Visible (18rem)    |
| Docs TOC       | Collapsible top   | Hidden            | Sticky right panel |
| Footer columns | Stacked (2-col)   | 3-column          | 5-column           |
| Code blocks    | Horizontal scroll | Horizontal scroll | Horizontal scroll  |
| Contact form   | Full width        | 7/12 width        | 7/12 width         |

---

## 13. Page Transition Strategy

| Transition Type    | Implementation                  | Duration | Notes                   |
| ------------------ | ------------------------------- | -------- | ----------------------- |
| Docs page swap     | Starlight default (full reload) | 0ms      | Static pages, fast TTFB |
| Marketing sections | CSS fade-in on scroll           | 400ms    | `IntersectionObserver`  |
| Mobile menu        | CSS `max-height` accordion      | 200ms    | No JS framework needed  |
| Search modal       | Pagefind's built-in animation   | 200ms    | Overlay + backdrop blur |

> **Decision:** No view transitions API or client-side routing. Each page is a static HTML document. Time-to-first-byte is the priority. This matches Cloudflare Docs and Netlify Docs approach — fast initial loads, minimal client JS.

---

## 14. Accessibility Requirements

| Requirement          | Implementation                                         |
| -------------------- | ------------------------------------------------------ |
| Skip to content link | Starlight built-in; marketing layout adds same         |
| Focus visible ring   | `0 0 0 2px var(--accent)` on `:focus-visible`          |
| Touch targets        | Minimum 44×44px for all interactive elements           |
| Color independence   | Information never conveyed by color alone              |
| Heading hierarchy    | Single H1 per page, sequential H2→H3→H4                |
| Landmark regions     | `<header>`, `<nav>`, `<main>`, `<aside>`, `<footer>`   |
| Alt text             | All images have descriptive alt; decorative = `alt=""` |
| Keyboard navigation  | Tab order follows visual order, no traps               |
| Reduced motion       | See [11-design-system.md §8.4](./11-design-system.md)  |

---

_Cross-references: [00-overview](./00-overview.md) · [03-info-architecture](./03-information-architecture.md) · [04-project-setup](./04-starlight-project-setup.md) · [11-design-system](./11-design-system.md) · [13-components](./13-component-library.md)_
