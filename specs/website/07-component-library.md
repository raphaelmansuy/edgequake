# SPEC-WEBSITE-07: Component Library

> **Status**: DRAFT  
> **Created**: 2026-03-21  
> **Parent**: [00-overview.md](./00-overview.md)  
> **Related**: [03-page-specifications.md](./03-page-specifications.md) · [06-design-system.md](./06-design-system.md) · [04-technical-architecture.md](./04-technical-architecture.md)

---

## 1. shadcn/ui Components (Pre-built)

Components installed via `pnpm dlx shadcn@latest add <name>`.

| Component        | Usage                                            | Pages           |
| ---------------- | ------------------------------------------------ | --------------- |
| `Button`         | CTAs, links, form submit                         | All             |
| `Card`           | Feature cards, crate cards, pain cards           | Home, Ecosystem |
| `Tabs`           | Code tabs (Rust/Docker/API), query mode selector | Home, Demo      |
| `Sheet`          | Mobile navigation drawer                         | All (mobile)    |
| `NavigationMenu` | Desktop header navigation with dropdowns         | All             |
| `Accordion`      | FAQ section, collapsible sidebar sections        | Docs, Home      |
| `Badge`          | Version tags, status indicators, tech labels     | All             |
| `Input`          | Contact form fields                              | Contact         |
| `Select`         | Inquiry type dropdown                            | Contact         |
| `Textarea`       | Message field                                    | Contact         |
| `Separator`      | Section dividers                                 | All             |
| `Tooltip`        | Architecture diagram hover info                  | Home, Docs      |
| `DropdownMenu`   | Theme switcher, mobile overflow                  | All             |
| `Dialog`         | Confirmations, details popups                    | Contact         |
| `ScrollArea`     | Docs sidebar, long code blocks                   | Docs            |
| `Toggle`         | Dark mode switch                                 | All             |
| `Skeleton`       | Loading states for demo                          | Demo            |

---

## 2. Custom Components

### 2.1 Layout Components

#### `Header`

```
┌──────────────────────────────────────────────────────────────┐
│  🔷 EdgeQuake    Docs  Demo  Ecosystem  Enterprise           │
│                                          [GitHub ★]  [CTA]  │
└──────────────────────────────────────────────────────────────┘
```

| Prop | Type | Description                            |
| ---- | ---- | -------------------------------------- |
| —    | —    | No props. Reads path for active state. |

**Behavior:**

- Sticky on scroll (`position: sticky; top: 0`)
- Backdrop blur when scrolled (`backdrop-blur-md`)
- Active link underlined
- Mobile: collapses to hamburger → `Sheet` drawer
- GitHub badge shows live star count (build-time fetched)
- CTA: "Get Started" → `/docs/getting-started`

#### `Footer`

```
┌──────────────────────────────────────────────────────────────┐
│  Product      Developers    Company      Community           │
│  Features     Quick Start   About        GitHub              │
│  Demo         API Docs      Enterprise   Twitter/X           │
│  Ecosystem    Guides        Contact      Discord             │
│  Changelog    Comparisons   Blog                             │
│                                                              │
│  ──────────────────────────────────────────────────────────  │
│  🔷 EdgeQuake · Built by Elitizon · Apache 2.0 License       │
│  © 2026 All rights reserved                                  │
└──────────────────────────────────────────────────────────────┘
```

#### `DocsSidebar`

| Prop          | Type               | Description                  |
| ------------- | ------------------ | ---------------------------- |
| `sections`    | `SidebarSection[]` | Tree of navigation items     |
| `currentPath` | `string`           | Active page for highlighting |

**Behavior:**

- Collapsible section groups (accordion)
- Active page highlighted with left border accent
- Scroll synced to active item
- Mobile: opens as `Sheet` overlay

#### `MobileNav`

Sheet drawer component for mobile navigation. Triggered by hamburger icon in Header.

---

### 2.2 Homepage Components

#### `Hero`

| Prop | Type | Description              |
| ---- | ---- | ------------------------ |
| —    | —    | Static content, no props |

**Elements:**

- Animated background (knowledge graph SVG with subtle node pulses)
- Gradient H1: "Graph-RAG Built for Speed"
- Subtitle paragraph
- 3 CTA buttons (primary, secondary, ghost)
- Badge row (4 pills)
- Scroll indicator chevron

**Client component:** Yes (animations require `"use client"`)

#### `ProblemSection`

| Prop    | Type         | Description            |
| ------- | ------------ | ---------------------- |
| `cards` | `PainCard[]` | Array of problem cards |

```typescript
interface PainCard {
  icon: LucideIcon;
  headline: string;
  body: string;
}
```

**Layout:** 3-column grid (1 on mobile), stagger animation on scroll.

#### `SolutionGrid`

| Prop       | Type        | Description         |
| ---------- | ----------- | ------------------- |
| `features` | `Feature[]` | Array of 6 features |

```typescript
interface Feature {
  icon: LucideIcon;
  headline: string;
  body: string;
  cta: { label: string; href: string };
}
```

**Layout:** 3-column grid, card hover lifts with shadow transition.

#### `ArchitectureDiagram`

| Prop | Type | Description                     |
| ---- | ---- | ------------------------------- |
| —    | —    | SVG-based, interactive tooltips |

**Implementation:**

- SVG inline component with hover tooltips (`Tooltip` from shadcn)
- Shows: Documents → Pipeline → Storage (pgvector + AGE) → Query Engine → API → Clients
- Nodes are clickable, linking to relevant docs page
- Progressive draw animation on scroll entry

**Client component:** Yes

#### `BenchmarkChart`

| Prop     | Type              | Description     |
| -------- | ----------------- | --------------- |
| `data`   | `BenchmarkData[]` | Comparison data |
| `metric` | `string`          | Chart title     |

```typescript
interface BenchmarkData {
  name: string;
  value: number;
  color: string;
  isHighlighted: boolean;
}
```

**Implementation:**

- Horizontal bar chart using CSS (no charting library)
- Bars animate width from 0% to target on scroll
- EdgeQuake bar highlighted with brand gradient
- Responsive: bars stack naturally

**Client component:** Yes (scroll animation)

#### `QuickStart`

| Prop   | Type        | Description                |
| ------ | ----------- | -------------------------- |
| `tabs` | `CodeTab[]` | Code examples per language |

```typescript
interface CodeTab {
  label: string; // "Rust" | "Docker" | "REST API"
  language: string; // for Shiki highlighting
  code: string; // raw code string
}
```

**Implementation:**

- Uses shadcn `Tabs` component
- Code highlighted at build time via Shiki
- Copy-to-clipboard button on each block
- "Read the Full Guide →" link below

#### `EcosystemPreview`

| Prop     | Type             | Description       |
| -------- | ---------------- | ----------------- |
| `crates` | `CrateSummary[]` | Top 5 crate cards |

**Layout:** Horizontal scroll on mobile, 5-column grid on desktop. "Explore All Crates →" link.

#### `EnterpriseCTA`

| Prop | Type | Description   |
| ---- | ---- | ------------- |
| —    | —    | Static banner |

**Design:** Full-width gradient background. Heading + paragraph + "Contact Us →" button.

---

### 2.3 Demo Components

#### `QueryPanel`

| Prop        | Type             | Description                |
| ----------- | ---------------- | -------------------------- |
| `modes`     | `QueryMode[]`    | Available query modes      |
| `questions` | `DemoQuestion[]` | Pre-built sample questions |

**Behavior:**

- Tab selector for 6 query modes
- Dropdown selector for sample questions
- Shows pre-computed result for selected mode + question combo

**Client component:** Yes

#### `GraphViewer`

| Prop    | Type          | Description        |
| ------- | ------------- | ------------------ |
| `nodes` | `GraphNode[]` | Entity nodes       |
| `edges` | `GraphEdge[]` | Relationship edges |

```typescript
interface GraphNode {
  id: string;
  label: string;
  type: "person" | "organization" | "location" | "concept" | "event";
  x?: number;
  y?: number;
}

interface GraphEdge {
  source: string;
  target: string;
  label: string;
  weight: number;
}
```

**Implementation:**

- D3.js force-directed layout
- Node colors mapped to entity type (see [06-design-system.md](./06-design-system.md))
- Click node → highlights connected edges
- Zoom + pan supported
- Mobile: fixed viewport (no zoom)

**Client component:** Yes

#### `ResultPanel`

| Prop      | Type        | Description       |
| --------- | ----------- | ----------------- |
| `content` | `string`    | Markdown response |
| `mode`    | `QueryMode` | Active query mode |

Renders markdown response with syntax highlighting.

---

### 2.4 Ecosystem Components

#### `CrateCard`

| Prop          | Type      | Description                |
| ------------- | --------- | -------------------------- |
| `name`        | `string`  | Crate name                 |
| `description` | `string`  | One-line description       |
| `category`    | `string`  | Core / API / Storage / ... |
| `docsUrl`     | `string`  | Link to docs section       |
| `sourceUrl`   | `string`  | Link to GitHub source      |
| `published`   | `boolean` | Whether on crates.io       |
| `version`     | `string`  | If published               |

**Layout:** Card with icon, name, description, category badge, two link buttons.

#### `IntegrationCard`

| Prop          | Type         | Description               |
| ------------- | ------------ | ------------------------- |
| `name`        | `string`     | Integration name          |
| `description` | `string`     | Brief description         |
| `icon`        | `LucideIcon` | Visual icon               |
| `docsUrl`     | `string`     | Link to integration guide |

---

### 2.5 Shared Components

#### `CodeBlock`

| Prop              | Type       | Description               |
| ----------------- | ---------- | ------------------------- |
| `code`            | `string`   | Source code               |
| `language`        | `string`   | Shiki language ID         |
| `filename`        | `string?`  | Optional file name header |
| `showLineNumbers` | `boolean?` | Default: false            |

**Implementation:**

- Shiki highlighted at build time
- Copy button (top-right)
- Optional filename tab header
- Scrollable overflow
- Themed for light + dark mode

#### `Callout`

| Prop       | Type                                       | Description      |
| ---------- | ------------------------------------------ | ---------------- |
| `type`     | `"info" \| "warning" \| "tip" \| "danger"` | Visual variant   |
| `title`    | `string?`                                  | Optional heading |
| `children` | `ReactNode`                                | Body content     |

```
┌─ ℹ️ Info ──────────────────────────────────┐
│  EdgeQuake uses Apache AGE for graph       │
│  storage in PostgreSQL.                    │
└────────────────────────────────────────────┘
```

#### `ScrollAnimation`

| Prop       | Type                                   | Description              |
| ---------- | -------------------------------------- | ------------------------ |
| `children` | `ReactNode`                            | Content to animate       |
| `variant`  | `"fadeIn" \| "slideUp" \| "slideLeft"` | Animation type           |
| `delay`    | `number?`                              | Stagger delay in seconds |

Wraps children in Framer Motion `motion.div` with Intersection Observer trigger.

**Client component:** Yes

#### `SeoHead`

| Prop          | Type      | Description      |
| ------------- | --------- | ---------------- |
| `title`       | `string`  | Page title       |
| `description` | `string`  | Meta description |
| `ogImage`     | `string?` | OG image path    |
| `canonical`   | `string?` | Canonical URL    |

Generates `<head>` meta via Next.js `generateMetadata()`.

---

## 3. Component Inventory Matrix

| Component           | Server/Client | JS Size (est.) | Pages Used       |
| ------------------- | ------------- | -------------- | ---------------- |
| Header              | Client        | 8KB            | All              |
| Footer              | Server        | 0KB            | All              |
| Hero                | Client        | 15KB           | Home             |
| ProblemSection      | Client        | 5KB            | Home             |
| SolutionGrid        | Client        | 6KB            | Home             |
| ArchitectureDiagram | Client        | 10KB           | Home             |
| BenchmarkChart      | Client        | 4KB            | Home             |
| QuickStart          | Client        | 3KB            | Home             |
| EcosystemPreview    | Server        | 0KB            | Home             |
| EnterpriseCTA       | Server        | 0KB            | Home, Enterprise |
| DocsSidebar         | Client        | 6KB            | Docs             |
| CodeBlock           | Server        | 0KB            | Docs, Home       |
| Callout             | Server        | 0KB            | Docs             |
| QueryPanel          | Client        | 5KB            | Demo             |
| GraphViewer         | Client        | 80KB           | Demo             |
| ResultPanel         | Server        | 0KB            | Demo             |
| CrateCard           | Server        | 0KB            | Ecosystem        |
| ContactForm         | Client        | 8KB            | Contact          |
| ScrollAnimation     | Client        | 2KB            | All              |

**Total unique JS:** ~152KB (before gzip, ~50KB gzipped)

---

## 4. Component File Organization

```
src/components/
├── ui/                    # shadcn/ui (auto-generated)
│   ├── button.tsx
│   ├── card.tsx
│   └── ... (16 components)
├── layout/
│   ├── header.tsx
│   ├── footer.tsx
│   ├── mobile-nav.tsx
│   └── docs-sidebar.tsx
├── home/
│   ├── hero.tsx
│   ├── problem-section.tsx
│   ├── solution-grid.tsx
│   ├── architecture-diagram.tsx
│   ├── benchmark-chart.tsx
│   ├── quick-start.tsx
│   ├── ecosystem-preview.tsx
│   └── enterprise-cta.tsx
├── demo/
│   ├── query-panel.tsx
│   ├── graph-viewer.tsx
│   └── result-panel.tsx
├── ecosystem/
│   ├── crate-card.tsx
│   └── integration-card.tsx
├── contact/
│   └── contact-form.tsx
└── shared/
    ├── code-block.tsx
    ├── callout.tsx
    ├── scroll-animation.tsx
    ├── seo-head.tsx
    └── mdx-components.tsx
```

---

## 5. MDX Component Mapping

```typescript
// src/components/shared/mdx-components.tsx
import { CodeBlock } from "./code-block";
import { Callout } from "./callout";

export const mdxComponents = {
  pre: CodeBlock, // Override default code blocks
  Callout, // <Callout type="info">...</Callout>
  CrateLink, // <CrateLink name="edgequake-core" />
  ApiEndpoint, // <ApiEndpoint method="POST" path="/api/v1/documents" />
  Architecture, // <Architecture /> inline SVG diagram
};
```

---

_Previous: [06-design-system.md](./06-design-system.md) · Next: [08-contact-lead-generation.md](./08-contact-lead-generation.md)_
