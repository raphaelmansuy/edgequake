# Phase 5: Design Mockups - Query Page UX/UI Improvement

> **Date**: December 27, 2025  
> **Purpose**: Visual reference for implementation  
> **Fidelity**: Low-to-medium (ASCII mockups with detailed specs)

---

## 1. Query Page - Full Layout

### 1.1 Desktop Layout (≥1024px)

```
┌────────────────────────────────────────────────────────────────────────────────────┐
│ ┌──────────────────────────────────────────────────────────────────────────────────┐ │
│ │                           EdgeQuake Query Interface                              │ │
│ └──────────────────────────────────────────────────────────────────────────────────┘ │
├──────────────────────┬──────────────────────────────────────────────────────────────┤
│                      │                                                              │
│  ┌────────────────┐  │  ┌────────────────────────────────────────────────────────┐  │
│  │ 🔍 Search...   │  │  │                                                        │  │
│  └────────────────┘  │  │                                                        │  │
│                      │  │                                                        │  │
│  ┌────────────────┐  │  │                    [Empty State]                       │  │
│  │ + New Query    │  │  │                                                        │  │
│  └────────────────┘  │  │                    ┌──────────┐                        │  │
│                      │  │                    │    🔮    │                        │  │
│  ─────────────────   │  │                    │ ╭──────╮ │                        │  │
│                      │  │                    │ │ 🎯→🎯│ │                        │  │
│  📁 TODAY            │  │                    │ ╰──────╯ │                        │  │
│                      │  │                    └──────────┘                        │  │
│  ┌────────────────┐  │  │                                                        │  │
│  │ 💬 Entity Graph│  │  │           Explore Your Knowledge Graph                 │  │
│  │    Exploration │  │  │                                                        │  │
│  │    12:34 PM    │  │  │         0 entities  •  0 relationships  •  0 docs      │  │
│  └────────────────┘  │  │                                                        │  │
│                      │  │                                                        │  │
│  📁 YESTERDAY        │  │        ┌──────────────────────────────────────────┐     │  │
│                      │  │        │ Try: "Explain entity extraction process"│     │  │
│  ┌────────────────┐  │  │        └──────────────────────────────────────────┘     │  │
│  │ 💬 Database    │  │  │                                                        │  │
│  │    Schema Q&A  │  │  │        ┌──────────────────────────────────────────┐     │  │
│  │    3:22 PM     │  │  │        │ Try: "Compare local vs global queries"  │     │  │
│  └────────────────┘  │  │        └──────────────────────────────────────────┘     │  │
│                      │  │                                                        │  │
│  📁 LAST 7 DAYS      │  │                                                        │  │
│                      │  └────────────────────────────────────────────────────────┘  │
│  ┌────────────────┐  │                                                              │
│  │ 💬 RAG vs Naive│  │  ┌────────────────────────────────────────────────────────┐  │
│  │    Retrieval   │  │  │ 🎤 │  Ask a question about your knowledge graph...   │  │  │
│  │    Yesterday   │  │  │    └───────────────────────────────────────────────────┤  │
│  └────────────────┘  │  │    ├────┬────┬────┐    ┌────────────────┬────────────┐ │  │
│                      │  │    │local│hybrid│global│    │ 📎 Attach    │ ➤ Send   │ │  │
│         ...          │  │    └────┴────┴────┘    └────────────────┴────────────┘ │  │
│                      │  └────────────────────────────────────────────────────────┘  │
│  ─────────────────   │                                                              │
│                      │                                                              │
│  ⚙️ Settings         │                                                              │
│                      │                                                              │
└──────────────────────┴──────────────────────────────────────────────────────────────┘
            │                                        │
        280px fixed                            Fluid main content
```

### 1.2 Tablet Layout (768px - 1023px)

```
┌──────────────────────────────────────────────────────────────────────┐
│                      EdgeQuake Query Interface                        │
├──────────────────────────────────────────────────────────────────────┤
│ ┌──┐                                                                  │
│ │☰ │  Conversation list collapses to hamburger menu                  │
│ └──┘                                                                  │
│                                                                       │
│  ┌────────────────────────────────────────────────────────────────┐  │
│  │                                                                │  │
│  │                     [Chat Content Area]                        │  │
│  │                                                                │  │
│  │     Full width on tablet, more reading space                   │  │
│  │                                                                │  │
│  └────────────────────────────────────────────────────────────────┘  │
│                                                                       │
│  ┌────────────────────────────────────────────────────────────────┐  │
│  │ 🎤 │  Ask a question...                                       │  │
│  │    └───────────────────────────────────────────────────────────┤  │
│  │    ├──────────────────────────────────────────┬──────────────┐ │  │
│  │    │ local │ hybrid │ global        📎        │   ➤ Send    │ │  │
│  │    └──────────────────────────────────────────┴──────────────┘ │  │
│  └────────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────┘
```

### 1.3 Mobile Layout (<768px)

```
┌────────────────────────────────────┐
│ ☰   EdgeQuake Query    ⋮          │
├────────────────────────────────────┤
│                                    │
│  ┌──────────────────────────────┐  │
│  │                              │  │
│  │     [Chat Content Area]      │  │
│  │                              │  │
│  │  Messages stack vertically   │  │
│  │  with max 85% width          │  │
│  │                              │  │
│  │  Code blocks scroll horiz.   │  │
│  │                              │  │
│  └──────────────────────────────┘  │
│                                    │
│  ┌──────────────────────────────┐  │
│  │ Ask a question...            │  │
│  ├──────────────────────────────┤  │
│  │ local  hybrid  global        │  │
│  ├────────────────┬─────────────┤  │
│  │      📎        │    ➤       │  │
│  └────────────────┴─────────────┘  │
└────────────────────────────────────┘

   ↓ Swipe left for history panel

┌────────────────────────────────────┐
│ ← History                   ✕     │
├────────────────────────────────────┤
│                                    │
│  🔍 Search conversations...        │
│                                    │
│  ─────────────────────────────     │
│                                    │
│  TODAY                             │
│  ┌──────────────────────────────┐  │
│  │ 💬 Entity Graph Exploration  │  │
│  │    12:34 PM                  │  │
│  └──────────────────────────────┘  │
│                                    │
│  YESTERDAY                         │
│  ┌──────────────────────────────┐  │
│  │ 💬 Database Schema Q&A       │  │
│  │    3:22 PM                   │  │
│  └──────────────────────────────┘  │
│                                    │
│       ... more conversations       │
│                                    │
└────────────────────────────────────┘
```

---

## 2. Chat Message Component

### 2.1 User Message

```
                                          ┌────────────────────────────────┐
                                          │ What is entity extraction and  │
                                          │ how does EdgeQuake implement   │
                                          │ it?                            │
                                          ├────────────────────────────────┤
                                          │ 12:34 PM                       │
                                          └────────────────────────────────┘
                                                           │
                                                   Max 75% width
                                                   Right-aligned
                                                   Primary color bg
```

**Specifications:**

- Background: `oklch(0.97 0.02 var(--hue-primary))`
- Border radius: `12px 12px 4px 12px`
- Padding: `12px 16px`
- Max width: `75%`
- Font: System UI, 14px, line-height 1.5
- Timestamp: `text-xs text-muted-foreground`

### 2.2 Assistant Message - Thinking State

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  🧠  Thinking...                                                    [▼ Hide] │
├──────────────────────────────────────────────────────────────────────────────┤
│  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░    │
│  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░                                 │
│  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░                │
└──────────────────────────────────────────────────────────────────────────────┘
           │                                                        │
      Shimmer animation (left → right)                    Collapsible toggle
      3 varying width bars                                Remembers state
```

**Specifications:**

- Background: `oklch(0.96 0.01 var(--hue-primary))` (subtle tint)
- Shimmer: CSS gradient animation, 1.5s duration, infinite
- Brain icon: Pulse animation, 1s duration
- Collapse transition: 200ms ease-out
- Border: `1px dashed oklch(0.80 0.05 var(--hue-primary))`

### 2.3 Assistant Message - Content

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                                                                              │
│  Entity extraction is the process of identifying and extracting named       │
│  entities from unstructured text.                                           │
│                                                                              │
│  ## How EdgeQuake Implements It                                              │
│                                                                              │
│  EdgeQuake uses a multi-step pipeline:                                       │
│                                                                              │
│  1. **Chunking** - Documents are split into semantic chunks                  │
│  2. **LLM Extraction** - GPT-4 identifies entities and relationships         │
│  3. **Normalization** - Entity names are standardized (UPPERCASE)            │
│  4. **Deduplication** - Similar entities are merged                          │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │ typescript                                                   [Copy] 📋│  │
│  ├────────────────────────────────────────────────────────────────────────┤  │
│  │ interface Entity {                                                     │  │
│  │   name: string;        // Normalized: "SARAH_CHEN"                     │  │
│  │   type: string;        // "PERSON", "ORGANIZATION", etc.               │  │
│  │   description: string; // LLM-generated summary                        │  │
│  │ }                                                                      │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  > [!NOTE]                                                                   │
│  > EdgeQuake extracts 2-3x more entities than naive approaches[^1]          │
│                                                                              │
│  ───────────────────────────────────────────────────────────────────────     │
│  Sources                                                                     │
│  ───────────────────────────────────────────────────────────────────────     │
│                                                                              │
│  📄 [docs/architecture.md]  📄 [docs/algorithms.md]  📄 [paper.pdf]         │
│                                                                              │
├──────────────────────────────────────────────────────────────────────────────┤
│  12:35 PM  •  hybrid mode  •  3 sources                    [📋] [🔄] [👍]   │
└──────────────────────────────────────────────────────────────────────────────┘
         │                                                          │
     Max 80% width                                         Action buttons:
     Left-aligned                                          Copy, Regenerate, Like
```

**Specifications:**

- Background: `var(--card)` (adapts to theme)
- Border radius: `12px 12px 12px 4px`
- Padding: `16px 20px`
- Max width: `80%`
- Headings: bold, scaled sizing (h2: 18px, h3: 16px)
- Code blocks: `var(--code-bg)`, syntax highlighting
- Sources section: Horizontal scroll on mobile

---

## 3. Markdown Extensions

### 3.1 GitHub-Style Alerts

```
┌─ NOTE ────────────────────────────────────────────────────────────────┐
│ ℹ️  This is informational content that helps understanding.           │
└───────────────────────────────────────────────────────────────────────┘

┌─ TIP ─────────────────────────────────────────────────────────────────┐
│ 💡  Pro tip: Use hybrid mode for best results with large graphs.      │
└───────────────────────────────────────────────────────────────────────┘

┌─ WARNING ─────────────────────────────────────────────────────────────┐
│ ⚠️  Careful: This operation cannot be undone.                          │
└───────────────────────────────────────────────────────────────────────┘

┌─ CAUTION ─────────────────────────────────────────────────────────────┐
│ 🛑  Stop! This may cause data loss if used incorrectly.                │
└───────────────────────────────────────────────────────────────────────┘

┌─ IMPORTANT ───────────────────────────────────────────────────────────┐
│ ⭐  Required: Set OPENAI_API_KEY before running production.            │
└───────────────────────────────────────────────────────────────────────┘
```

**Color Specifications:**

| Type      | Background             | Border                 | Icon      |
| --------- | ---------------------- | ---------------------- | --------- |
| NOTE      | `oklch(0.96 0.02 220)` | `oklch(0.60 0.15 220)` | ℹ️ Blue   |
| TIP       | `oklch(0.96 0.02 140)` | `oklch(0.60 0.15 140)` | 💡 Green  |
| WARNING   | `oklch(0.96 0.04 80)`  | `oklch(0.70 0.15 80)`  | ⚠️ Yellow |
| CAUTION   | `oklch(0.96 0.04 25)`  | `oklch(0.65 0.20 25)`  | 🛑 Red    |
| IMPORTANT | `oklch(0.96 0.03 280)` | `oklch(0.60 0.15 280)` | ⭐ Purple |

### 3.2 Footnotes

```
Regular text with a reference[^1] that links to the bottom.

Another paragraph with multiple footnotes[^2][^note].

──────────────────────────────────────────────────────────────────────────

[^1]: This is the first footnote definition. It appears at the bottom.

[^2]: Second footnote with more details.

[^note]: Named footnotes work too!
```

**Interaction:**

- Superscript number: Clickable, scrolls to definition
- Definition: Has back arrow to return to reference
- Hover preview (optional): Shows footnote content in tooltip

### 3.3 Collapsible Details

````
┌─ <details> ───────────────────────────────────────────────────────────┐
│ ▶ Click to expand implementation details                              │
└───────────────────────────────────────────────────────────────────────┘

         ↓ After clicking

┌─ <details open> ──────────────────────────────────────────────────────┐
│ ▼ Click to expand implementation details                              │
├───────────────────────────────────────────────────────────────────────┤
│                                                                       │
│  Here is the expanded content with full details:                      │
│                                                                       │
│  - First point about implementation                                   │
│  - Second point with more context                                     │
│  - Third point with code example below                                │
│                                                                       │
│  ```rust                                                              │
│  fn example() -> Result<()> {                                         │
│      // Implementation here                                           │
│  }                                                                    │
│  ```                                                                  │
│                                                                       │
└───────────────────────────────────────────────────────────────────────┘
````

**Specifications:**

- Summary: `font-weight: 500`, `cursor: pointer`
- Arrow: Rotates 90° on open
- Transition: `max-height 200ms ease-out`
- Content padding: `12px 16px`

---

## 4. Tables - Streaming Behavior

### 4.1 During Streaming (Incomplete)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│  Table data loading...                                                      │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░│   │
│  │ ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░│   │
│  │ ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░│   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
                    │
              Skeleton with shimmer animation
              3 rows placeholder
              Shows while table structure incomplete
```

### 4.2 After Streaming Complete

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│  ┌───────────────────┬────────────────┬───────────────┬─────────────────┐  │
│  │ Query Mode        │ Latency (P50)  │ Quality Score │ Use Case        │  │
│  ├───────────────────┼────────────────┼───────────────┼─────────────────┤  │
│  │ local             │ 45ms           │ 0.72          │ Fast lookup     │  │
│  ├───────────────────┼────────────────┼───────────────┼─────────────────┤  │
│  │ hybrid            │ 120ms          │ 0.89          │ General use     │  │
│  ├───────────────────┼────────────────┼───────────────┼─────────────────┤  │
│  │ global            │ 280ms          │ 0.95          │ Complex queries │  │
│  └───────────────────┴────────────────┴───────────────┴─────────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
                    │
              Table renders with fade-in animation (200ms)
              Horizontal scroll on mobile
              Striped rows optional
```

---

## 5. Input Area Component

### 5.1 Default State

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │ 🎤 │  Ask a question about your knowledge graph...                    │  │
│  │    │                                                                   │  │
│  │    │                                                                   │  │
│  │    └───────────────────────────────────────────────────────────────────┤  │
│  │    ├────────────────────────────────────────────┬──────────────────────┤  │
│  │    │ local  │  hybrid ●  │  global              │  📎    ➤            │  │
│  │    └────────────────────────────────────────────┴──────────────────────┘  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│                           hybrid: Balanced query                             │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
      │                           │                              │
  Voice input                Mode selector                  Action buttons
  (future)                   with tooltip                   Attach, Send
```

**Specifications:**

- Min height: `48px`
- Max height: `200px` (auto-resize)
- Border: `1px solid var(--border)`
- Focus border: `2px solid var(--primary)`
- Placeholder: `text-muted-foreground`
- Mode chips: Toggle group, `px-3 py-1.5`, border-radius `9999px`

### 5.2 Typing State

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │ 🎤 │  How does the entity extraction algorithm handle                 │  │
│  │    │  co-reference resolution for pronouns in multi-                  │  │
│  │    │  paragraph documents?█                                           │  │
│  │    └───────────────────────────────────────────────────────────────────┤  │
│  │    ├────────────────────────────────────────────┬──────────────────────┤  │
│  │    │ local  │  hybrid ●  │  global              │  📎    ➤●           │  │
│  │    └────────────────────────────────────────────┴──────────────────────┘  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  ⌘ + Enter to send   •   Shift + Enter for new line                         │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
                                                               │
                                                       Send button active
                                                       (primary color)
```

### 5.3 Streaming State

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │ 🎤 │  [disabled during response]                                      │  │
│  │    │                                                                   │  │
│  │    └───────────────────────────────────────────────────────────────────┤  │
│  │    ├────────────────────────────────────────────┬──────────────────────┤  │
│  │    │ [modes disabled]                           │  📎    ⏹            │  │
│  │    └────────────────────────────────────────────┴──────────────────────┘  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  ⏹ Stop generating   •   Response streaming...                              │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
                                                               │
                                                       Stop button (red)
```

---

## 6. Conversation History Panel

### 6.1 With Filters Active

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │  🔍 Search conversations...                                            │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │  Filters  ▼                                                            │  │
│  ├────────────────────────────────────────────────────────────────────────┤  │
│  │                                                                        │  │
│  │  Mode:   ○ All   ● hybrid   ○ local   ○ global                        │  │
│  │                                                                        │  │
│  │  Date:   ┌──────────────┐  →  ┌──────────────┐                        │  │
│  │          │ Dec 20, 2025 │     │ Dec 27, 2025 │                        │  │
│  │          └──────────────┘     └──────────────┘                        │  │
│  │                                                                        │  │
│  │  Sort:   ▼ Newest first                                               │  │
│  │                                                                        │  │
│  │  ┌──────────────────────────────────────────────────────────────────┐ │  │
│  │  │  ✕ Clear filters                                  12 results    │ │  │
│  │  └──────────────────────────────────────────────────────────────────┘ │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  ─────────────────────────────────────────────────────────────────────────   │
│                                                                              │
│  TODAY                                                                       │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │  💬 Entity Graph Exploration                              ⋮          │  │
│  │     hybrid • 3 messages                           12:34 PM           │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │  💬 Performance Benchmarks                                 ⋮          │  │
│  │     hybrid • 8 messages                           10:15 AM           │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

**Specifications:**

- Search: Debounced 300ms, searches title + content
- Mode filter: Radio group, immediate filter
- Date filter: Date range picker component
- Sort: Dropdown (Newest, Oldest, Alphabetical)
- Clear button: Resets all filters
- Results count: Updates immediately

---

## 7. Loading & Error States

### 7.1 Conversation Loading Skeleton

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │ ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░│  │
│  │ ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░                                     │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│                                          ┌────────────────────────────────┐  │
│                                          │ ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░│  │
│                                          │ ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░│  │
│                                          │ ░░░░░░░░░░░░░░░░░░░░         │  │
│                                          └────────────────────────────────┘  │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │ ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░│  │
│  │ ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░                      │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 7.2 Error State

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                                                                              │
│                          ┌──────────────────────────┐                        │
│                          │                          │                        │
│                          │          ⚠️              │                        │
│                          │                          │                        │
│                          └──────────────────────────┘                        │
│                                                                              │
│                    Something went wrong                                      │
│                                                                              │
│          We couldn't process your request. Please try again.                 │
│                                                                              │
│                    ┌────────────────────────┐                                │
│                    │     🔄 Retry           │                                │
│                    └────────────────────────┘                                │
│                                                                              │
│          Error: Connection timeout after 30s                                 │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 7.3 Empty Search Results

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                                                                              │
│                          ┌──────────────────────────┐                        │
│                          │                          │                        │
│                          │          🔍              │                        │
│                          │                          │                        │
│                          └──────────────────────────┘                        │
│                                                                              │
│                    No conversations found                                    │
│                                                                              │
│          No results match "entity graph" with current filters.               │
│                                                                              │
│                    ┌────────────────────────┐                                │
│                    │   ✕ Clear filters      │                                │
│                    └────────────────────────┘                                │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## 8. Component Specifications Summary

| Component           | Width   | Padding   | Border Radius      | Shadows |
| ------------------- | ------- | --------- | ------------------ | ------- |
| Chat Message (User) | max 75% | 12px 16px | 12px 12px 4px 12px | none    |
| Chat Message (AI)   | max 80% | 16px 20px | 12px 12px 12px 4px | sm      |
| Code Block          | 100%    | 12px 16px | 8px                | none    |
| Alert Box           | 100%    | 12px 16px | 6px                | none    |
| Input Area          | 100%    | 16px      | 12px               | md      |
| Conversation Item   | 100%    | 12px 16px | 8px                | none    |
| Filter Panel        | 100%    | 16px      | 8px                | none    |

---

## References

- [Design Strategy](./02_design_strategy.md) - Design principles and color system
- [Technical Specification](./03_technical_spec.md) - Implementation details
- [Implementation Roadmap](./04_implementation_roadmap.md) - Sprint plan

---

_Document Version: 1.0 | Last Updated: December 27, 2025_
