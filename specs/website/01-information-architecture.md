# SPEC-WEBSITE-01: Information Architecture

> **Status**: DRAFT  
> **Created**: 2026-03-21  
> **Parent**: [00-overview.md](./00-overview.md)  
> **Related**: [03-page-specifications.md](./03-page-specifications.md) · [05-seo-strategy.md](./05-seo-strategy.md)

---

## 1. Site Map

```
edgequake.dev
│
├── / ................................ HOME (landing page)
│   ├── #hero ........................ Hero banner + CTA
│   ├── #problem ..................... "RAG is Broken" problem section
│   ├── #solution .................... EdgeQuake approach (6 features)
│   ├── #architecture ................ Architecture diagram (interactive)
│   ├── #benchmarks .................. Performance comparison charts
│   ├── #quickstart .................. Code snippets (Rust / Docker / API)
│   ├── #ecosystem ................... Crates & integrations overview
│   ├── #enterprise .................. Enterprise CTA banner
│   └── #cta ......................... Final call-to-action
│
├── /docs ............................ DOCUMENTATION HUB
│   ├── /docs/getting-started ........ Installation & quickstart
│   │   ├── /docs/getting-started/installation
│   │   ├── /docs/getting-started/quick-start
│   │   └── /docs/getting-started/first-ingestion
│   ├── /docs/concepts ............... Core concepts
│   │   ├── /docs/concepts/graph-rag
│   │   ├── /docs/concepts/entity-extraction
│   │   ├── /docs/concepts/knowledge-graph
│   │   ├── /docs/concepts/query-modes
│   │   └── /docs/concepts/hybrid-retrieval
│   ├── /docs/architecture ........... System architecture
│   │   ├── /docs/architecture/overview
│   │   ├── /docs/architecture/data-flow
│   │   └── /docs/architecture/crates/[crate]
│   ├── /docs/guides ................. How-to guides
│   │   ├── /docs/guides/pdf-ingestion
│   │   ├── /docs/guides/multi-tenant
│   │   ├── /docs/guides/mcp-integration
│   │   └── /docs/guides/custom-providers
│   ├── /docs/api-reference .......... REST API reference
│   │   ├── /docs/api-reference/documents
│   │   ├── /docs/api-reference/query
│   │   ├── /docs/api-reference/workspaces
│   │   └── /docs/api-reference/graph
│   ├── /docs/deployment ............. Production deployment
│   │   ├── /docs/deployment/docker
│   │   ├── /docs/deployment/kubernetes
│   │   └── /docs/deployment/configuration
│   └── /docs/comparisons ............ vs. other tools
│       ├── /docs/comparisons/vs-traditional-rag
│       ├── /docs/comparisons/vs-graphrag
│       └── /docs/comparisons/vs-lightrag-python
│
├── /demo ............................ INTERACTIVE DEMO
│   ├── Graph explorer preview
│   ├── Query mode comparison
│   └── Sample knowledge graph
│
├── /ecosystem ....................... CRATES & INTEGRATIONS
│   ├── Crate cards (10 crates)
│   ├── MCP tools listing
│   └── SDK / client libraries
│
├── /enterprise ...................... ENTERPRISE PAGE
│   ├── Features for enterprise
│   ├── Support tiers
│   ├── Case studies (placeholder)
│   └── Contact CTA
│
├── /contact ......................... CONTACT FORM
│   ├── Lead capture form
│   ├── Use case selector
│   └── Confirmation page
│
├── /blog ............................ ARTICLES (v2)
│   └── (deferred — link to existing articles/)
│
└── /changelog ....................... RELEASE NOTES
    └── Version history from CHANGELOG.md
```

---

## 2. Navigation Architecture

### 2.1 Primary Navigation (Header)

```
┌──────────────────────────────────────────────────────────────────┐
│  🔷 EdgeQuake          Docs  Demo  Ecosystem  Enterprise        │
│                                                                  │
│                                    [GitHub ★]  [Get Started →]   │
└──────────────────────────────────────────────────────────────────┘
```

| Position | Item | Type | Target |
|----------|------|------|--------|
| Left | Logo + "EdgeQuake" | Brand | `/` |
| Center | Docs | Link | `/docs` |
| Center | Demo | Link | `/demo` |
| Center | Ecosystem | Link | `/ecosystem` |
| Center | Enterprise | Link | `/enterprise` |
| Right | GitHub ★ | External + badge | `github.com/raphaelmansuy/edgequake` |
| Right | "Get Started →" | CTA Button | `/docs/getting-started` |

### 2.2 Mobile Navigation

```
┌─────────────────────────────────┐
│  🔷 EdgeQuake            [≡]   │
└─────────────────────────────────┘
         │
         ▼ (slide-out drawer)
┌─────────────────────────────────┐
│  Docs                        →  │
│  Demo                        →  │
│  Ecosystem                   →  │
│  Enterprise                  →  │
│  ────────────────────────────   │
│  GitHub                      ↗  │
│  Contact                     →  │
│  ────────────────────────────   │
│  [Get Started →]                │
│                                 │
│  ☀️/🌙 Theme Toggle             │
└─────────────────────────────────┘
```

### 2.3 Documentation Sidebar

```
┌──────────────────────────────┐
│ 📖 Documentation             │
│                              │
│ ▼ Getting Started            │
│   ├─ Installation            │
│   ├─ Quick Start             │
│   └─ First Ingestion         │
│                              │
│ ▼ Core Concepts              │
│   ├─ What is Graph-RAG?      │
│   ├─ Entity Extraction       │
│   ├─ Knowledge Graphs        │
│   ├─ Query Modes             │
│   └─ Hybrid Retrieval        │
│                              │
│ ▼ Architecture               │
│   ├─ System Overview         │
│   ├─ Data Flow               │
│   └─ Crate Reference         │
│       ├─ edgequake-core      │
│       ├─ edgequake-api       │
│       ├─ edgequake-storage   │
│       ├─ edgequake-pipeline  │
│       ├─ edgequake-query     │
│       ├─ edgequake-llm       │
│       ├─ edgequake-pdf2md    │
│       ├─ edgequake-auth      │
│       ├─ edgequake-audit     │
│       └─ edgequake-tasks     │
│                              │
│ ▼ Guides                     │
│   ├─ PDF Ingestion           │
│   ├─ Multi-Tenant Setup      │
│   ├─ MCP Integration         │
│   └─ Custom LLM Providers    │
│                              │
│ ▼ API Reference              │
│   ├─ Documents API           │
│   ├─ Query API               │
│   ├─ Workspaces API          │
│   └─ Graph API               │
│                              │
│ ▼ Deployment                 │
│   ├─ Docker                  │
│   ├─ Kubernetes              │
│   └─ Configuration           │
│                              │
│ ▼ Comparisons                │
│   ├─ vs Traditional RAG      │
│   ├─ vs Microsoft GraphRAG   │
│   └─ vs LightRAG (Python)    │
└──────────────────────────────┘
```

### 2.4 Footer

```
┌───────────────────────────────────────────────────────────────────────┐
│                                                                       │
│  🔷 EdgeQuake                                                         │
│  High-performance Graph-RAG                                           │
│  framework built in Rust.                                             │
│                                                                       │
│  Product          Developers       Company        Community           │
│  ─────────        ──────────       ───────        ─────────           │
│  Features         Docs             About          GitHub              │
│  Ecosystem        API Reference    Enterprise     Discussions         │
│  Demo             Quick Start      Contact        Contributing        │
│  Changelog        Guides           Blog           Twitter/X           │
│                                                                       │
│  ─────────────────────────────────────────────────────────────────    │
│  Built with ❤️ by Elitizon · contact@elitizon.com                     │
│  © 2026 EdgeQuake · Apache 2.0 License                                │
│                                                                       │
└───────────────────────────────────────────────────────────────────────┘
```

---

## 3. User Journey Flows

### 3.1 Developer Journey ("Alex the AI Engineer")

```
Google Search                Home /                    Docs
"graph rag rust"      ┌─────────────────┐     ┌──────────────┐
─────────────────────►│ Hero: "10x      │────►│ Quick Start  │
                      │ faster RAG"     │     │ Installation │
                      │                 │     │ First Query  │
                      │ [Get Started →] │     └──────┬───────┘
                      └─────────────────┘            │
                                                     ▼
                                              ┌──────────────┐
                                              │ API Reference│
                                              │ Code Examples│
                                              │ cargo add    │
                                              └──────┬───────┘
                                                     │
                                                     ▼
                                              ┌──────────────┐
                                              │ GitHub Repo  │
                                              │ ★ Star       │
                                              │ 🍴 Fork      │
                                              └──────────────┘
```

### 3.2 Decision Maker Journey ("Sarah the CTO")

```
LinkedIn Article         Home /                Enterprise
or Referral       ┌─────────────────┐     ┌──────────────────┐
─────────────────►│ Hero: "10x      │────►│ Production       │
                  │ faster RAG"     │     │ Features         │
                  │                 │     │ Multi-tenant     │
                  │ Benchmarks ▼   │     │ Rate Limiting    │
                  │ Architecture ▼ │     │ Audit Logging    │
                  └────────┬────────┘     └──────┬───────────┘
                           │                     │
                           ▼                     ▼
                    ┌──────────────┐     ┌──────────────────┐
                    │ Comparisons  │     │ Contact Form     │
                    │ vs GraphRAG  │     │ "Enterprise      │
                    │ vs LightRAG  │     │  Inquiry"        │
                    └──────────────┘     │ → contact@       │
                                        │   elitizon.com   │
                                        └──────────────────┘
```

### 3.3 Contributor Journey ("Marcus")

```
GitHub Trending        Home /                 Docs
or HN/Reddit    ┌─────────────────┐     ┌──────────────┐
────────────────►│ Hero            │────►│ Architecture │
                │ Ecosystem ▼     │     │ Crate Docs   │
                │                 │     │ Contributing │
                └────────┬────────┘     └──────┬───────┘
                         │                     │
                         ▼                     ▼
                  ┌──────────────┐     ┌──────────────┐
                  │ Ecosystem    │     │ GitHub       │
                  │ All 10 Crates│     │ Issues       │
                  │ Contribution │     │ Pull Requests│
                  │ Opportunities│     └──────────────┘
                  └──────────────┘
```

---

## 4. URL Strategy

### 4.1 URL Patterns

| Pattern | Example | Purpose |
|---------|---------|---------|
| `/` | `edgequake.dev/` | Landing page |
| `/docs/[section]` | `/docs/getting-started` | Documentation sections |
| `/docs/[section]/[page]` | `/docs/concepts/graph-rag` | Documentation pages |
| `/docs/architecture/crates/[crate]` | `/docs/architecture/crates/edgequake-core` | Crate-specific docs |
| `/demo` | `/demo` | Interactive demo |
| `/ecosystem` | `/ecosystem` | Crates & integrations |
| `/enterprise` | `/enterprise` | Enterprise features |
| `/contact` | `/contact` | Contact form |
| `/changelog` | `/changelog` | Release notes |

### 4.2 URL Rules

1. **Lowercase only**: `/docs/getting-started` not `/docs/Getting-Started`
2. **Hyphens for spaces**: `/docs/graph-rag` not `/docs/graph_rag`
3. **No trailing slashes in content**: managed by Next.js `trailingSlash` config
4. **Canonical URLs**: every page has a canonical `<link>` tag
5. **No file extensions**: no `.html` in URLs (Next.js handles this)

---

## 5. Content Hierarchy (Priority)

```
PRIORITY 1 (Must-Have for Launch)
─────────────────────────────────
├── Home page (complete)
├── /docs/getting-started (3 pages)
├── /docs/concepts (5 pages)
├── /contact (lead form)
└── /enterprise (CTA page)

PRIORITY 2 (Week 2)
────────────────────
├── /docs/architecture (3 pages + 10 crate pages)
├── /docs/api-reference (4 pages)
├── /ecosystem (crate showcase)
└── /changelog

PRIORITY 3 (Week 3-4)
──────────────────────
├── /docs/guides (4 pages)
├── /docs/deployment (3 pages)
├── /docs/comparisons (3 pages)
├── /demo (interactive)
└── Search functionality
```

---

## 6. Breadcrumb Strategy

Every page outside the home displays breadcrumbs:

```
Home > Docs > Concepts > Entity Extraction
Home > Docs > Architecture > Crates > edgequake-core
Home > Enterprise
Home > Contact
```

Schema.org `BreadcrumbList` JSON-LD on every page (see [05-seo-strategy.md](./05-seo-strategy.md)).

---

## 7. Cross-Linking Strategy

### Internal Linking Rules

1. **Every docs page** links to ≥ 2 related concept pages
2. **Every concept page** links to the relevant crate in ecosystem
3. **Every crate page** links to its API reference
4. **Architecture pages** link to deployment guides
5. **Comparison pages** link back to the matching EdgeQuake feature
6. **Contact CTA** appears on: Homepage, Enterprise, every docs page (footer)

### External Links (Open in New Tab)

| Link Target | Appears On |
|------------|------------|
| GitHub repo | Header, Footer, Hero |
| crates.io (edgequake-llm) | Ecosystem page |
| npm (MCP server) | Ecosystem page |
| Elitizon website | Footer |
| Apache 2.0 License | Footer |

---

*Previous: [00-overview.md](./00-overview.md) · Next: [02-content-strategy.md](./02-content-strategy.md)*
