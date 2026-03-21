# SPEC-WEBSITE-03: Page Specifications

> **Status**: DRAFT  
> **Created**: 2026-03-21  
> **Parent**: [00-overview.md](./00-overview.md)  
> **Related**: [01-information-architecture.md](./01-information-architecture.md) · [02-content-strategy.md](./02-content-strategy.md) · [06-design-system.md](./06-design-system.md) · [07-component-library.md](./07-component-library.md)

---

## 1. Homepage (`/`)

### 1.1 Full Page Layout

```
╔══════════════════════════════════════════════════════════════════════╗
║  HEADER (sticky)                                                     ║
║  🔷 EdgeQuake    Docs  Demo  Ecosystem  Enterprise  [GitHub★] [CTA] ║
╠══════════════════════════════════════════════════════════════════════╣
║                                                                      ║
║                        HERO SECTION                                  ║
║                                                                      ║
║            Graph-RAG Built for Speed                                 ║
║                                                                      ║
║    Turn documents into knowledge graphs.                             ║
║    Query with 6 modes. 10x faster than Python RAG.                   ║
║                                                                      ║
║    [Get Started]  [Live Demo]  [GitHub →]                            ║
║                                                                      ║
║    ┌────────────────────────────────────────────┐                    ║
║    │  Animated knowledge graph visualization    │                    ║
║    │  (nodes + edges pulsing gently)            │                    ║
║    └────────────────────────────────────────────┘                    ║
║                                                                      ║
║    [Apache 2.0]  [1000+ docs/min]  [6 Query Modes]  [Rust]          ║
║                                                                      ║
╠══════════════════════════════════════════════════════════════════════╣
║                                                                      ║
║                     THE PROBLEM                                      ║
║            "Traditional RAG Loses Knowledge"                         ║
║                                                                      ║
║  ┌──────────┐  ┌──────────┐  ┌──────────┐                           ║
║  │ Lost     │  │ Slow at  │  │ Single   │                           ║
║  │ Relation-│  │ Scale    │  │ Query    │                           ║
║  │ ships    │  │          │  │ Mode     │                           ║
║  └──────────┘  └──────────┘  └──────────┘                           ║
║  ┌──────────┐  ┌──────────┐  ┌──────────┐                           ║
║  │ No Multi-│  │ PDF Pain │  │ Cloud-   │                           ║
║  │ Tenancy  │  │          │  │ Only Cost│                           ║
║  └──────────┘  └──────────┘  └──────────┘                           ║
║                                                                      ║
╠══════════════════════════════════════════════════════════════════════╣
║                                                                      ║
║                      THE SOLUTION                                    ║
║         "Graph-RAG That Actually Works in Production"                ║
║                                                                      ║
║  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐               ║
║  │ 🧠 Knowledge │  │ ⚡ 10x Faster│  │ 🔀 6 Query   │               ║
║  │ Graph Engine │  │              │  │ Modes        │               ║
║  │              │  │ Rust core... │  │ Naive, Local │               ║
║  │ [Learn →]   │  │ [Bench →]    │  │ Global,...   │               ║
║  └──────────────┘  └──────────────┘  └──────────────┘               ║
║  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐               ║
║  │ 🏢 Multi-    │  │ 📄 PDF Vision│  │ 🤖 MCP       │               ║
║  │ Tenant       │  │ Pipeline     │  │ Integration  │               ║
║  │              │  │              │  │              │               ║
║  │ [Enterprise] │  │ [PDF Guide]  │  │ [MCP Docs]   │               ║
║  └──────────────┘  └──────────────┘  └──────────────┘               ║
║                                                                      ║
╠══════════════════════════════════════════════════════════════════════╣
║                                                                      ║
║                    ARCHITECTURE                                      ║
║     "How It All Fits Together"                                       ║
║                                                                      ║
║  ┌────────────────────────────────────────────────┐                  ║
║  │  Interactive architecture diagram              │                  ║
║  │  (SVG with hover tooltips on each component)   │                  ║
║  │                                                │                  ║
║  │  Documents → Pipeline → [pgvector + AGE]       │                  ║
║  │                           ↕                    │                  ║
║  │  Clients ← API ← Query Engine                 │                  ║
║  └────────────────────────────────────────────────┘                  ║
║                                                                      ║
║  [View Full Architecture →]                                          ║
║                                                                      ║
╠══════════════════════════════════════════════════════════════════════╣
║                                                                      ║
║                     BENCHMARKS                                       ║
║       "Why EdgeQuake Outperforms Alternatives"                       ║
║                                                                      ║
║  ┌─── Throughput (docs/min) ─────────────────┐                       ║
║  │ EdgeQuake   ████████████████████████ 1000  │                      ║
║  │ LightRAG    ████                      100  │                      ║
║  │ GraphRAG    ██                          50 │                      ║
║  │ Trad. RAG   ████████                  200  │                      ║
║  └────────────────────────────────────────────┘                      ║
║                                                                      ║
║  ┌─── Memory per Core ───────────────────────┐                       ║
║  │ EdgeQuake   ██                       300MB │                      ║
║  │ LightRAG    ████████████████         3GB   │                      ║
║  │ GraphRAG    ████████                 1.5GB │                      ║
║  └────────────────────────────────────────────┘                      ║
║                                                                      ║
║  [See Detailed Comparison →]                                         ║
║                                                                      ║
╠══════════════════════════════════════════════════════════════════════╣
║                                                                      ║
║                   QUICK START                                        ║
║         "Get Started in 60 Seconds"                                  ║
║                                                                      ║
║  [Rust]  [Docker]  [REST API]                                        ║
║  ┌────────────────────────────────────────────────┐                  ║
║  │  cargo add edgequake-core edgequake-llm        │                  ║
║  │                                                │                  ║
║  │  // ... code example ...                       │                  ║
║  └────────────────────────────────────────────────┘                  ║
║                                                                      ║
║  [Read the Full Guide →]                                             ║
║                                                                      ║
╠══════════════════════════════════════════════════════════════════════╣
║                                                                      ║
║                    ECOSYSTEM                                         ║
║         "10 Modular Crates, Infinite Possibilities"                  ║
║                                                                      ║
║  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐                           ║
║  │core │ │api  │ │store│ │pipe │ │query│  ...5 more                  ║
║  └─────┘ └─────┘ └─────┘ └─────┘ └─────┘                           ║
║                                                                      ║
║  [Explore All Crates →]                                              ║
║                                                                      ║
╠══════════════════════════════════════════════════════════════════════╣
║                                                                      ║
║              ENTERPRISE CTA BANNER                                   ║
║                                                                      ║
║   ┌──────────────────────────────────────────────────────┐           ║
║   │                                                      │           ║
║   │  Need EdgeQuake for your organization?               │           ║
║   │                                                      │           ║
║   │  Get dedicated support, custom integrations, and     │           ║
║   │  architecture consulting from the EdgeQuake team.    │           ║
║   │                                                      │           ║
║   │  [Contact Us →]                                      │           ║
║   │                                                      │           ║
║   └──────────────────────────────────────────────────────┘           ║
║                                                                      ║
╠══════════════════════════════════════════════════════════════════════╣
║                       FOOTER                                         ║
║  Product  |  Developers  |  Company  |  Community                    ║
║  © 2026 EdgeQuake · Built by Elitizon · Apache 2.0                   ║
╚══════════════════════════════════════════════════════════════════════╝
```

### 1.2 Hero Section Spec

| Element | Spec |
|---------|------|
| **Background** | Subtle animated graph network (particles.js style or custom SVG) |
| **H1** | "Graph-RAG Built for Speed" — 48px / font-bold / gradient text |
| **Subtitle** | "Turn documents into knowledge graphs. Query with 6 modes. 10x faster than Python RAG." — 20px / text-muted |
| **Primary CTA** | "Get Started" → `/docs/getting-started` — solid button, primary color |
| **Secondary CTA** | "Live Demo" → `/demo` — outline button |
| **Tertiary CTA** | "GitHub →" → external link — ghost button with GitHub icon |
| **Badge Row** | 4 pills: Apache 2.0 · 1000+ docs/min · 6 Query Modes · Built with Rust |
| **Hero Visual** | Animated SVG knowledge graph with nodes (Person, Org, Concept) and edges pulsing |
| **Scroll Indicator** | Subtle "↓" chevron animation |

### 1.3 Section Specifications

| Section | ID | Scroll Position | Animation |
|---------|-----|----------------|-----------|
| Hero | `#hero` | 0-100vh | Fade-in on load |
| Problem | `#problem` | 100vh | Cards stagger in from bottom |
| Solution | `#solution` | 200vh | Cards slide in from sides |
| Architecture | `#architecture` | 300vh | SVG draws progressively |
| Benchmarks | `#benchmarks` | 400vh | Bars animate to width |
| Quick Start | `#quickstart` | 500vh | Tab content fades |
| Ecosystem | `#ecosystem` | 600vh | Grid fades in |
| Enterprise CTA | `#enterprise` | 650vh | Slide up from bottom |

---

## 2. Documentation Hub (`/docs`)

### 2.1 Layout

```
╔══════════════════════════════════════════════════════════════════════╗
║  HEADER                                                             ║
╠════════════════════╦═════════════════════════════════════════════════╣
║                    ║                                                 ║
║  SIDEBAR (240px)   ║  MAIN CONTENT (flex-1)                         ║
║                    ║                                                 ║
║  🔍 Search...      ║  ┌───────────────────────────────────────────┐ ║
║                    ║  │  Breadcrumbs: Home > Docs > Concepts      │ ║
║  ▼ Getting Started ║  │                                           │ ║
║    Installation    ║  │  # Entity Extraction                      │ ║
║    Quick Start     ║  │                                           │ ║
║    First Ingestion ║  │  EdgeQuake uses LLM-powered entity        │ ║
║                    ║  │  extraction to identify 7 entity types... │ ║
║  ▼ Core Concepts   ║  │                                           │ ║
║    Graph-RAG       ║  │  ## Entity Types                          │ ║
║    Entity Extract. ║  │  - Person                                 │ ║
║  ► Knowledge Graph ║  │  - Organization                           │ ║
║    Query Modes     ║  │  - Location                               │ ║
║    Hybrid Retriev. ║  │  - ...                                    │ ║
║                    ║  │                                           │ ║
║  ► Architecture    ║  │  ## Related                               │ ║
║  ► Guides          ║  │  → Knowledge Graphs                      │ ║
║  ► API Reference   ║  │  → Pipeline Architecture                 │ ║
║  ► Deployment      ║  │                                           │ ║
║  ► Comparisons     ║  └───────────────────────────────────────────┘ ║
║                    ║                                                 ║
║                    ║  ┌──────────┐  ┌──────────────┐                ║
║                    ║  │ ← Prev   │  │   Next →     │                ║
║                    ║  │ Graph-RAG│  │ Knowl. Graph │                ║
║                    ║  └──────────┘  └──────────────┘                ║
║                    ║                                                 ║
╠════════════════════╩═════════════════════════════════════════════════╣
║  FOOTER                                                             ║
╚══════════════════════════════════════════════════════════════════════╝
```

### 2.2 Documentation Page Elements

| Element | Description |
|---------|------------|
| **Sidebar** | Collapsible tree navigation, active item highlighted, scroll-synced |
| **Search** | Static search (Pagefind) with keyboard shortcut (⌘K) |
| **TOC** | Right-side table of contents (hidden on mobile, visible ≥1280px) |
| **Breadcrumbs** | Full path from Home |
| **Prev/Next** | Navigation between sequential pages |
| **Edit on GitHub** | Link to source MDX file |
| **Last Updated** | Git timestamp |
| **Reading Time** | Estimated from word count |

### 2.3 Docs Content Source

| Section | Source | Transform |
|---------|--------|-----------|
| Getting Started | `docs/getting-started/*.md` | Copy + adapt |
| Concepts | `docs/concepts/*.md` | Copy + adapt |
| Architecture | `docs/architecture/*.md` | Copy + adapt |
| Guides | `docs/tutorials/*.md` | Rename + adapt |
| API Reference | `docs/api-reference/*.md` + OpenAPI | Generate |
| Deployment | `docs/operations/*.md` | Copy + adapt |
| Comparisons | `docs/comparisons/*.md` | Copy + adapt |

---

## 3. Demo Page (`/demo`)

### 3.1 Layout

```
╔══════════════════════════════════════════════════════════════════════╗
║  HEADER                                                             ║
╠══════════════════════════════════════════════════════════════════════╣
║                                                                      ║
║                    INTERACTIVE DEMO                                  ║
║         "See EdgeQuake in Action"                                    ║
║                                                                      ║
║  ┌────────────────────────────────────────────────────────────┐      ║
║  │                                                            │      ║
║  │     QUERY MODE SELECTOR                                    │      ║
║  │     [Naive] [Local] [Global] [Hybrid] [Mix] [Bypass]      │      ║
║  │                                                            │      ║
║  │  ┌─────────────────────────────────────────────────────┐   │      ║
║  │  │  Sample Question: "Who are the key researchers      │   │      ║
║  │  │  and what organizations are they affiliated with?"  │   │      ║
║  │  └─────────────────────────────────────────────────────┘   │      ║
║  │                                                            │      ║
║  │  ┌─────────────────────┐  ┌────────────────────────────┐  │      ║
║  │  │                     │  │                            │  │      ║
║  │  │  KNOWLEDGE GRAPH    │  │  QUERY RESULT              │  │      ║
║  │  │  (interactive SVG)  │  │  (markdown rendered)       │  │      ║
║  │  │                     │  │                            │  │      ║
║  │  │  ● SARAH_CHEN       │  │  Based on the knowledge   │  │      ║
║  │  │   ├─works_at─► ACME │  │  graph, Sarah Chen works  │  │      ║
║  │  │   └─authored─► ...  │  │  at Acme Corp and has     │  │      ║
║  │  │                     │  │  authored 3 papers on...   │  │      ║
║  │  │  ● ACME_CORP        │  │                            │  │      ║
║  │  │   ├─sector─► TECH   │  │                            │  │      ║
║  │  │   └─hq─► SF         │  │                            │  │      ║
║  │  │                     │  │                            │  │      ║
║  │  └─────────────────────┘  └────────────────────────────┘  │      ║
║  │                                                            │      ║
║  └────────────────────────────────────────────────────────────┘      ║
║                                                                      ║
║  Note: This demo uses pre-computed results from a sample dataset.    ║
║  [Try with your data → Docker Quick Start]                           ║
║                                                                      ║
╠══════════════════════════════════════════════════════════════════════╣
║  FOOTER                                                             ║
╚══════════════════════════════════════════════════════════════════════╝
```

### 3.2 Demo Implementation

| Aspect | Approach |
|--------|---------|
| **Data** | Pre-computed JSON results from sample dataset (build-time generated) |
| **Graph Viz** | Sigma.js or D3.js force-directed graph embedded as client component |
| **Query Modes** | Tab selector shows different results for same question |
| **No Server** | All data bundled statically; no runtime API calls |
| **Mobile** | Graph and result stack vertically |

---

## 4. Ecosystem Page (`/ecosystem`)

### 4.1 Layout

```
╔══════════════════════════════════════════════════════════════════════╗
║  HEADER                                                             ║
╠══════════════════════════════════════════════════════════════════════╣
║                                                                      ║
║                       ECOSYSTEM                                      ║
║       "10 Modular Crates, Infinite Possibilities"                    ║
║                                                                      ║
║  ┌── Filter: [All] [Core] [API] [Storage] [LLM] [Tools] ──┐        ║
║  │                                                          │        ║
║  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │        ║
║  │  │ edgequake-   │  │ edgequake-   │  │ edgequake-   │   │        ║
║  │  │ core         │  │ api          │  │ storage      │   │        ║
║  │  │              │  │              │  │              │   │        ║
║  │  │ Orchestrat-  │  │ REST API     │  │ Vector +     │   │        ║
║  │  │ ion layer    │  │ with SSE,    │  │ Graph +      │   │        ║
║  │  │ with types   │  │ OpenAPI,     │  │ KV adapters  │   │        ║
║  │  │ and pipeline │  │ Swagger      │  │ for PG       │   │        ║
║  │  │              │  │              │  │              │   │        ║
║  │  │ [Docs] [Src] │  │ [Docs] [Src] │  │ [Docs] [Src] │   │        ║
║  │  └──────────────┘  └──────────────┘  └──────────────┘   │        ║
║  │                                                          │        ║
║  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │        ║
║  │  │ edgequake-   │  │ edgequake-   │  │ edgequake-   │   │        ║
║  │  │ pipeline     │  │ query        │  │ llm          │   │        ║
║  │  │ ...          │  │ ...          │  │ ...          │   │        ║
║  │  └──────────────┘  └──────────────┘  └──────────────┘   │        ║
║  │                                                          │        ║
║  │  ... (4 more crate cards)                                │        ║
║  └──────────────────────────────────────────────────────────┘        ║
║                                                                      ║
║  ────────────── INTEGRATIONS ──────────────                          ║
║                                                                      ║
║  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐               ║
║  │ 🤖 MCP       │  │ 🐳 Docker    │  │ ☸️ Kubernetes │               ║
║  │ Server       │  │              │  │              │               ║
║  │ 18 tools     │  │ Single-      │  │ Health       │               ║
║  │ for AI agents│  │ container    │  │ probes,      │               ║
║  │              │  │ deployment   │  │ HPA ready    │               ║
║  └──────────────┘  └──────────────┘  └──────────────┘               ║
║                                                                      ║
╠══════════════════════════════════════════════════════════════════════╣
║  FOOTER                                                             ║
╚══════════════════════════════════════════════════════════════════════╝
```

---

## 5. Enterprise Page (`/enterprise`)

### 5.1 Layout

```
╔══════════════════════════════════════════════════════════════════════╗
║  HEADER                                                             ║
╠══════════════════════════════════════════════════════════════════════╣
║                                                                      ║
║                     ENTERPRISE                                       ║
║    "Production-Grade Graph-RAG for Your Organization"                ║
║                                                                      ║
║  ┌────────────────────────────────────────────────────────────┐      ║
║  │  WHY ENTERPRISE?                                           │      ║
║  │                                                            │      ║
║  │  EdgeQuake is 100% open source (Apache 2.0).               │      ║
║  │  Enterprise support provides:                              │      ║
║  │                                                            │      ║
║  │  ✓ Dedicated technical support                             │      ║
║  │  ✓ Architecture consulting                                │      ║
║  │  ✓ Custom integration development                         │      ║
║  │  ✓ Priority bug fixes                                     │      ║
║  │  ✓ Training and onboarding                                │      ║
║  │  ✓ SLA guarantees                                         │      ║
║  └────────────────────────────────────────────────────────────┘      ║
║                                                                      ║
║  ──────── ENTERPRISE FEATURES ────────                               ║
║                                                                      ║
║  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐               ║
║  │ 🏢 Multi-    │  │ 🔐 RBAC +   │  │ 📋 Audit     │               ║
║  │ Tenant       │  │ JWT Auth     │  │ Logging      │               ║
║  └──────────────┘  └──────────────┘  └──────────────┘               ║
║  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐               ║
║  │ ⚡ Rate      │  │ 🔄 Task      │  │ 💰 Hybrid    │               ║
║  │ Limiting     │  │ Queue        │  │ Providers    │               ║
║  └──────────────┘  └──────────────┘  └──────────────┘               ║
║                                                                      ║
║  ──────── USE CASES ────────                                         ║
║                                                                      ║
║  • Internal knowledge management                                    ║
║  • Customer support intelligence                                    ║
║  • Legal document analysis                                          ║
║  • Research paper mining                                            ║
║  • Compliance documentation                                        ║
║                                                                      ║
║  ──────── CONTACT ────────                                           ║
║                                                                      ║
║  ┌────────────────────────────────────────────────────────────┐      ║
║  │  Ready to discuss your use case?                           │      ║
║  │                                                            │      ║
║  │  Our team provides architecture consulting, custom         │      ║
║  │  integrations, and dedicated support for production        │      ║
║  │  EdgeQuake deployments.                                    │      ║
║  │                                                            │      ║
║  │  [Contact Us →]  contact@elitizon.com                      │      ║
║  └────────────────────────────────────────────────────────────┘      ║
║                                                                      ║
╠══════════════════════════════════════════════════════════════════════╣
║  FOOTER                                                             ║
╚══════════════════════════════════════════════════════════════════════╝
```

---

## 6. Contact Page (`/contact`)

### 6.1 Layout

```
╔══════════════════════════════════════════════════════════════════════╗
║  HEADER                                                             ║
╠══════════════════════════════════════════════════════════════════════╣
║                                                                      ║
║          GET IN TOUCH                                                ║
║                                                                      ║
║  ┌──────────────────────────────┬─────────────────────────────┐      ║
║  │                              │                             │      ║
║  │  Contact Information         │   Tell us about your        │      ║
║  │                              │   project                   │      ║
║  │  We respond within two       │                             │      ║
║  │  business days.              │   First Name *              │      ║
║  │                              │   [________________]        │      ║
║  │  ✉ contact@elitizon.com     │                             │      ║
║  │                              │   Last Name *               │      ║
║  │  🏢 Elitizon                 │   [________________]        │      ║
║  │     Paris, France            │                             │      ║
║  │                              │   Business Email *          │      ║
║  │  🔗 Links                    │   [________________]        │      ║
║  │  • GitHub                    │                             │      ║
║  │  • Twitter/X                 │   Company                   │      ║
║  │  • LinkedIn                  │   [________________]        │      ║
║  │                              │                             │      ║
║  │                              │   Inquiry Type *            │      ║
║  │                              │   [▼ Select...]             │      ║
║  │                              │   • Enterprise Support      │      ║
║  │                              │   • Partnership             │      ║
║  │                              │   • Consulting              │      ║
║  │                              │   • Custom Integration      │      ║
║  │                              │   • Other                   │      ║
║  │                              │                             │      ║
║  │                              │   Message *                 │      ║
║  │                              │   [                    ]    │      ║
║  │                              │   [                    ]    │      ║
║  │                              │   [                    ]    │      ║
║  │                              │                             │      ║
║  │                              │   [Submit →]                │      ║
║  │                              │                             │      ║
║  └──────────────────────────────┴─────────────────────────────┘      ║
║                                                                      ║
╠══════════════════════════════════════════════════════════════════════╣
║  FOOTER                                                             ║
╚══════════════════════════════════════════════════════════════════════╝
```

### 6.2 Form Specification

See [08-contact-lead-generation.md](./08-contact-lead-generation.md) for complete form spec.

---

## 7. Changelog Page (`/changelog`)

### 7.1 Layout

```
╔══════════════════════════════════════════════════════════════════════╗
║  HEADER                                                             ║
╠══════════════════════════════════════════════════════════════════════╣
║                                                                      ║
║                      CHANGELOG                                       ║
║                                                                      ║
║  ┌────────────────────────────────────────────────────────────┐      ║
║  │  Version 0.7.0 — March 2026                   [Current]    │      ║
║  │  ──────────────────────────────────────────                │      ║
║  │  • Vector Storage Optimization with SQL-level filtering   │      ║
║  │  • 90% fewer vector scans for multi-tenant queries        │      ║
║  │  • GIN + B-tree indexes on metadata                       │      ║
║  │  • Materialized columns for workspace/tenant IDs          │      ║
║  └────────────────────────────────────────────────────────────┘      ║
║                                                                      ║
║  ┌────────────────────────────────────────────────────────────┐      ║
║  │  Version 0.6.0 — February 2026                             │      ║
║  │  ──────────────────────────────────────────                │      ║
║  │  • Hybrid provider mode (SPEC-033)                        │      ║
║  │  • Background task queue with auto-retry                  │      ║
║  │  • ...                                                    │      ║
║  └────────────────────────────────────────────────────────────┘      ║
║                                                                      ║
║  ... (earlier versions)                                              ║
║                                                                      ║
╠══════════════════════════════════════════════════════════════════════╣
║  FOOTER                                                             ║
╚══════════════════════════════════════════════════════════════════════╝
```

### 7.2 Content Source

Generated from `CHANGELOG.md` at build time via MDX processing.

---

## 8. Responsive Breakpoints

| Breakpoint | Width | Layout Changes |
|-----------|-------|---------------|
| **Mobile** | < 768px | Single column, hamburger menu, stacked cards |
| **Tablet** | 768-1024px | 2-column grid, sidebar collapsible |
| **Desktop** | 1024-1280px | 3-column grid, sidebar visible |
| **Wide** | > 1280px | 3-column grid + right TOC in docs |

### Mobile Adaptations

| Component | Desktop | Mobile |
|-----------|---------|--------|
| Navigation | Horizontal links | Hamburger drawer |
| Hero visual | Side-by-side | Stacked (visual below text) |
| Feature grid | 3 columns | 1 column (stacked cards) |
| Docs sidebar | Always visible | Slide-out drawer |
| Architecture diagram | Full SVG | Simplified / scrollable |
| Benchmark charts | Horizontal bars | Vertical bars |
| Contact form | 2 columns | Single column |

---

## 9. Page Performance Budgets

| Page | HTML | CSS | JS | Images | Total | LCP Target |
|------|------|------|-----|--------|-------|-----------|
| Home | 15KB | 30KB | 120KB | 100KB | 265KB | < 2.5s |
| Docs | 10KB | 25KB | 80KB | 20KB | 135KB | < 1.8s |
| Demo | 12KB | 25KB | 180KB | 50KB | 267KB | < 3.0s |
| Ecosystem | 10KB | 25KB | 60KB | 40KB | 135KB | < 2.0s |
| Enterprise | 10KB | 25KB | 60KB | 30KB | 125KB | < 2.0s |
| Contact | 8KB | 25KB | 70KB | 10KB | 113KB | < 1.5s |

---

*Previous: [02-content-strategy.md](./02-content-strategy.md) · Next: [04-technical-architecture.md](./04-technical-architecture.md)*
