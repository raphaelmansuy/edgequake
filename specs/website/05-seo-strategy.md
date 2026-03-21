# SPEC-WEBSITE-05: SEO Strategy

> **Status**: DRAFT  
> **Created**: 2026-03-21  
> **Parent**: [00-overview.md](./00-overview.md)  
> **Related**: [01-information-architecture.md](./01-information-architecture.md) · [02-content-strategy.md](./02-content-strategy.md) · [04-technical-architecture.md](./04-technical-architecture.md)

---

## 1. Keyword Strategy

### 1.1 Primary Keywords

<<<<<<< HEAD
| Keyword | Monthly Volume (est.) | Difficulty | Target Page |
|---------|--------------------|------------|-------------|
| graph rag | High | Medium | Homepage |
| graph rag framework | Medium | Low | Homepage |
| knowledge graph rag | Medium | Low | `/docs/concepts/graph-rag` |
| rust rag framework | Low | Very Low | Homepage |
| entity extraction llm | Medium | Medium | `/docs/concepts/entity-extraction` |
| rag query modes | Low | Very Low | `/docs/concepts/query-modes` |
| pdf to knowledge graph | Medium | Low | `/docs/guides/pdf-ingestion` |
| mcp rag integration | Low | Very Low | `/docs/guides/mcp-integration` |

### 1.2 Long-Tail Keywords

| Keyword Phrase | Target Page |
|---------------|-------------|
| "how to build knowledge graph from documents" | `/docs/getting-started/quick-start` |
| "graph rag vs traditional rag" | `/docs/comparisons/vs-traditional-rag` |
| "self-hosted rag framework" | `/enterprise` |
| "open source graph rag rust" | Homepage |
| "multi-tenant rag system" | `/enterprise` |
| "pdf entity extraction pipeline" | `/docs/guides/pdf-ingestion` |
| "lightrag alternative" | `/docs/comparisons/vs-lightrag` |
| "graphrag alternative faster" | `/docs/comparisons/vs-graphrag` |
=======
| Keyword                | Monthly Volume (est.) | Difficulty | Target Page                        |
| ---------------------- | --------------------- | ---------- | ---------------------------------- |
| graph rag              | High                  | Medium     | Homepage                           |
| graph rag framework    | Medium                | Low        | Homepage                           |
| knowledge graph rag    | Medium                | Low        | `/docs/concepts/graph-rag`         |
| rust rag framework     | Low                   | Very Low   | Homepage                           |
| entity extraction llm  | Medium                | Medium     | `/docs/concepts/entity-extraction` |
| rag query modes        | Low                   | Very Low   | `/docs/concepts/query-modes`       |
| pdf to knowledge graph | Medium                | Low        | `/docs/guides/pdf-ingestion`       |
| mcp rag integration    | Low                   | Very Low   | `/docs/guides/mcp-integration`     |

### 1.2 Long-Tail Keywords

| Keyword Phrase                                | Target Page                            |
| --------------------------------------------- | -------------------------------------- |
| "how to build knowledge graph from documents" | `/docs/getting-started/quick-start`    |
| "graph rag vs traditional rag"                | `/docs/comparisons/vs-traditional-rag` |
| "self-hosted rag framework"                   | `/enterprise`                          |
| "open source graph rag rust"                  | Homepage                               |
| "multi-tenant rag system"                     | `/enterprise`                          |
| "pdf entity extraction pipeline"              | `/docs/guides/pdf-ingestion`           |
| "lightrag alternative"                        | `/docs/comparisons/vs-lightrag`        |
| "graphrag alternative faster"                 | `/docs/comparisons/vs-graphrag`        |
>>>>>>> origin/edgequake-main

### 1.3 Branded Keywords

- edgequake
- edgequake rag
- edgequake graph rag
- elitizon rag

---

## 2. Meta Tags Per Page

### 2.1 Homepage (`/`)

```html
<<<<<<< HEAD
<title>EdgeQuake — Graph-RAG Built for Speed | Rust Knowledge Graph Framework</title>
<meta name="description" content="Turn documents into queryable knowledge graphs. 10x faster than Python RAG frameworks. 6 query modes. Open source Apache 2.0." />
<meta name="keywords" content="graph rag, knowledge graph, entity extraction, rust, open source, rag framework" />
=======
<title>
  EdgeQuake — Graph-RAG Built for Speed | Rust Knowledge Graph Framework
</title>
<meta
  name="description"
  content="Turn documents into queryable knowledge graphs. 10x faster than Python RAG frameworks. 6 query modes. Open source Apache 2.0."
/>
<meta
  name="keywords"
  content="graph rag, knowledge graph, entity extraction, rust, open source, rag framework"
/>
>>>>>>> origin/edgequake-main
<link rel="canonical" href="https://edgequake.dev/" />
```

### 2.2 Documentation Hub (`/docs`)

```html
<title>Documentation — EdgeQuake</title>
<<<<<<< HEAD
<meta name="description" content="Complete guide to EdgeQuake: installation, concepts, architecture, API reference, and deployment." />
=======
<meta
  name="description"
  content="Complete guide to EdgeQuake: installation, concepts, architecture, API reference, and deployment."
/>
>>>>>>> origin/edgequake-main
<link rel="canonical" href="https://edgequake.dev/docs/" />
```

### 2.3 Demo (`/demo`)

```html
<title>Interactive Demo — EdgeQuake</title>
<<<<<<< HEAD
<meta name="description" content="See EdgeQuake in action. Explore 6 query modes, knowledge graph visualization, and entity extraction results." />
=======
<meta
  name="description"
  content="See EdgeQuake in action. Explore 6 query modes, knowledge graph visualization, and entity extraction results."
/>
>>>>>>> origin/edgequake-main
<link rel="canonical" href="https://edgequake.dev/demo/" />
```

### 2.4 Ecosystem (`/ecosystem`)

```html
<title>Ecosystem — 10 Modular Crates | EdgeQuake</title>
<<<<<<< HEAD
<meta name="description" content="Explore EdgeQuake's modular Rust crate ecosystem: core, API, storage, pipeline, query engine, LLM providers, and more." />
=======
<meta
  name="description"
  content="Explore EdgeQuake's modular Rust crate ecosystem: core, API, storage, pipeline, query engine, LLM providers, and more."
/>
>>>>>>> origin/edgequake-main
<link rel="canonical" href="https://edgequake.dev/ecosystem/" />
```

### 2.5 Enterprise (`/enterprise`)

```html
<title>Enterprise — Production-Grade Graph-RAG | EdgeQuake</title>
<<<<<<< HEAD
<meta name="description" content="Get dedicated support, architecture consulting, and custom integrations for production EdgeQuake deployments." />
=======
<meta
  name="description"
  content="Get dedicated support, architecture consulting, and custom integrations for production EdgeQuake deployments."
/>
>>>>>>> origin/edgequake-main
<link rel="canonical" href="https://edgequake.dev/enterprise/" />
```

### 2.6 Contact (`/contact`)

```html
<title>Contact — EdgeQuake Enterprise</title>
<<<<<<< HEAD
<meta name="description" content="Contact the EdgeQuake team for enterprise support, partnerships, and consulting. contact@elitizon.com" />
=======
<meta
  name="description"
  content="Contact the EdgeQuake team for enterprise support, partnerships, and consulting. contact@elitizon.com"
/>
>>>>>>> origin/edgequake-main
<link rel="canonical" href="https://edgequake.dev/contact/" />
```

### 2.7 Individual Doc Page (template)

```html
<title>{page.title} — EdgeQuake Docs</title>
<meta name="description" content="{page.description}" />
<link rel="canonical" href="https://edgequake.dev/docs/{page.slug}/" />
```

---

## 3. OpenGraph Tags

### 3.1 Default (all pages)

```html
<meta property="og:site_name" content="EdgeQuake" />
<meta property="og:type" content="website" />
<meta property="og:locale" content="en_US" />
```

### 3.2 Homepage

```html
<meta property="og:title" content="EdgeQuake — Graph-RAG Built for Speed" />
<<<<<<< HEAD
<meta property="og:description" content="Turn documents into queryable knowledge graphs. 10x faster than Python RAG. Open source." />
=======
<meta
  property="og:description"
  content="Turn documents into queryable knowledge graphs. 10x faster than Python RAG. Open source."
/>
>>>>>>> origin/edgequake-main
<meta property="og:image" content="https://edgequake.dev/og/home.png" />
<meta property="og:image:width" content="1200" />
<meta property="og:image:height" content="630" />
<meta property="og:url" content="https://edgequake.dev/" />
```

### 3.3 OG Image Spec

<<<<<<< HEAD
| Image | Dimensions | Content |
|-------|-----------|---------|
| `og/home.png` | 1200x630 | Logo + tagline + graph background |
| `og/docs.png` | 1200x630 | Logo + "Documentation" + code snippet |
| `og/demo.png` | 1200x630 | Logo + "Interactive Demo" + graph screenshot |
| `og/enterprise.png` | 1200x630 | Logo + "Enterprise" + professional gradient |
| `og/docs/[slug].png` | 1200x630 | Logo + page title (auto-generated at build time) |
=======
| Image                | Dimensions | Content                                          |
| -------------------- | ---------- | ------------------------------------------------ |
| `og/home.png`        | 1200x630   | Logo + tagline + graph background                |
| `og/docs.png`        | 1200x630   | Logo + "Documentation" + code snippet            |
| `og/demo.png`        | 1200x630   | Logo + "Interactive Demo" + graph screenshot     |
| `og/enterprise.png`  | 1200x630   | Logo + "Enterprise" + professional gradient      |
| `og/docs/[slug].png` | 1200x630   | Logo + page title (auto-generated at build time) |
>>>>>>> origin/edgequake-main

### 3.4 OG Image Template Design

```
┌──────────────────────────────────────────────────────┐
│  ┌────────────────────────────────────────────────┐  │
│  │                                                │  │
│  │    🔷 EdgeQuake                                │  │
│  │                                                │  │
│  │    Graph-RAG Built for Speed                   │  │
│  │                                                │  │
│  │    Turn documents into queryable               │  │
│  │    knowledge graphs. 10x faster.               │  │
│  │                                                │  │
│  │                    edgequake.dev                │  │
│  │                                                │  │
│  └────────────────────────────────────────────────┘  │
│  Background: subtle graph/network pattern            │
└──────────────────────────────────────────────────────┘
  1200px × 630px
```

---

## 4. Twitter Card Tags

```html
<meta name="twitter:card" content="summary_large_image" />
<meta name="twitter:title" content="{page.title}" />
<meta name="twitter:description" content="{page.description}" />
<meta name="twitter:image" content="https://edgequake.dev/og/{page.ogImage}" />
```

---

## 5. JSON-LD Structured Data

### 5.1 Organization (all pages)

```json
{
  "@context": "https://schema.org",
  "@type": "Organization",
  "name": "EdgeQuake",
  "url": "https://edgequake.dev",
  "logo": "https://edgequake.dev/logos/edgequake.svg",
  "contactPoint": {
    "@type": "ContactPoint",
    "email": "contact@elitizon.com",
    "contactType": "sales"
  },
<<<<<<< HEAD
  "sameAs": [
    "https://github.com/raphaelmansuy/edgequake"
  ]
=======
  "sameAs": ["https://github.com/raphaelmansuy/edgequake"]
>>>>>>> origin/edgequake-main
}
```

### 5.2 SoftwareApplication (homepage)

```json
{
  "@context": "https://schema.org",
  "@type": "SoftwareApplication",
  "name": "EdgeQuake",
  "applicationCategory": "DeveloperApplication",
  "operatingSystem": "Linux, macOS, Windows",
  "offers": {
    "@type": "Offer",
    "price": "0",
    "priceCurrency": "USD"
  },
  "description": "Graph-RAG framework built in Rust for fast document-to-knowledge-graph processing.",
  "softwareVersion": "0.7.0",
  "license": "https://opensource.org/licenses/Apache-2.0",
  "url": "https://edgequake.dev",
  "downloadUrl": "https://github.com/raphaelmansuy/edgequake",
  "programmingLanguage": "Rust"
}
```

### 5.3 BreadcrumbList (all pages with breadcrumbs)

```json
{
  "@context": "https://schema.org",
  "@type": "BreadcrumbList",
  "itemListElement": [
    {
      "@type": "ListItem",
      "position": 1,
      "name": "Home",
      "item": "https://edgequake.dev/"
    },
    {
      "@type": "ListItem",
      "position": 2,
      "name": "Docs",
      "item": "https://edgequake.dev/docs/"
    },
    {
      "@type": "ListItem",
      "position": 3,
      "name": "Entity Extraction",
      "item": "https://edgequake.dev/docs/concepts/entity-extraction/"
    }
  ]
}
```

### 5.4 FAQPage (homepage or dedicated FAQ)

```json
{
  "@context": "https://schema.org",
  "@type": "FAQPage",
  "mainEntity": [
    {
      "@type": "Question",
      "name": "What is EdgeQuake?",
      "acceptedAnswer": {
        "@type": "Answer",
        "text": "EdgeQuake is an open-source Graph-RAG framework built in Rust that converts documents into queryable knowledge graphs with 6 retrieval modes."
      }
    },
    {
      "@type": "Question",
      "name": "How is EdgeQuake different from traditional RAG?",
      "acceptedAnswer": {
        "@type": "Answer",
        "text": "Traditional RAG uses flat vector similarity. EdgeQuake builds a knowledge graph with entities and relationships, enabling multi-hop reasoning and 6 complementary query modes."
      }
    },
    {
      "@type": "Question",
      "name": "Is EdgeQuake free?",
      "acceptedAnswer": {
        "@type": "Answer",
        "text": "Yes. EdgeQuake is 100% open source under the Apache 2.0 license. Enterprise support is available from Elitizon."
      }
    }
  ]
}
```

---

## 6. Technical SEO

### 6.1 `robots.txt`

```
User-agent: *
Allow: /
Sitemap: https://edgequake.dev/sitemap.xml

Disallow: /_next/
Disallow: /api/
```

### 6.2 Sitemap Generation

Generated at build time using `next-sitemap` or a custom script. Each page includes:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url>
    <loc>https://edgequake.dev/</loc>
    <lastmod>2026-03-21</lastmod>
    <changefreq>weekly</changefreq>
    <priority>1.0</priority>
  </url>
  <url>
    <loc>https://edgequake.dev/docs/</loc>
    <lastmod>2026-03-21</lastmod>
    <changefreq>weekly</changefreq>
    <priority>0.9</priority>
  </url>
  <!-- ... all pages ... -->
</urlset>
```

### 6.3 Page Speed Checklist

<<<<<<< HEAD
| Check | Target | Method |
|-------|--------|--------|
| Largest Contentful Paint | < 2.5s | Static HTML, minimal JS |
| First Input Delay | < 100ms | Deferred hydration |
| Cumulative Layout Shift | < 0.1 | Explicit image dimensions, font-display |
| Time to First Byte | < 200ms | GitHub Pages CDN |
| Total Blocking Time | < 200ms | Code splitting, tree shaking |

### 6.4 Accessibility (SEO Signal)

| Requirement | Implementation |
|-------------|---------------|
| Semantic HTML | `<main>`, `<nav>`, `<article>`, `<section>`, `<header>`, `<footer>` |
| Alt text | All images have descriptive `alt` attributes |
| ARIA labels | Navigation landmarks, form inputs |
| Heading hierarchy | Single `<h1>` per page, sequential `<h2>`–`<h6>` |
| Focus management | Visible focus rings, skip-to-content link |
| Color contrast | WCAG AA (4.5:1 for text, 3:1 for large text) |

### 6.5 Internal Linking Strategy

| Rule | Description |
|------|------------|
| IL-01 | Every doc page links to ≥ 2 related doc pages |
| IL-02 | Homepage links to all top-level sections |
| IL-03 | Every comparison page links back to feature it discusses |
| IL-04 | Ecosystem page links to docs for each crate |
| IL-05 | All CTAs point to relevant landing pages |
=======
| Check                    | Target  | Method                                  |
| ------------------------ | ------- | --------------------------------------- |
| Largest Contentful Paint | < 2.5s  | Static HTML, minimal JS                 |
| First Input Delay        | < 100ms | Deferred hydration                      |
| Cumulative Layout Shift  | < 0.1   | Explicit image dimensions, font-display |
| Time to First Byte       | < 200ms | GitHub Pages CDN                        |
| Total Blocking Time      | < 200ms | Code splitting, tree shaking            |

### 6.4 Accessibility (SEO Signal)

| Requirement       | Implementation                                                      |
| ----------------- | ------------------------------------------------------------------- |
| Semantic HTML     | `<main>`, `<nav>`, `<article>`, `<section>`, `<header>`, `<footer>` |
| Alt text          | All images have descriptive `alt` attributes                        |
| ARIA labels       | Navigation landmarks, form inputs                                   |
| Heading hierarchy | Single `<h1>` per page, sequential `<h2>`–`<h6>`                    |
| Focus management  | Visible focus rings, skip-to-content link                           |
| Color contrast    | WCAG AA (4.5:1 for text, 3:1 for large text)                        |

### 6.5 Internal Linking Strategy

| Rule  | Description                                              |
| ----- | -------------------------------------------------------- |
| IL-01 | Every doc page links to ≥ 2 related doc pages            |
| IL-02 | Homepage links to all top-level sections                 |
| IL-03 | Every comparison page links back to feature it discusses |
| IL-04 | Ecosystem page links to docs for each crate              |
| IL-05 | All CTAs point to relevant landing pages                 |
>>>>>>> origin/edgequake-main

---

## 7. Analytics Implementation

### 7.1 Plausible (recommended)

```html
<!-- Added to root layout <head> -->
<script
  defer
  data-domain="edgequake.dev"
  src="https://plausible.io/js/script.js"
></script>
```

No cookies. GDPR-compliant. No consent banner needed.

### 7.2 Events to Track

<<<<<<< HEAD
| Event | Trigger | Purpose |
|-------|---------|---------|
| `pageview` | Auto | Traffic analysis |
| `cta_click` | "Get Started" / "Contact Us" buttons | Conversion tracking |
| `demo_interact` | Query mode change in demo | Engagement |
| `docs_search` | Pagefind search query | Content gaps |
| `github_click` | GitHub link click | Community funnel |
| `copy_code` | Code block copy button | Developer engagement |

---

*Previous: [04-technical-architecture.md](./04-technical-architecture.md) · Next: [06-design-system.md](./06-design-system.md)*
=======
| Event           | Trigger                              | Purpose              |
| --------------- | ------------------------------------ | -------------------- |
| `pageview`      | Auto                                 | Traffic analysis     |
| `cta_click`     | "Get Started" / "Contact Us" buttons | Conversion tracking  |
| `demo_interact` | Query mode change in demo            | Engagement           |
| `docs_search`   | Pagefind search query                | Content gaps         |
| `github_click`  | GitHub link click                    | Community funnel     |
| `copy_code`     | Code block copy button               | Developer engagement |

---

_Previous: [04-technical-architecture.md](./04-technical-architecture.md) · Next: [06-design-system.md](./06-design-system.md)_
>>>>>>> origin/edgequake-main
