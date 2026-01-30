# EdgeQuake WebUI: AI That Shows Its Work (X.com Thread)

## Tweet 1/14

Most AI interfaces feel the same:

Type question → Loading spinner → Wall of text

Where did it come from? No idea.

We built something different.

Here's the EdgeQuake WebUI—an AI interface that shows its work 🧵

## Tweet 2/14

The Problem:

AI tools are black boxes. You ask a question, you get an answer.

But:
• Which documents informed it?
• How did it find that info?
• What did it "think" before answering?
• What did it cost to process?

These questions matter. Most tools don't answer them.

## Tweet 3/14

The Principle: Show Everything

EdgeQuake WebUI makes every step visible:

1️⃣ Document processing with status tracking
2️⃣ Knowledge graph you can explore
3️⃣ Chain-of-thought reasoning
4️⃣ Source citations for every answer
5️⃣ Cost per document

Transparency builds trust.

## Tweet 4/14

Document Processing:

Upload docs → watch entities extracted in real-time.

Status badges show exactly where each doc is:
• 🟡 Pending
• 🔵 Processing
• 🟢 Completed
• 🔴 Failed (with retry button)

Plus: cost tracking per document. Know what you're spending.

## Tweet 5/14

Knowledge Graph Exploration:

After processing, explore the graph visually.

Interactive Sigma.js visualization with:
• Zoom, pan, click nodes
• Filter by entity type
• Time-based filtering
• Minimap for large graphs
• Multiple layout algorithms

This isn't a gimmick—it's verification.

## Tweet 6/14

Graph Performance:

We use Sigma.js 3.0 with WebGL acceleration.

Handles 1000+ nodes smoothly because:
• GPU-accelerated rendering (not SVG)
• Level-of-detail hiding
• Off-screen culling
• Web worker layouts

Professional feel requires professional performance.

## Tweet 7/14

Query Interface:

Four query modes:
🎯 Local: Entity neighborhood search
🌍 Global: Full graph search
📚 Hybrid: Combined (recommended)
⚡ Naive: Skip graph, direct LLM

You choose the retrieval strategy. Different queries need different approaches.

## Tweet 8/14

Streaming Responses:

Answers stream token-by-token. You see the AI "typing."

Plus: Chain-of-thought display shows what the AI is reasoning about BEFORE the final answer.

This makes the thinking process visible.

## Tweet 9/14

The Hard Part: Streaming Markdown

LLM tokenizers add leading spaces that break markdown during streaming.

"The** Code2Doc**" instead of "The **Code2Doc**"

We built 442 lines of normalization logic to fix these in real-time.

Details matter.

## Tweet 10/14

More Streaming Challenges:

• Tables must be complete before rendering (buffered)
• Code blocks need syntax highlighting (wait for closing fence)
• Math needs KaTeX (wait for complete expressions)
• Auto-scroll throttled to 60fps

Polished streaming is complex.

## Tweet 11/14

Tech Stack:

• React 19.2.3 (concurrent features)
• Next.js 16.1.0 (App Router)
• Tailwind CSS 4.1.18
• shadcn/ui (100+ components)
• Zustand (state)
• TanStack Query (data fetching)
• Sigma.js 3.0.2 (graphs)

All latest versions.

## Tweet 12/14

Component Count:

• 17+ Query components
• 26+ Graph components
• 18+ Document components
• 40+ UI primitives

897 lines in QueryInterface alone.
785 lines in GraphViewer.

This isn't a demo. It's a production interface.

## Tweet 13/14

What We Learned:

1. Streaming is complex—plan for normalization
2. Visibility builds trust—show the graph
3. Components compound—shadcn/ui accelerates
4. Performance is UX—60fps or it feels broken
5. Accessibility matters—keyboard nav, ARIA labels

## Tweet 14/14

EdgeQuake WebUI is open source.

Try it:

```
cd edgequake_webui
bun install && bun run dev
```

Repo: github.com/raphaelmansuy/edgequake

AI should show its work.
This one does.
