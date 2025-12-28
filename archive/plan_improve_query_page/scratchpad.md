# Query Page UX/UI Improvement - Research Scratchpad

## 2025-12-27 10:00 - Initial Setup

- Starting comprehensive audit of Query Page
- Key areas to investigate: markdown rendering, streaming, state persistence
- Will compare with OpenWebUI implementation

---

## 2025-12-27 10:15 - OpenWebUI Markdown Rendering Analysis

### Key Findings from OpenWebUI Source Code:

**1. Architecture:**

- Uses `marked` library for tokenization (same as EdgeQuake)
- Svelte-based component system (EdgeQuake uses React/Next.js)
- Token-based rendering with specialized components:
  - `Markdown.svelte` - Main entry point
  - `MarkdownTokens.svelte` - Block-level token renderer
  - `MarkdownInlineTokens.svelte` - Inline token renderer
  - `KatexRenderer.svelte` - Math rendering
  - `CodeBlock.svelte` - Code with syntax highlighting
  - `AlertRenderer.svelte` - GitHub-style alerts

**2. Marked Extensions Used:**

- `markedKatexExtension` - KaTeX math support
- `markedExtension` - Details/collapsible blocks
- `citationExtension` - Source citations
- `footnoteExtension` - Footnotes
- `mentionExtension` - @mentions and #channels
- `disableSingleTilde` - Strikethrough handling

**3. KaTeX Extension Pattern:**

```ts
// Delimiters supported:
const DELIMITER_LIST = [
  { left: "$$", right: "$$", display: true },
  { left: "$", right: "$", display: false },
  { left: "\\pu{", right: "}", display: false },
  { left: "\\ce{", right: "}", display: false },
  { left: "\\(", right: "\\)", display: false },
  { left: "\\[", right: "\\]", display: true },
  { left: "\\begin{equation}", right: "\\end{equation}", display: true },
];
```

**4. Streaming Handling:**

- Uses `done` prop to track streaming status
- `TextToken.svelte` adds fade animations for streaming text
- Progressive rendering with token-by-token display

**5. Key Differences from EdgeQuake:**

- OpenWebUI uses lazy loading for heavy components (Mermaid, Vega)
- DOMPurify for HTML sanitization
- More extensive token type support
- Better error handling with try-catch blocks

---

## 2025-12-27 10:25 - EdgeQuake Current Implementation Analysis

### EdgeQuake Query Page Components:

**1. Component Hierarchy:**

- `query/page.tsx` → `QueryInterface`
- `QueryInterface` → `ChatMessage`, `ConversationHistoryPanelV2`
- `ChatMessage` → `StreamingMarkdownRenderer`
- `StreamingMarkdownRenderer` → `MarkdownTokens`
- `MarkdownTokens` → `MarkdownInlineTokens`, `CodeBlock`, `MermaidBlock`, `KatexMath`

**2. State Management:**

- Uses Zustand (`useQueryUIStore`) for UI state
- React Query for server state (`useConversations`, `useConversation`)
- Local state for streaming messages (`pendingMessage`)

**3. Streaming Implementation:**

- Uses `chatCompletionStream` for SSE streaming
- Handles `thinking`, `generating`, `complete` states
- Pending message pattern for optimistic updates

**4. Known Issues from Code Review:**

- Markdown normalization for streaming artifacts (spaces around **bold**)
- Limited token type support compared to OpenWebUI
- No citation extension (only basic source display)
- No footnote or alert support

---

## 2025-12-27 11:10 - Execution Complete

### Summary of Work Done:

1. **Phase 1 - Discovery & Analysis**

   - Analyzed 10+ EdgeQuake component files
   - Fetched and analyzed OpenWebUI GitHub repo
   - Identified key gaps in markdown rendering
   - Created comprehensive audit document

2. **Phase 2 - Design Strategy**

   - Defined design principles (Progressive, Accessible, Performant)
   - Created information architecture diagrams
   - Documented interaction patterns with ASCII diagrams
   - Established OKLCH color system specs

3. **Phase 3 - Technical Specification**

   - Created database schema DDL for new features
   - Documented OpenAPI 3.0 specifications
   - Provided TypeScript code for marked extensions
   - Specified React Query configuration

4. **Phase 4 - Implementation Roadmap**

   - Prioritized 18 features into P0/P1/P2/P3
   - Created 3-sprint plan with weekly breakdowns
   - Defined acceptance criteria per sprint
   - Created risk register with 10 identified risks

5. **Phase 5 - Design Mockups**
   - Created ASCII wireframes for all major views
   - Documented responsive layouts (desktop/tablet/mobile)
   - Specified component dimensions and styling
   - Illustrated loading/error/empty states

### Files Created:

- `01_audit_findings.md`
- `02_design_strategy.md`
- `03_technical_spec.md`
- `04_implementation_roadmap.md`
- `05_design_mockups.md`
- `README.md`

### Key Technical Decisions:

1. Use DOMPurify for HTML sanitization (security)
2. Buffer incomplete tables during streaming (UX)
3. Port marked extensions from OpenWebUI (feature parity)
4. Use Zustand + immer for complex state (maintainability)
5. Virtual scrolling for long conversations (performance)
