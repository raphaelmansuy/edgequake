# The Best AI Interfaces Show Their Thinking

Most AI tools feel like black boxes.

Type a question. Loading spinner. Wall of text appears.
Where did that answer come from? Which documents? No idea.

When we built the EdgeQuake WebUI, we made a different choice: **show everything**.

## 🔍 What "Show Everything" Means

**Document Processing Visibility**
→ Watch entities being extracted in real-time
→ Status badges: Pending, Processing, Completed, Failed
→ Cost tracking per document
→ Retry failed documents with one click

**Knowledge Graph Exploration**
→ Interactive Sigma.js visualization
→ Filter by entity type (people, organizations, concepts)
→ Time-based filtering to see graph evolution
→ Minimap for graphs with 1000+ nodes

**Streaming Responses**
→ Token-by-token generation (not loading → dump)
→ Chain-of-thought display shows AI reasoning
→ Source citations for every answer
→ Four query modes: Local, Global, Hybrid, Naive

## 💡 Why Transparency Matters

For enterprise users, transparency isn't nice-to-have—it's required:
• Legal teams verify where answers originate
• Finance tracks what processing costs
• Engineers debug when things break
• Everyone trusts the system more

## ⚡ The Tech Stack

React 19.2.3 (latest concurrent features)
Next.js 16.1.0 (App Router, streaming)
Sigma.js 3.0.2 (WebGL-accelerated graphs)
shadcn/ui (100+ customizable components)
TanStack Query (automatic caching)
Zustand (minimal state management)

## 🔧 The Hard Part: Streaming Markdown

LLM tokenizers add leading spaces that break markdown during streaming:

`"The** Code2Doc**"` instead of `"The **Code2Doc**"`

Our StreamingMarkdownRenderer (442 lines) handles:
• Token normalization in real-time
• Table buffering (complete tables only)
• 60fps auto-scroll throttling

This is the detail work that separates polished AI interfaces from prototypes.

## 📊 By The Numbers

• 100+ React components
• 897 lines in QueryInterface
• 785 lines in GraphViewer
• 4 query modes exposed in UI

---

AI tools that show their work build trust.
AI tools that hide their work create doubt.

EdgeQuake WebUI shows everything.

🔗 github.com/raphaelmansuy/edgequake

#AIInterface #ReactJS #NextJS #GraphVisualization #Streaming #LLM #OpenSource #RAG #KnowledgeGraph #WebDevelopment
