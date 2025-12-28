# Phase 2: Design Strategy

**Document**: `02_design_strategy.md`  
**Created**: 2024-12-27  
**Status**: Complete

---

## 1. Design Principles for EdgeQuake Query Page

### 1.1 The SLICK Manifesto

EdgeQuake's Query Page should embody **SLICKness** – a design philosophy that prioritizes:

| Principle       | Definition                      | Example                               |
| --------------- | ------------------------------- | ------------------------------------- |
| **S**imple      | Remove unnecessary complexity   | One input, one output, one history    |
| **L**ightweight | Fast, responsive, minimal bloat | <200ms TTI, <50ms streaming latency   |
| **I**ntuitive   | Zero learning curve             | New users submit query in <30 seconds |
| **C**lean       | Visual clarity over decoration  | High contrast, consistent spacing     |
| **K**inetic     | Smooth, purposeful motion       | Shimmer loading, fade-in tokens       |

### 1.2 Seven Design Principles

#### Principle 1: Progressive Disclosure

> Show only what's needed now; reveal complexity on demand.

**Application**:

- Default view: Query input + response
- Collapsed: Chain-of-thought reasoning
- Hidden: Query settings sheet
- On-demand: Source citations expandable

```
┌─────────────────────────────────────────┐
│ Query Input                        [⚙️] │  ← Settings hidden
├─────────────────────────────────────────┤
│ Response                                │
│ └─ [▶ Show reasoning]              ← COT collapsed
│ └─ [📚 3 sources]                  ← Citations collapsed
└─────────────────────────────────────────┘
```

#### Principle 2: Streaming-First Design

> Design for real-time content; final state is bonus.

**Application**:

- Skeleton states during `thinking` phase
- Token-by-token text reveal during `generating`
- No layout shift on completion
- Mermaid/KaTeX show "Rendering..." until ready

#### Principle 3: Conversation as Primary Unit

> History is organized by conversations, not individual queries.

**Application**:

- Sidebar shows conversation titles, not query text
- Conversations are named automatically from first query
- Messages inherit conversation's tenant/workspace

#### Principle 4: Persistent Context

> User's work should never be lost.

**Application**:

- Server-side persistence by default
- Offline-first with localStorage cache
- Sync indicator shows save status
- Auto-save on every message

#### Principle 5: Accessible Density

> Information-rich without feeling crowded.

**Application**:

- 16px base spacing, 8px for tight groups
- Maximum 3 levels of visual hierarchy per region
- Token counts and timing in muted secondary text
- Metadata revealed on hover/expand

#### Principle 6: Mode-Aware Styling

> Query mode affects response presentation.

**Application**:

- Mode badge visible in response header
- Color accent varies by mode (Local=blue, Global=green, Hybrid=purple, Naive=gray)
- Suggested prompts adapt to selected mode

#### Principle 7: Error as Opportunity

> Errors should guide, not block.

**Application**:

- Failed markdown renders with "Show raw" fallback
- Network errors show retry button + offline indicator
- Empty history suggests importing or starting fresh

---

## 2. Visual Hierarchy Rules

### 2.1 Typography Scale

| Level   | Size | Weight | Use Case                 |
| ------- | ---- | ------ | ------------------------ |
| H1      | 24px | 700    | Page title ("Query")     |
| H2      | 18px | 600    | Section headers          |
| H3      | 16px | 600    | Card titles              |
| Body    | 14px | 400    | Response text, UI labels |
| Caption | 12px | 400    | Timestamps, metadata     |
| Micro   | 10px | 500    | Badges, token counts     |

### 2.2 Spacing System

```
Base unit: 4px

Micro:    4px   (within components, icon gaps)
Small:    8px   (related elements, list items)
Medium:   16px  (between sections, card padding)
Large:    24px  (major section separation)
XLarge:   32px  (page margins, region separation)
```

### 2.3 Color Semantics

| Role        | Light Mode             | Dark Mode              | Use                |
| ----------- | ---------------------- | ---------------------- | ------------------ |
| Primary     | `oklch(0.65 0.25 265)` | `oklch(0.75 0.2 265)`  | Actions, links     |
| User Bubble | `oklch(0.65 0.25 265)` | `oklch(0.55 0.2 265)`  | User messages      |
| AI Bubble   | `oklch(0.98 0.01 265)` | `oklch(0.18 0.01 265)` | Assistant messages |
| Success     | `oklch(0.7 0.2 145)`   | `oklch(0.75 0.15 145)` | Completed states   |
| Warning     | `oklch(0.8 0.15 85)`   | `oklch(0.85 0.12 85)`  | Attention needed   |
| Error       | `oklch(0.65 0.25 25)`  | `oklch(0.7 0.2 25)`    | Failures           |
| Muted       | `oklch(0.55 0.01 265)` | `oklch(0.65 0.01 265)` | Secondary text     |

### 2.4 Mode Color Accents

| Mode   | Accent | Hex Approx |
| ------ | ------ | ---------- |
| Local  | Blue   | #3B82F6    |
| Global | Green  | #10B981    |
| Hybrid | Purple | #8B5CF6    |
| Naive  | Gray   | #6B7280    |

---

## 3. Information Architecture

### 3.1 Current IA (Problematic)

```
Query Page
├── Header (fixed)
│   ├── Title
│   ├── Mode Selector
│   └── Settings
├── Main Content (scrollable)
│   ├── Empty State OR
│   └── Message List
├── Input (fixed)
└── History Panel (collapsible sidebar)
```

**Issues**:

- History panel competes with main content
- No clear visual separation between queries
- Conversation context invisible once started

### 3.2 Redesigned IA

```
Query Page
├── Left Rail: History Panel (collapsible, 280px)
│   ├── New Conversation CTA
│   ├── Search/Filter Bar
│   │   ├── Text search
│   │   ├── Date range filter
│   │   └── Mode filter dropdown
│   └── Conversation List (virtualized, paginated)
│       └── Conversation Item
│           ├── Title (auto or custom)
│           ├── Preview (first line of last message)
│           ├── Metadata (time, mode, msg count)
│           └── Actions (rename, delete, export)
│
├── Main Area
│   ├── Conversation Header (sticky)
│   │   ├── Title (editable)
│   │   ├── Mode Badge
│   │   ├── Save Status Indicator
│   │   └── Actions (share, export, delete)
│   │
│   ├── Message Thread (scrollable)
│   │   └── Message[]
│   │       ├── User Message Bubble
│   │       └── Assistant Message Bubble
│   │           ├── Thinking (collapsible)
│   │           ├── Response (markdown)
│   │           └── Sources (expandable)
│   │
│   └── Input Area (sticky bottom)
│       ├── Mode Selector (compact)
│       ├── Query Textarea
│       └── Send/Stop Button
│
└── Right Rail: Context Panel (optional, 320px)
    ├── Query Settings
    ├── Graph Stats
    └── Active Document Context
```

### 3.3 IA Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Query Page                                      │
├──────────────────┬─────────────────────────────────────────┬────────────────┤
│                  │                                         │                │
│  HISTORY RAIL    │            MAIN AREA                    │ CONTEXT RAIL   │
│  (280px)         │            (flex)                       │ (320px)        │
│                  │                                         │ [collapsible]  │
│  ┌────────────┐  │  ┌─────────────────────────────────┐   │                │
│  │ + New Chat │  │  │ Conversation Title    [⚙️] [↗️] │   │  ┌──────────┐  │
│  └────────────┘  │  │ Mode: Hybrid  •  Saving...     │   │  │ Settings │  │
│                  │  └─────────────────────────────────┘   │  └──────────┘  │
│  🔍 Search...    │                                         │                │
│  [Date ▾][Mode ▾]│  ┌─────────────────────────────────┐   │  ┌──────────┐  │
│                  │  │                                 │   │  │ Graph    │  │
│  ───────────────  │  │  User: What entities are       │   │  │ Stats    │  │
│                  │  │        connected to Sarah?      │   │  │ 📊 142   │  │
│  📝 Research...  │  │                                 │   │  └──────────┘  │
│     Dec 27, 3:15 │  ├─────────────────────────────────┤   │                │
│     Hybrid • 12  │  │                                 │   │  ┌──────────┐  │
│                  │  │  🤖 [▶ Show reasoning]          │   │  │ Context  │  │
│  📝 API Design   │  │                                 │   │  │ 3 docs   │  │
│     Dec 27, 2:00 │  │  Sarah Chen is connected to:    │   │  │ active   │  │
│     Local • 5    │  │  • **TechCorp** (employer)     │   │  └──────────┘  │
│                  │  │  • **John Smith** (colleague)   │   │                │
│  📝 Entity Map   │  │                                 │   │                │
│     Dec 26, 4:30 │  │  [📚 Sources (2)]               │   │                │
│     Global • 8   │  │                                 │   │                │
│                  │  └─────────────────────────────────┘   │                │
│  [Load more...]  │                                         │                │
│                  │  ┌─────────────────────────────────┐   │                │
│                  │  │ [🔍] Ask a question...      [➤] │   │                │
│                  │  │      Shift+Enter for newline    │   │                │
│                  │  └─────────────────────────────────┘   │                │
│                  │                                         │                │
└──────────────────┴─────────────────────────────────────────┴────────────────┘
```

---

## 4. Query History Sidebar Redesign

### 4.1 Conversation Item Structure

```tsx
interface ConversationItemDisplay {
  title: string; // Auto-generated or user-set
  preview: string; // First 50 chars of last message
  updatedAt: Date; // Human-friendly relative time
  mode: QueryMode; // Badge color
  messageCount: number; // "12 messages"
  isPinned: boolean; // Show pin icon
  isActive: boolean; // Highlight state
}
```

### 4.2 Filtering Options

| Filter | Type         | Options                                             |
| ------ | ------------ | --------------------------------------------------- |
| Search | Text         | Full-text search in titles and content              |
| Date   | Range        | Today, Yesterday, Last 7 days, Last 30 days, Custom |
| Mode   | Multi-select | Local, Global, Hybrid, Naive                        |
| Status | Toggle       | Active, Archived                                    |
| Pinned | Toggle       | Show pinned only                                    |

### 4.3 Sorting Options

| Sort          | Direction | Default |
| ------------- | --------- | ------- |
| Updated       | Desc      | ✓       |
| Created       | Desc      |         |
| Title         | Asc       |         |
| Message Count | Desc      |         |

### 4.4 Pagination Strategy

**Cursor-based pagination** for optimal performance:

```typescript
interface PaginatedConversations {
  items: Conversation[];
  nextCursor: string | null; // Opaque cursor for next page
  prevCursor: string | null;
  total: number;
  hasMore: boolean;
}
```

---

## 5. Interaction Design Patterns

### 5.1 Streaming Markdown Rendering

```
┌────────────────────────────────────────────────────────────┐
│  STATE MACHINE: Streaming Message Rendering                 │
├────────────────────────────────────────────────────────────┤
│                                                             │
│   ┌─────────┐    ┌──────────┐    ┌────────────┐            │
│   │  IDLE   │───▶│ THINKING │───▶│ GENERATING │            │
│   └─────────┘    └──────────┘    └────────────┘            │
│        ▲              │               │                    │
│        │              │               │                    │
│        │              ▼               ▼                    │
│        │         ┌─────────┐    ┌──────────┐               │
│        └─────────│  ERROR  │    │ COMPLETE │               │
│                  └─────────┘    └──────────┘               │
│                                                             │
├────────────────────────────────────────────────────────────┤
│  VISUALS BY STATE:                                          │
│                                                             │
│  THINKING:                                                  │
│  ┌──────────────────────────────────────┐                  │
│  │ 🧠 Processing your query...          │                  │
│  │ ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │ ← shimmer       │
│  │ ░░░░░░░░░░░░░░░░░░░░░░░░░░░░         │                  │
│  │ ░░░░░░░░░░░░░░░░░                    │                  │
│  └──────────────────────────────────────┘                  │
│                                                             │
│  GENERATING:                                                │
│  ┌──────────────────────────────────────┐                  │
│  │ Sarah Chen is connected to           │ ← fade-in       │
│  │ several entities in the knowledge█   │ ← cursor blink  │
│  └──────────────────────────────────────┘                  │
│                                                             │
│  COMPLETE:                                                  │
│  ┌──────────────────────────────────────┐                  │
│  │ Sarah Chen is connected to           │                  │
│  │ several entities in the knowledge    │                  │
│  │ graph:                               │                  │
│  │ • **TechCorp** (employer)           │ ← full markdown  │
│  │ • **John Smith** (colleague)        │                  │
│  └──────────────────────────────────────┘                  │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

### 5.2 Token-by-Token Rendering Algorithm

```typescript
// Pseudo-code for streaming markdown
class StreamingMarkdownRenderer {
  private buffer: string = "";
  private tokens: Token[] = [];

  onChunk(chunk: string) {
    this.buffer += chunk;

    // Try to parse complete tokens from buffer
    const { tokens, remainder } = this.parseCompleteTokens(this.buffer);

    this.tokens.push(...tokens);
    this.buffer = remainder;

    // Render all complete tokens + partial indicator
    this.render(this.tokens, this.buffer);
  }

  parseCompleteTokens(text: string): { tokens: Token[]; remainder: string } {
    const lexer = new marked.Lexer();
    const tokens: Token[] = [];

    // Split by potential token boundaries
    const lines = text.split("\n");
    let remainder = "";

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      const isLast = i === lines.length - 1;

      if (isLast && !this.isCompleteLine(line)) {
        remainder = line;
      } else {
        tokens.push(...lexer.lex(line + "\n"));
      }
    }

    return { tokens, remainder };
  }
}
```

### 5.3 Pagination UX

**Infinite scroll with intersection observer**:

```
┌──────────────────────────────┐
│ 📝 Conversation 1            │
│ 📝 Conversation 2            │
│ 📝 Conversation 3            │
│ 📝 Conversation 4            │
│ 📝 Conversation 5            │
├──────────────────────────────┤
│       [Loading more...]      │ ← Intersection triggers fetch
│       ●●● (spinner)         │
├──────────────────────────────┤
│ 📝 Conversation 6            │ ← New items appended
│ 📝 Conversation 7            │
│ ...                          │
└──────────────────────────────┘
```

### 5.4 Filter Interaction

**Immediate vs. Applied Filters**:

| Filter Type | Behavior          | Rationale                 |
| ----------- | ----------------- | ------------------------- |
| Text search | Debounced (300ms) | Avoid excessive API calls |
| Date range  | On select         | Discrete selection        |
| Mode        | Immediate toggle  | Quick filtering           |
| Clear all   | Immediate         | Reset to defaults         |

```
┌─────────────────────────────────────────┐
│ 🔍 [Search conversations...        ]    │
├─────────────────────────────────────────┤
│ Date: [This week ▾]  Mode: [All ▾]      │
│                                         │
│ Showing 12 of 47 conversations          │
│ [Clear filters]                         │
└─────────────────────────────────────────┘
```

---

## 6. Skeleton States & Loading Patterns

### 6.1 Conversation List Loading

```
┌──────────────────────────────┐
│ + New Chat                   │
├──────────────────────────────┤
│ 🔍 Search...                 │
├──────────────────────────────┤
│ ░░░░░░░░░░░░░░░░░░░░░       │ ← Skeleton item
│ ░░░░░░░░░░░░   ░░░░░        │
├──────────────────────────────┤
│ ░░░░░░░░░░░░░░░░░           │
│ ░░░░░░░░░░   ░░░░░░         │
├──────────────────────────────┤
│ ░░░░░░░░░░░░░░░░░░░░        │
│ ░░░░░░░░░░░░   ░░░░         │
└──────────────────────────────┘
```

### 6.2 Message Loading

```
┌──────────────────────────────────────────┐
│  You                                  3s │
│  ┌────────────────────────────────────┐  │
│  │ What are the key entities?         │  │
│  └────────────────────────────────────┘  │
├──────────────────────────────────────────┤
│  EdgeQuake                            now │
│  ┌────────────────────────────────────┐  │
│  │ 🧠 Processing...                   │  │
│  │ ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │  │
│  │ ░░░░░░░░░░░░░░░░░░░░░░░░░░░░       │  │
│  │ ░░░░░░░░░░░░░░░░░░░                │  │
│  └────────────────────────────────────┘  │
└──────────────────────────────────────────┘
```

---

## 7. Responsive Breakpoints

### 7.1 Breakpoint Definitions

| Breakpoint | Width       | Layout                                 |
| ---------- | ----------- | -------------------------------------- |
| Mobile     | <640px      | History as bottom sheet, single column |
| Tablet     | 640-1024px  | Collapsible history rail               |
| Desktop    | 1024-1440px | History rail visible                   |
| Wide       | >1440px     | History + Context rails visible        |

### 7.2 Mobile Layout

```
┌──────────────────────────┐
│ Query                [≡] │ ← Hamburger for history
├──────────────────────────┤
│                          │
│  Message Thread          │
│  (full width)            │
│                          │
├──────────────────────────┤
│ [Ask a question...]  [➤] │ ← Fixed input
└──────────────────────────┘

[≡] opens bottom sheet:
┌──────────────────────────┐
│ ┌────────────────────┐   │
│ │▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓│   │ ← Drag handle
│ ├────────────────────┤   │
│ │ Conversation 1     │   │
│ │ Conversation 2     │   │
│ │ Conversation 3     │   │
│ └────────────────────┘   │
└──────────────────────────┘
```

---

## 8. Accessibility Requirements

### 8.1 WCAG 2.1 AA Compliance

| Requirement         | Implementation                                  |
| ------------------- | ----------------------------------------------- |
| Keyboard navigation | Tab order, Enter to submit, Esc to cancel       |
| Screen reader       | aria-labels, live regions for streaming         |
| Color contrast      | 4.5:1 minimum for text                          |
| Focus indicators    | Visible focus rings on all interactive elements |
| Reduced motion      | Respect `prefers-reduced-motion`                |

### 8.2 ARIA Landmarks

```html
<main role="main" aria-label="Query interface">
  <aside role="complementary" aria-label="Conversation history">
    <nav aria-label="Conversation list">...</nav>
  </aside>
  <section aria-label="Active conversation">
    <header aria-label="Conversation header">...</header>
    <div role="log" aria-label="Messages" aria-live="polite">...</div>
    <form aria-label="Query input">...</form>
  </section>
</main>
```

---

## 9. Next Steps

1. **Phase 3**: Create technical specifications → [03_technical_spec.md](03_technical_spec.md)
2. **Phase 4**: Build implementation roadmap → [04_implementation_roadmap.md](04_implementation_roadmap.md)
3. **Phase 5**: Design mockups → [05_design_mockups.md](05_design_mockups.md)

---

_Last updated: 2024-12-27_
