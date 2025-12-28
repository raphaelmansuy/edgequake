# EdgeQuake Query UX/UI Craftpad

> Brainstorming and raw findings for the chat-based query interface redesign.

## Audit Date: December 26, 2025

---

## 🎯 Project Goal

Create the most **slick, delightful user experience** for chat-based data querying and visualization with beautifully integrated markdown, code snippets, mermaid diagrams, and KaTeX math rendering.

---

## 📸 Current State Assessment

### Screenshots Captured
1. `01-dashboard.png` - Dashboard overview
2. `02-query-empty.png` - Query page empty state
3. `03-query-response.png` - Query with response
4. `04-documents.png` - Documents list
5. `05-knowledge-graph.png` - Graph visualization
6. `06-settings.png` - Settings page
7. `07-api-explorer.png` - API Explorer
8. `08-query-mobile.png` - Mobile responsive view

---

## 🏗️ Current Architecture

### Tech Stack
- **Framework**: Next.js 16.1 with React 19
- **Styling**: Tailwind CSS v4 with design tokens
- **Components**: shadcn/ui (Radix primitives)
- **Markdown**: react-markdown v10+
- **Syntax Highlighting**: rehype-highlight
- **Math**: KaTeX (lazy-loaded)
- **Diagrams**: Mermaid (lazy-loaded)
- **State**: Zustand stores
- **Queries**: TanStack Query

### Key Files
```
src/components/query/
├── query-interface.tsx      # Main chat container (1094 lines)
├── markdown-renderer.tsx    # Markdown with code/mermaid/katex (515 lines)
├── conversation-history-panel.tsx
├── query-mode-selector.tsx
├── source-citations.tsx
└── thinking-display.tsx

src/app/
├── globals.css             # Global styles (622 lines)
└── design-tokens.css       # Design system tokens (466 lines)
```

---

## ✅ What's Working Well

1. **Loading Animation** - Shimmer effect with animated dots is delightful
2. **Mode Selector** - Clean toggle between Local/Global/Hybrid/Simple
3. **Conversation History** - Timestamps and message counts
4. **Error Boundary** - Graceful fallback for markdown parsing errors
5. **Design Tokens** - Comprehensive spacing and typography system
6. **Theme Support** - Dark/light mode with oklch color space
7. **Streaming Support** - Partial content handling during streaming
8. **Source Citations** - Entity and document linking
9. **Code Block Copy** - Functional copy button with feedback
10. **Mermaid Support** - Dynamic diagram rendering

---

## ❌ Pain Points Identified

### Visual Polish Issues

#### Message Bubbles
- [ ] Basic rounded corners, no shadows or depth
- [ ] User bubble (primary color) lacks contrast refinement
- [ ] Assistant bubble styling is generic
- [ ] No slide-in animation on new messages
- [ ] Metadata (time, tokens) appears too dense

#### Code Blocks
- [ ] Header with language badge overlaps content area
- [ ] Copy button only appears on hover (accessibility concern)
- [ ] Syntax theme doesn't match app theme perfectly
- [ ] No line numbers for longer code
- [ ] Very long code blocks need collapse/expand

#### Markdown Rendering
- [ ] Typography could have better hierarchy
- [ ] Tables lack visual distinction
- [ ] Blockquotes are too subtle
- [ ] Lists need better bullet styling
- [ ] Links need better hover states

#### Mermaid Diagrams
- [ ] No container padding
- [ ] Dark mode colors may not match theme
- [ ] No zoom/fullscreen capability
- [ ] Loading state is basic placeholder
- [ ] Error state could be more helpful

#### KaTeX Math
- [ ] LaTeX rendering disabled during streaming (breaks display)
- [ ] No visual indicator that math is being rendered
- [ ] Inline vs block math styling could be more distinct

### Layout Issues

#### Query Page
- [ ] Three-column layout feels cramped
- [ ] History panel takes too much horizontal space
- [ ] Empty state suggestions are too uniform
- [ ] Input area is basic, lacks refinement
- [ ] Breadcrumb takes up vertical space

#### Mobile Responsive
- [ ] Header elements get cut off at 375px
- [ ] History panel isn't accessible on mobile
- [ ] Mode selector wraps awkwardly
- [ ] Input area needs mobile optimization

### Interaction Issues

#### Input Area
- [ ] No character count or limit indicator
- [ ] Textarea doesn't auto-resize elegantly
- [ ] Send button disabled state isn't clear
- [ ] No keyboard shortcut hints

#### History Panel
- [ ] Search is basic text matching
- [ ] No grouping by date
- [ ] Delete confirmation is modal-based
- [ ] No swipe-to-delete on mobile
- [ ] Active conversation highlight is subtle

---

## 💡 Improvement Ideas

### Visual Enhancements

#### Message Bubble Redesign
```css
/* User Message */
- Gradient background: from-primary to-primary/90
- Subtle shadow: 0 2px 8px rgba(0,0,0,0.1)
- Rounded: rounded-2xl rounded-tr-sm
- Slide-in animation from right

/* Assistant Message */
- Background: card with subtle border
- Avatar: gradient ring effect
- Subtle shadow: 0 1px 4px rgba(0,0,0,0.05)
- Slide-in animation from left
```

#### Code Block Redesign
```
┌─────────────────────────────────────────┐
│ javascript                     📋 Copy  │
├─────────────────────────────────────────┤
│   1 │ function greet(name) {            │
│   2 │   console.log(`Hello, ${name}!`); │
│   3 │ }                                 │
└─────────────────────────────────────────┘

- Line numbers in muted gutter
- Language badge left-aligned
- Copy button always visible
- Collapse toggle for >20 lines
- Themed syntax colors matching app
```

#### Markdown Typography
```css
/* Headings */
h1: text-xl font-bold tracking-tight
h2: text-lg font-semibold
h3: text-base font-medium

/* Lists */
- Custom bullet/number styling
- Nested list indentation
- Proper spacing between items

/* Blockquotes */
- Thicker left border (4px)
- Background tint
- Icon indicator

/* Tables */
- Zebra striping
- Sticky header
- Horizontal scroll container
```

### Layout Improvements

#### Query Page Redesign
```
┌─────────────────────────────────────────────────────────┐
│ [≡] EdgeQuake    Test Workspace ▾    🌍 ☀️ 👤           │
├─────────────────────────────────────────────────────────┤
│ ┌──────────┬────────────────────────────┬─────────────┐ │
│ │          │                            │             │ │
│ │ History  │     Chat Messages          │  (Collapsed │ │
│ │ Panel    │                            │   Details)  │ │
│ │ (280px)  │     - Centered content     │             │ │
│ │          │     - Max-width 800px      │             │ │
│ │          │     - Proper spacing       │             │ │
│ │          │                            │             │ │
│ ├──────────┼────────────────────────────┤             │ │
│ │          │ ┌──────────────────────┐   │             │ │
│ │          │ │ Ask a question...    │   │             │ │
│ │          │ │                 [→]  │   │             │ │
│ │          │ └──────────────────────┘   │             │ │
│ │          │ Local | Global | Hybrid    │             │ │
│ └──────────┴────────────────────────────┴─────────────┘ │
└─────────────────────────────────────────────────────────┘
```

#### Mobile Layout (< 768px)
```
┌─────────────────────────┐
│ [≡] EdgeQuake      🔍 ☰ │
├─────────────────────────┤
│                         │
│    Chat Messages        │
│    (Full width)         │
│                         │
│                         │
├─────────────────────────┤
│ ┌─────────────────────┐ │
│ │ Ask a question...   │ │
│ │                [→]  │ │
│ └─────────────────────┘ │
│ Local | Global | Hybrid │
└─────────────────────────┘

- History: Bottom sheet or off-canvas
- Details: Expandable drawer
- Mode selector: Horizontal scroll
```

### Micro-interactions

#### Message Animations
```css
@keyframes messageSlideIn {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.message-enter {
  animation: messageSlideIn 0.2s ease-out;
}
```

#### Thinking State
```
┌────────────────────────────────────────┐
│ 🧠 Reasoning...                    2.1s │
│ ┌────────────────────────────────────┐  │
│ │ ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │  │
│ │ Analyzing knowledge graph...       │  │
│ └────────────────────────────────────┘  │
└────────────────────────────────────────┘
```

#### Input Focus State
```css
.input-container:focus-within {
  box-shadow: 0 0 0 2px var(--primary);
  border-color: var(--primary);
}
```

---

## 🎨 Design Token Updates Needed

### New Tokens
```css
/* Chat-specific tokens */
--chat-message-max-width: 800px;
--chat-message-user-bg: var(--primary);
--chat-message-assistant-bg: var(--card);
--chat-bubble-radius: 1.25rem;
--chat-bubble-radius-tail: 0.25rem;
--chat-message-shadow: 0 1px 4px rgba(0,0,0,0.08);

/* Code block tokens */
--code-block-bg: oklch(0.97 0 0);
--code-block-header-bg: oklch(0.94 0 0);
--code-block-border: oklch(0.9 0 0);
--code-line-number-color: oklch(0.6 0 0);

/* Animation tokens */
--animation-message-slide: 0.2s ease-out;
--animation-thinking-pulse: 1s ease-in-out infinite;
```

---

## 📋 Component Specifications Needed

1. **ChatMessage** - Enhanced message bubble component
2. **CodeBlock** - Polished code display with line numbers
3. **MermaidDiagram** - Container with zoom/fullscreen
4. **MathBlock** - KaTeX rendering with styling
5. **ChatInput** - Enhanced input with attachments
6. **HistoryPanel** - Improved conversation list
7. **ThinkingIndicator** - Refined loading state
8. **SourceCitation** - Better visual integration

---

## 🔄 User Flow Optimizations

### Query Flow
1. User arrives → Show engaging empty state
2. User types → Auto-suggest, character count
3. User submits → Immediate feedback, thinking animation
4. Response streams → Smooth content appearance
5. Response complete → Show metadata, actions
6. User interacts → Copy, regenerate, cite sources

### Mobile Flow
1. User taps query → Full-screen input
2. User submits → Return to chat view
3. User swipes → Access history (bottom sheet)
4. User taps message → Expand actions

---

## 🚀 Implementation Priority

### Phase 1: Visual Polish (High Impact)
- [ ] Message bubble shadows and animations
- [ ] Code block redesign
- [ ] Input area refinement
- [ ] Typography improvements

### Phase 2: Functional Enhancements
- [ ] Line numbers for code
- [ ] Collapsible long content
- [ ] History grouping by date
- [ ] Mobile-optimized layout

### Phase 3: Delight Features
- [ ] Smooth scroll behavior
- [ ] Advanced mermaid controls
- [ ] Keyboard shortcuts
- [ ] Sound feedback (optional)

---

## 📚 References

- **ChatGPT**: Minimal, focused, excellent streaming
- **Claude**: Clean typography, good code blocks
- **Perplexity**: Source integration, visual hierarchy
- **OpenWebUI**: Customizable, feature-rich
- **shadcn/ui**: Component patterns and styling

---

## 🗒️ Notes

- Current markdown-renderer.tsx has extensive error handling for streaming
- Design tokens are well-organized, extend rather than replace
- React 19 with Next.js 16 - use modern patterns
- Tailwind v4 with oklch colors - stay consistent
- Consider reduced motion preferences for animations

---

*Last updated: December 26, 2025*
