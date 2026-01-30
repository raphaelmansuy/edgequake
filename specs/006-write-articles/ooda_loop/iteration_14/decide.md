# Decide Phase - Iteration 14: WebUI Experience

## Article Plan

### Core Message

"The best AI interfaces show their work. EdgeQuake WebUI makes every step visible—from document upload to graph exploration to streaming responses with chain-of-thought reasoning."

### Unique Angle

Most articles about AI UIs focus on chat. This article focuses on **transparency**:

- See the knowledge graph being built
- Watch tokens stream in real-time
- Understand where answers come from
- Know exactly what processing costs

## Platform-Specific Outlines

### Medium Article (~2200 words)

**Title**: "Building an AI Interface That Shows Its Work: Inside the EdgeQuake WebUI"

**Structure**:

1. **Hook** (100 words): The frustration of AI black boxes
2. **The Problem** (200 words): Why most AI interfaces feel opaque
3. **The Solution** (300 words): Transparency as a design principle
4. **Tech Stack** (200 words): React 19, Next.js 16, Sigma.js, shadcn/ui
5. **User Flow Walkthrough** (400 words): Upload → Process → Graph → Query
   - ASCII diagram of flow
6. **Technical Deep Dive: Streaming Markdown** (400 words)
   - LLM tokenizer challenges
   - Token normalization
   - Table buffering
   - Code snippet from StreamingMarkdownRenderer
7. **Graph Visualization** (200 words): Sigma.js with 1000+ nodes
8. **Component Architecture** (200 words): 100+ components, shadcn/ui base
9. **Call to Action** (100 words): Try the demo, contribute

### LinkedIn Post (~2800 chars)

**Hook**: "The best AI interfaces show their thinking."

**Structure**:

- Problem: AI tools feel like black boxes
- Solution: Transparency at every step
- Key features (bullets with emojis)
- Tech stack mention
- Call to action

### X.com Thread (14 tweets)

**Thread arc**:

1. Hook: Why AI interfaces need to show their work
   2-4. Document upload and processing visibility
   5-7. Knowledge graph visualization
   8-10. Streaming query interface with chain-of-thought
   11-12. Tech stack highlights
2. Component count and maintainability
3. Call to action with links

### HackerNews Post (~700 words)

**Title**: "Show HN: EdgeQuake WebUI – React 19 + Next.js 16 interface for Graph-RAG"

**Structure**:

- What it is (brief)
- Tech stack choices and why
- Interesting technical challenge (streaming markdown)
- Performance considerations
- Open source, contributions welcome

### Reddit Post (~850 words)

**Target subreddits**: r/reactjs, r/webdev, r/nextjs

**Angle**: "I built a streaming AI interface with React 19 and Next.js 16. Here's what I learned."

**Structure**:

- Context: Building UI for a Graph-RAG system
- Tech stack: React 19, Next.js 16, Zustand, TanStack Query
- Streaming challenges and solutions
- shadcn/ui experience
- Lessons learned
- Request for feedback

### Substack Article (~1400 words)

**Title**: "Why I Built an AI Interface That Shows Its Thinking"

**Tone**: Personal, reflective

**Structure**:

1. The frustration that started it all
2. Why transparency matters in AI
3. What "showing your work" looks like
4. The technical challenge of streaming markdown
5. What I learned about building AI interfaces
6. What's next

## Technical Content to Include

### Code Snippet: Streaming Normalization

```tsx
/**
 * LLM tokenizers often add leading spaces to word tokens.
 * This breaks markdown syntax during streaming.
 *
 * "The** Code2Doc**" → "The **Code2Doc**"
 */
function normalizeMarkdownForStreaming(content: string): string {
  let normalized = content;

  // Pattern: word** text → word **text
  // LLM tokenizers attach ** to previous word during streaming
  normalized = normalized.replace(
    /(?<!\*\*[^*]*)([a-zA-Z0-9])\*\* (\w)/g,
    "$1 **$2",
  );

  return normalized;
}
```

### Key Metrics

- 100+ React components
- 897 lines in QueryInterface
- 785 lines in GraphViewer
- 442 lines in StreamingMarkdownRenderer
- 4 query modes exposed in UI

## Validation Checklist

- [ ] All articles credit the tech stack accurately
- [ ] Code snippets are from actual codebase
- [ ] ASCII diagrams are clear and helpful
- [ ] Each platform has appropriate length and tone
- [ ] Technical claims are verifiable
- [ ] Call to action is clear
