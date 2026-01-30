# Why I Built an AI Interface That Shows Its Thinking

_On transparency, trust, and the details that matter_

---

## The Frustration That Started It

I was using an AI tool for document research. Asked a question. Got an answer. Seemed good.

Then someone asked where that answer came from. Which documents? Which sections?

I had no idea.

The tool didn't tell me. It just... answered. Like a magician pulling information from a hat. Impressive, but impossible to verify.

For my use case—legal research—that was a problem. You can't cite "the AI said so" in a legal brief. You need sources. You need verification. You need to trust the answer.

That frustration is why I built the EdgeQuake WebUI differently.

---

## What "Showing Your Work" Looks Like

EdgeQuake is a Graph-RAG system. It builds a knowledge graph from documents, then uses that graph to answer questions. The process is inherently visual.

So the interface shows everything:

**Document Processing**

When you upload documents, you see exactly what's happening:

- Each document has a status badge (pending, processing, completed, failed)
- A progress bar shows extraction progress
- Cost tracking tells you what each document costs to process
- Failed documents have a retry button—not a dead end

You're not staring at "Processing..." and hoping. You're watching the system work.

**The Knowledge Graph**

After processing, you can explore the graph visually. Entities are nodes. Relationships are edges. You can:

- Zoom and pan
- Filter by entity type (people, organizations, concepts)
- Filter by time (see how the graph evolved)
- Click nodes to see their connections
- Search for specific entities

This isn't a gimmick. It's verification. You can see what the system "learned" from your documents. If entities are wrong, you spot it immediately.

**Streaming Responses**

When you ask a question, the answer streams token-by-token. You see the AI "typing" in real-time.

But more importantly: the chain-of-thought display shows what the AI is reasoning about _before_ generating the final answer. This makes the thinking process visible.

**Source Citations**

Every answer includes citations. Click a citation to see the original source text. Verify claims against documents.

This is the transparency I was missing.

---

## The Technical Challenge I Didn't Expect

Streaming markdown from an LLM sounds simple. It's not.

LLM tokenizers are optimized for natural language, not markdown. They often add leading spaces to word tokens. When tokens get concatenated during streaming, markdown syntax breaks.

Example:

```
Tokens arrive: ["The", "**", " Code2Doc", "**", " project"]
Concatenated: "The** Code2Doc** project"
Expected: "The **Code2Doc** project"
```

The `**` attached to "The" instead of standing alone. Now markdown parsing fails.

I ended up writing 442 lines of streaming markdown normalization logic. It handles:

- Bold/italic token reattachment
- Table buffering (partial tables look broken)
- Code block detection (need the closing fence for syntax highlighting)
- Math expressions (KaTeX needs complete expressions)
- 60fps auto-scroll throttling (naive implementation causes jank)

This is the kind of detail work that separates polished interfaces from prototypes. Nobody notices when it works. Everyone notices when it doesn't.

---

## What I Learned About Building AI Interfaces

**1. Transparency builds trust**

When users can see the knowledge graph, they trust the system more. They can verify that entities were extracted correctly. They can understand where answers come from.

Black boxes create doubt. Visibility creates confidence.

**2. Streaming is complex**

Don't underestimate the work required for polished streaming. Token normalization, buffering, scroll behavior—these all need careful implementation.

**3. Performance is UX**

A laggy graph viewer feels broken, even if it's functionally correct. I spent significant time on Sigma.js optimization—WebGL rendering, level-of-detail, culling, web worker layouts.

60fps is the baseline for feeling professional.

**4. Details compound**

Every small improvement—better loading states, clearer error messages, smoother animations—compounds into an overall experience that feels polished.

**5. Components accelerate**

Starting with shadcn/ui let me build 100+ components quickly. All consistent, all customizable. The copy-paste model sounds weird but works.

---

## What's Next

The interface is open source, and there's more to do:

- **Larger graphs**: Handling 10,000+ nodes smoothly
- **Accessibility**: Better screen reader support, keyboard navigation
- **Collaboration**: Shared workspaces, annotations, comments
- **Mobile**: Better touch interactions for graph exploration

Building AI interfaces is becoming its own discipline. The patterns that work for traditional apps don't always transfer. Streaming, chain-of-thought, citations, verification—these are new requirements.

I'm learning as I go. And sharing what I learn.

---

The EdgeQuake WebUI is at: github.com/raphaelmansuy/edgequake

If you're building AI interfaces, I'd love to hear what patterns you've found. The field is still young, and we're all figuring it out together.

_Because AI should show its work._
