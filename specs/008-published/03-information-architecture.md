# SPEC-008-03: Information Architecture

| Field      | Value                        |
| ---------- | ---------------------------- |
| **Parent** | [SPEC-008](./00-overview.md) |
| **Status** | Draft                        |
| **Date**   | 2026-03-21                   |

---

## 1. Overview

This document defines the unified information architecture for `edgequake.com` — covering both the marketing pages and the published documentation. The site has two distinct zones served by one Astro project:

1. **Marketing zone** — Custom Astro pages (`/`, `/demo/`, `/ecosystem/`, `/enterprise/`, `/contact/`)
2. **Documentation zone** — Starlight-powered pages under `/docs/`

---

## 2. Unified Sitemap

```
edgequake.com
|
+-- /                          Marketing: Home/landing (hero, problem, solution, arch, benchmarks)
+-- /demo/                     Marketing: Interactive demo (6 query modes, sample queries)
+-- /ecosystem/                Marketing: 8 Rust crates + 16 MCP tools
+-- /enterprise/               Marketing: Enterprise features + pricing tiers
+-- /contact/                  Marketing: Contact form with use-case selector
+-- /404                       Marketing: Custom 404 page
|
+-- /docs/                     Docs: Landing / index page
    +-- getting-started/
    |   +-- installation/
    |   +-- quick-start/
    +-- concepts/
    |   +-- entity-extraction/
    |   +-- graph-rag/
    |   +-- hybrid-retrieval/
    |   +-- knowledge-graph/
    +-- architecture/
    |   +-- overview/
    |   +-- data-flow/
    |   +-- lineage-tracking/
    |   +-- crates/
    +-- tutorials/
    |   +-- document-ingestion/
    |   +-- first-rag-app/
    |   +-- migration/
    |   +-- pdf-processing/
    |   +-- hybrid-query/
    |   +-- multi-tenant/
    |   +-- mcp-integration/
    +-- api-reference/
    |   +-- rest-api/
    |   +-- extended-api/
    |   +-- lineage-endpoints/
    |   +-- document-upload/
    +-- deep-dives/
    |   +-- chunking/
    |   +-- embeddings/
    |   +-- entity-normalization/
    |   +-- ... (13 total)
    +-- operations/
    |   +-- configuration/
    |   +-- deployment/
    |   +-- monitoring/
    |   +-- performance/
    |   +-- metadata/
    +-- integrations/
    |   +-- custom-clients/
    |   +-- langchain/
    |   +-- open-webui/
    +-- comparisons/
    |   +-- vs-graphrag/
    |   +-- vs-lightrag/
    |   +-- vs-traditional-rag/
    |   +-- superiority/
    +-- security/
    |   +-- best-practices/
    +-- troubleshooting/
    |   +-- common-issues/
    +-- cookbook/
    +-- faq/
    +-- features/
```

**Total: 7 marketing pages + 52 documentation pages = 59 pages**

---

## 3. Navigation Structure

### 3.1 Global Header (Shared Across All Pages)

The header is consistent across marketing and docs zones:

```
+------------------------------------------------------------------+
| [Logo] EdgeQuake    Docs  Demo  Ecosystem  Enterprise  [GitHub]  |
|                                              [Theme Toggle]      |
+------------------------------------------------------------------+
```

| Nav Item     | Target                   | Zone      |
| ------------ | ------------------------ | --------- |
| Logo         | `/`                      | Marketing |
| Docs         | `/docs/`                 | Docs      |
| Demo         | `/demo/`                 | Marketing |
| Ecosystem    | `/ecosystem/`            | Marketing |
| Enterprise   | `/enterprise/`           | Marketing |
| GitHub       | `https://github.com/...` | External  |
| Theme Toggle | (client-side)            | Both      |

**Implementation:** Custom Astro component shared via layout. Starlight's default header is overridden to show the global header.

### 3.2 Documentation Sidebar (Docs Zone Only)

The sidebar appears only on `/docs/` pages. It is configured in `astro.config.mjs` using Starlight's sidebar option:

```
Sidebar
+-- Getting Started
|   +-- Installation
|   +-- Quick Start
+-- Concepts
|   +-- Entity Extraction
|   +-- Graph RAG
|   +-- Hybrid Retrieval
|   +-- Knowledge Graph
+-- Architecture
|   +-- Overview
|   +-- Data Flow
|   +-- Lineage Tracking
|   +-- Crates
+-- Tutorials
|   +-- Document Ingestion
|   +-- First RAG App
|   +-- Migration Guide
|   +-- PDF Processing
|   +-- Hybrid Query
|   +-- Multi-Tenant Setup
|   +-- MCP Integration
+-- API Reference
|   +-- REST API
|   +-- Extended API
|   +-- Lineage Endpoints
|   +-- Document Upload
+-- Deep Dives
|   +-- (13 technical articles)
+-- Operations
|   +-- Configuration
|   +-- Deployment
|   +-- Monitoring
|   +-- Performance
|   +-- Metadata
+-- Integrations
|   +-- Custom Clients
|   +-- LangChain
|   +-- Open WebUI
+-- Comparisons
|   +-- vs GraphRAG
|   +-- vs LightRAG
|   +-- vs Traditional RAG
|   +-- Superiority Analysis
+-- Security
|   +-- Best Practices
+-- Troubleshooting
|   +-- Common Issues
+-- Resources
    +-- Cookbook
    +-- FAQ
    +-- Features
```

### 3.3 Footer (Shared)

```
+------------------------------------------------------------------+
| Product        Developers       Community    Company              |
| Features       Documentation    GitHub       About Elitizon       |
| Pricing        Quick Start      Discord      Blog                 |
| Demo           API Reference    Twitter/X    Contact              |
| Enterprise     Tutorials                                          |
|                Cookbook                                             |
|                                                                   |
| (c) 2026 Elitizon. Apache 2.0 License.                          |
+------------------------------------------------------------------+
```

---

## 4. Content Taxonomy

### 4.1 Documentation Categories

| Category            | Purpose                 | Audience                           | File Count                  |
| ------------------- | ----------------------- | ---------------------------------- | --------------------------- |
| **Getting Started** | Onboarding flow         | New users                          | 2                           |
| **Concepts**        | Mental models           | All users                          | 4                           |
| **Architecture**    | System internals        | Advanced users, contributors       | 4                           |
| **Tutorials**       | Step-by-step guides     | Developers building with EdgeQuake | 7                           |
| **API Reference**   | Endpoint specifications | Integration developers             | 4                           |
| **Deep Dives**      | Technical internals     | Advanced/research users            | 13                          |
| **Operations**      | Deployment & management | DevOps, SREs                       | 5                           |
| **Integrations**    | Third-party connections | Developers                         | 3                           |
| **Comparisons**     | Competitive positioning | Evaluators, decision makers        | 4                           |
| **Security**        | Security guidance       | Security teams                     | 1                           |
| **Troubleshooting** | Problem resolution      | All users                          | 1                           |
| **Resources**       | Standalone reference    | All users                          | 3 (cookbook, FAQ, features) |

### 4.2 Marketing Pages

| Page           | Purpose                              | Audience        |
| -------------- | ------------------------------------ | --------------- |
| **Home**       | Value proposition, hero, benchmarks  | Everyone        |
| **Demo**       | Interactive query mode demonstration | Evaluators      |
| **Ecosystem**  | Crate catalog + MCP tools            | Developers      |
| **Enterprise** | Enterprise pitch + pricing           | Decision makers |
| **Contact**    | Lead capture form                    | Prospects       |

---

## 5. User Journeys

### 5.1 New User Journey

```
Home (/)  -->  Quick Start (/docs/getting-started/quick-start/)
          -->  First RAG App (/docs/tutorials/first-rag-app/)
          -->  Concepts (/docs/concepts/)
```

### 5.2 Evaluator Journey

```
Home (/)  -->  Demo (/demo/)
          -->  Comparisons (/docs/comparisons/)
          -->  Benchmarks (homepage section)
          -->  Enterprise (/enterprise/)
```

### 5.3 Developer Journey

```
Docs (/docs/)  -->  API Reference (/docs/api-reference/rest-api/)
               -->  Tutorials (/docs/tutorials/)
               -->  Deep Dives (/docs/deep-dives/)
               -->  Cookbook (/docs/cookbook/)
```

### 5.4 Ops/SRE Journey

```
Docs (/docs/)  -->  Operations (/docs/operations/)
               -->  Deployment (/docs/operations/deployment/)
               -->  Security (/docs/security/best-practices/)
               -->  Monitoring (/docs/operations/monitoring/)
```

---

## 6. Cross-Zone Navigation

Marketing pages and docs pages must link to each other naturally:

| From                        | To                                   | Mechanism         |
| --------------------------- | ------------------------------------ | ----------------- |
| Home hero CTA "Get Started" | `/docs/getting-started/quick-start/` | Button link       |
| Home "View Docs" button     | `/docs/`                             | Button link       |
| Docs sidebar                | Marketing pages                      | Global header nav |
| Enterprise pricing          | `/contact/`                          | Button link       |
| Ecosystem crate cards       | `/docs/architecture/crates/`         | Card link         |
| Docs quickstart             | `/demo/`                             | Inline link       |
| Footer "Documentation"      | `/docs/`                             | Footer link       |
| Footer "API Reference"      | `/docs/api-reference/rest-api/`      | Footer link       |

---

## 7. Search Scope

Pagefind (Starlight's built-in search) indexes all pages that Starlight renders. For the unified site:

- **Docs pages**: Fully indexed by Pagefind (automatic)
- **Marketing pages**: Optionally indexed; use `data-pagefind-body` attribute on main content to include them

See [06-search-navigation-seo.md](./06-search-navigation-seo.md) for search configuration details.

---

## 8. URL Design Principles

| Principle          | Example                                             |
| ------------------ | --------------------------------------------------- |
| Descriptive slugs  | `/docs/tutorials/first-rag-app/` not `/docs/t/001/` |
| Trailing slashes   | All URLs end with `/` for consistency               |
| Lowercase          | All path segments lowercase                         |
| Hyphens for spaces | `entity-extraction` not `entity_extraction`         |
| Category prefixes  | `/docs/concepts/` groups concept pages              |
| No file extensions | `/docs/faq/` not `/docs/faq.html`                   |

---

## 9. Cross-References

- [00-overview.md](./00-overview.md) — Goals G1 (publish all docs) and G7 (migrate all pages)
- [04-starlight-project-setup.md](./04-starlight-project-setup.md) — Sidebar configuration in astro.config.mjs
- [06-search-navigation-seo.md](./06-search-navigation-seo.md) — Pagefind scope and SEO details
- [10-content-mapping-matrix.md](./10-content-mapping-matrix.md) — Exact file-to-URL mapping
