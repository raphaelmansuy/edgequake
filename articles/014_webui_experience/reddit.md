# I Built a Streaming AI Interface with React 19 and Next.js 16. Here's What I Learned.

**TL;DR**: Streaming markdown from LLMs is harder than it looks. Token normalization, table buffering, and 60fps scroll throttling are all required for a polished experience.

---

## Context

I'm building EdgeQuake, a Graph-RAG system that extracts knowledge graphs from documents. I needed a web interface that:

1. Shows the knowledge graph visually
2. Streams responses token-by-token
3. Displays chain-of-thought reasoning
4. Tracks document processing with real-time status

This is what I built and what I learned.

## Tech Stack

| Layer      | Choice                 | Why                               |
| ---------- | ---------------------- | --------------------------------- |
| Framework  | Next.js 16.1.0         | App Router, server components     |
| UI         | React 19.2.3           | Concurrent features for streaming |
| Components | shadcn/ui              | Copy-paste, full control          |
| State      | Zustand 5.0.9          | Minimal boilerplate               |
| Data       | TanStack Query 5.90.12 | Caching, background refresh       |
| Graphs     | Sigma.js 3.0.2         | WebGL, handles 1000+ nodes        |

## Lesson 1: Streaming Markdown is Complex

LLM tokenizers are optimized for natural language, not markdown. They add leading spaces to word tokens, which breaks markdown syntax when tokens concatenate:

```
Tokens: ["The", "**", " Code2Doc", "**"]
Result: "The** Code2Doc**"  // broken
Expected: "The **Code2Doc**"
```

**Solution**: Real-time normalization.

```tsx
// Pattern: word** text → word **text
normalized = normalized.replace(
  /(?<!\*\*[^*]*)([a-zA-Z0-9])\*\* (\w)/g,
  "$1 **$2",
);
```

I ended up with 442 lines of streaming markdown logic. Not what I expected when I started.

## Lesson 2: Buffer Tables and Code Blocks

Partial markdown tables look broken:

```
| Column 1 |
|-------
```

That's not a valid table. Users see broken formatting.

**Solution**: Detect table starts, buffer until complete, then render.

Same for code blocks—you need the closing fence before syntax highlighting. Same for math—KaTeX needs complete expressions.

## Lesson 3: 60fps Auto-Scroll

Naive implementation: Scroll on every token.

Result: Jank. Stuttering. Feels broken.

**Solution**: Throttle scroll to 60fps using requestAnimationFrame. The visual difference is significant.

## Lesson 4: Sigma.js for Large Graphs

I tried several graph libraries. Sigma.js 3.0 won because:

- **WebGL**: GPU-accelerated, handles 1000+ nodes
- **Level-of-detail**: Hide labels at low zoom
- **Culling**: Skip off-screen nodes
- **Web workers**: Layout algorithms don't block main thread

The minimap is essential for large graphs. Users get lost without it.

## Lesson 5: shadcn/ui Accelerates

Instead of npm installing a component library, shadcn/ui lets you copy components into your codebase. Sounds weird, works great.

Benefits:

- Full control over every component
- No version conflicts
- Customize freely
- 100+ components in my repo, all consistent

## Component Stats

| Component                 | Lines |
| ------------------------- | ----- |
| QueryInterface            | 897   |
| GraphViewer               | 785   |
| DocumentManager           | 1492  |
| StreamingMarkdownRenderer | 442   |

## What I'm Using React 19 For

- **Concurrent rendering**: Streaming responses don't block
- **Transitions**: UI stays responsive during updates
- **Suspense**: Lazy-load heavy components (graph, math renderer)

The concurrent features matter most for streaming—you don't want one slow token to block the UI.

## Open Source

The whole thing is at: github.com/raphaelmansuy/edgequake

Run it:

```bash
cd edgequake_webui
bun install
bun run dev
```

Would love feedback, especially:

- Performance with larger graphs
- Accessibility improvements
- Streaming edge cases I missed

Happy to answer questions about specific implementation details!
