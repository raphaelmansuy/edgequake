# Building an AI Interface That Shows Its Work

## Inside the EdgeQuake WebUI

_How we built a React 19 + Next.js 16 interface for Graph-RAG with streaming responses, knowledge graph visualization, and full processing transparency_

---

I've used a lot of AI tools. Most of them feel the same.

Type a question. Wait. Loading spinner. Wait more. Then—bam—a wall of text appears. Where did it come from? How did the AI find that information? What documents did it use? No idea.

These interfaces treat AI as a black box. Insert question, receive answer. Don't ask how.

When we built the EdgeQuake WebUI, we made a different choice: **show everything**.

---

## The Transparency Principle

EdgeQuake is a Graph-RAG system—it builds a knowledge graph from your documents, then uses that graph to answer questions. The data is inherently visual. The process is inherently observable. Why hide it?

The EdgeQuake WebUI shows:

1. **Document processing**: Watch entities being extracted in real-time
2. **Knowledge graph**: Explore the graph visually with Sigma.js
3. **Chain-of-thought**: See what the AI is thinking as it generates
4. **Source citations**: Know exactly which documents informed each answer
5. **Cost tracking**: Understand what processing costs per document

This isn't just a nice-to-have. For enterprise users, transparency is a requirement. Legal teams want to know where answers come from. Finance teams want to know what it costs. Everyone wants to trust the system.

---

## The User Journey

```
┌──────────────────────────────────────────────────────────────┐
│                    EdgeQuake WebUI Flow                       │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────┐    ┌───────────┐    ┌─────────┐    ┌─────────┐ │
│  │ Upload  │ → │ Process   │ → │ Graph   │ → │ Query   │  │
│  │ Docs    │    │ Pipeline  │    │ View    │    │ Chat    │  │
│  └─────────┘    └───────────┘    └─────────┘    └─────────┘  │
│       │              │               │              │        │
│       ↓              ↓               ↓              ↓        │
│  Drag-drop      Progress bar    Interactive    Streaming    │
│  File picker    Status badges   Sigma.js       Responses    │
│  Batch upload   Cost tracking   Entity filter  Citations    │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Step 1: Document Upload

Drag documents onto the page. The Document Manager accepts multiple files, shows batch progress, and tracks each document's status:

- **Pending**: Queued for processing
- **Processing**: Currently extracting entities
- **Completed**: Successfully added to knowledge graph
- **Failed**: Error occurred (with retry button)

Cost tracking shows exactly how much each document costs to process. When you're ingesting thousands of documents, this visibility matters.

### Step 2: Knowledge Graph Exploration

Once documents are processed, explore the knowledge graph:

- **Interactive visualization**: Zoom, pan, click nodes
- **Entity filtering**: Show only specific types (people, organizations, concepts)
- **Time filtering**: See how the graph evolved over time
- **Search**: Find specific entities by name
- **Minimap**: Navigate large graphs with 1000+ nodes
- **Layout control**: Force-directed, circular, or grid layouts

This isn't a gimmick. The graph shows you what the system "learned" from your documents. You can verify that entities were extracted correctly, spot missing relationships, and understand the structure of your knowledge base.

### Step 3: Query Interface

Ask questions in natural language. The query interface provides:

**Query Modes**:
| Mode | Icon | Description |
|------|------|-------------|
| Local | 🎯 | Search within entity neighborhoods |
| Global | 🌍 | Search the entire graph |
| Hybrid | 📚 | Combined approach (recommended) |
| Naive | ⚡ | Skip graph, direct LLM |

**Streaming Responses**:
Answers stream token-by-token. You see the AI "typing" in real-time, not waiting for a complete response.

**Chain-of-Thought Display**:
The "thinking" indicator shows what the AI is reasoning about before generating the final answer. This makes the reasoning process visible.

**Source Citations**:
Every answer includes citations to source documents. Click a citation to see the original text. Verify claims against sources.

---

## Technical Deep Dive: Streaming Markdown

Building a streaming markdown renderer is harder than it sounds.

LLM providers (OpenAI, Ollama, etc.) stream tokens one at a time. But LLM tokenizers are optimized for natural language, not markdown. They often add leading spaces to word tokens.

When tokens arrive and get concatenated, markdown syntax can break:

```
Tokens: ["The", "**", " Code2Doc", "**", " project"]
Expected: "The **Code2Doc** project"
Actual: "The** Code2Doc** project"
```

The `**` attached to "The" instead of standing alone. Now markdown parsing fails—"The\*\*" isn't a valid bold start.

Our `StreamingMarkdownRenderer` (442 lines) includes sophisticated normalization:

```tsx
/**
 * Normalize markdown for streaming.
 * Fixes issues from LLM token concatenation.
 */
function normalizeMarkdownForStreaming(content: string): string {
  let normalized = content;

  // Pattern: word** text → word **text
  // LLM tokenizers can attach ** to previous word
  normalized = normalized.replace(
    /(?<!\*\*[^*]*)([a-zA-Z0-9])\*\* (\w)/g,
    "$1 **$2",
  );

  return normalized;
}
```

But it goes deeper:

**Table Buffering**: Markdown tables must be complete before rendering. A partial table looks broken. We buffer table content until we detect the table is complete, then render it all at once.

**Code Block Handling**: Syntax highlighting requires the full code block. We detect language hints and buffer until the closing fence.

**Math Rendering**: KaTeX equations need complete expressions. Partial `$` or `$$` markers break rendering.

**Auto-Scroll Throttling**: Scrolling on every token causes jank. We throttle auto-scroll to 60fps for smooth visual updates.

This is the kind of detail work that separates polished AI interfaces from prototypes.

---

## Graph Visualization with Sigma.js

The knowledge graph viewer uses Sigma.js 3.0 with WebGL acceleration. It handles 1000+ nodes smoothly because:

1. **WebGL Rendering**: GPU-accelerated drawing, not SVG
2. **Level-of-Detail**: Hide labels at low zoom levels
3. **Culling**: Skip rendering for off-screen nodes
4. **Layout Algorithms**: Force-directed runs in web workers

Features include:

- **Entity Browser Panel**: Searchable list of all entities
- **Node Details Panel**: Click a node to see its relationships
- **Context Menus**: Right-click for actions (expand, hide, bookmark)
- **Keyboard Navigation**: Arrow keys, Enter, Escape for accessibility
- **Export**: Download graph as PNG or JSON

The minimap in the corner helps navigate large graphs—essential when you have thousands of entities.

---

## Component Architecture

The WebUI contains 100+ React components built on shadcn/ui:

| Category  | Components | Key Files                         |
| --------- | ---------- | --------------------------------- |
| Query     | 17+        | query-interface.tsx (897 lines)   |
| Graph     | 26+        | graph-viewer.tsx (785 lines)      |
| Documents | 18+        | document-manager.tsx (1492 lines) |
| UI        | 40+        | Button, Input, Select, etc.       |

Using shadcn/ui means components are copy-pasted into the codebase (not npm installed), so we can modify them freely. No "ejecting" needed, no version conflicts.

State management uses Zustand (minimal boilerplate) and data fetching uses TanStack Query (automatic caching, background refresh).

---

## Tech Stack

| Layer         | Technology                  |
| ------------- | --------------------------- |
| Framework     | Next.js 16.1.0 (App Router) |
| UI Library    | React 19.2.3                |
| Styling       | Tailwind CSS 4.1.18         |
| Components    | shadcn/ui                   |
| State         | Zustand 5.0.9               |
| Data Fetching | TanStack Query 5.90.12      |
| Graph         | Sigma.js 3.0.2 + Graphology |
| Icons         | Lucide React                |

All latest versions. React 19's concurrent features help with streaming. Next.js 16's App Router enables server components where appropriate. Tailwind 4 brings improved performance and new features.

---

## What We Learned

Building this interface taught us several lessons:

1. **Streaming is complex**: Token-by-token rendering requires careful buffering and normalization. Plan for this early.

2. **Visibility builds trust**: Users trust the system more when they can see what's happening. The graph visualization isn't just pretty—it's verification.

3. **Components compound**: Starting with shadcn/ui let us build faster. 100+ components in a few months, all consistent, all customizable.

4. **Performance is UX**: A janky graph viewer feels broken. A smooth 60fps experience feels professional. The extra optimization work pays off.

5. **Accessibility matters**: Keyboard navigation, ARIA labels, and proper focus management make the tool usable for everyone.

---

## Try It Yourself

EdgeQuake is open source. The WebUI is in the `edgequake_webui/` directory.

```bash
cd edgequake_webui
bun install
bun run dev
```

Open `http://localhost:3000` to see the interface.

**What you'll need**:

- Node.js 20+ or Bun 1.1+
- EdgeQuake API server running (port 8080)
- Some documents to upload

---

## Contribute

The best AI interfaces are built collaboratively. We're looking for contributions in:

- **New visualizations**: More graph layouts, timeline views
- **Accessibility improvements**: Screen reader support, high contrast
- **Performance optimizations**: Larger graph handling
- **Internationalization**: More languages

**Repository**: github.com/raphaelmansuy/edgequake

The interface is as much a product as the backend. We'd love your input on making it better.

---

_EdgeQuake WebUI: Because AI should show its work._
