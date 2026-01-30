# Show HN: EdgeQuake WebUI – React 19 + Next.js 16 interface for Graph-RAG

**What it is**: A web interface for exploring knowledge graphs and querying documents using Graph-RAG. Built with the latest React 19 and Next.js 16.

**Repo**: github.com/raphaelmansuy/edgequake

---

## Why we built this

Most AI chat interfaces are black boxes. You ask a question, get an answer, and have no idea where it came from. For many use cases (legal, healthcare, enterprise), this isn't acceptable.

EdgeQuake is a Graph-RAG system—it builds a knowledge graph from documents, then uses graph traversal + vector search to answer questions. The data is inherently visual. The process is inherently observable. We wanted an interface that reflected that.

## Tech stack

| Technology     | Version | Why                                 |
| -------------- | ------- | ----------------------------------- |
| React          | 19.2.3  | Concurrent features for streaming   |
| Next.js        | 16.1.0  | App Router, server components       |
| Sigma.js       | 3.0.2   | WebGL-accelerated graph rendering   |
| shadcn/ui      | Latest  | Copy-paste components, full control |
| Zustand        | 5.0.9   | Minimal state management            |
| TanStack Query | 5.90.12 | Caching, background refresh         |

## Interesting technical challenge: Streaming markdown

LLM providers stream tokens one at a time. LLM tokenizers are optimized for natural language, not markdown. They often add leading spaces to word tokens.

When tokens get concatenated, markdown syntax breaks:

```
Tokens: ["The", "**", " Code2Doc", "**"]
Result: "The** Code2Doc**"
Expected: "The **Code2Doc**"
```

Our `StreamingMarkdownRenderer` (442 lines) includes:

- Token normalization via regex
- Table buffering (don't render partial tables)
- Code block detection and buffering
- 60fps auto-scroll throttling

This is the kind of detail that separates polished interfaces from prototypes.

## Performance considerations

The graph viewer uses Sigma.js 3.0 with WebGL. Handles 1000+ nodes because:

- GPU-accelerated rendering (not DOM-based SVG)
- Level-of-detail: hide labels at low zoom
- Culling: skip off-screen nodes
- Web worker layouts: force-directed doesn't block main thread

## Component stats

- 100+ components total
- QueryInterface: 897 lines
- GraphViewer: 785 lines
- DocumentManager: 1492 lines
- StreamingMarkdownRenderer: 442 lines

## Features

- **Document processing visibility**: Status per document, cost tracking, retry failed
- **Knowledge graph visualization**: Interactive Sigma.js, filter by type/time, minimap
- **Query modes**: Local, Global, Hybrid, Naive (4 retrieval strategies)
- **Streaming responses**: Token-by-token with chain-of-thought display
- **Source citations**: Click to see original text

## What we're looking for

Feedback on:

- UX patterns for AI interfaces
- Performance with larger graphs (10,000+ nodes)
- Accessibility improvements
- Any bugs you find

Happy to answer questions about the implementation.
